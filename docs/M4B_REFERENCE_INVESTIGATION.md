# M4b: the rest of Episode 1 (teleporters, the Baron, A_BossDeath, hard skill, DEMO1): reference investigation

Phase 1 note for milestone M4b, the second of the Episode 1 milestones that follow the whole-engine harness
([M3h9c note](M3H9C_REFERENCE_INVESTIGATION.md), [M4a note](M4A_REFERENCE_INVESTIGATION.md)). M4a took the
harness to E1M1–E1M4, E1M6 and E1M7 on medium skill. M4b adds what E1M5, E1M8 and E1M9 still need, hard skill, and the
`DEMO1` of the local four-episode `DOOM.WAD` (which the genuine engine runs as registered because of its file name, §1.1). This note holds a counts-only census of those maps on skills 2 and 3, the original C facts
for every new mechanism (function, file and line), the Rust wire 23 inventory that wire 24 must extend, the
owner decisions I1–I13 with recommendations, and the plan with a size estimate and cap.

Source authority is `linuxdoom-1.10/` at the repository's upstream commit; line numbers are of those files, and
`crates/` and `tools/` line numbers are of master at the start of this milestone (`b28d4cf`, the status commit after the
merge of PR #57). Milestone status lives in [ROADMAP](ROADMAP.md), support in [COMPATIBILITY](COMPATIBILITY.md), schemas in
[STATE_FORMAT](STATE_FORMAT.md), commands in [TESTING](TESTING.md) and the asset rules in [ASSETS](ASSETS.md) (see the
[documentation authority map](README.md#documentation-authority)). The observations of this milestone will go to
`docs/validation/M4B.md`, created in Phase 2. This note owns the source facts, the design, the decisions and the plan.
It never contains an IWAD-derived coordinate, angle, lump byte or per-object datum: only counts, special numbers,
doomednums with their `info.c` names, header values, tic counts and digests.

**Status / next step (2026-11-14 PST).** Phase 1 done; the owner confirmed decisions I1–I13 on 2026-11-10 (§5, §7). Phase 2 steps 1–14 done (step 14, the independent review at aafe328: finding F1 fixed, finding 30; see the [review record](validation/M4B.md#independent-review-step-14)). Steps 1–5: synthetic WADs and genuine runs (b42326d), the DSIM40 projection and goldens (e526cee, b59cda3), wire 24 plumbing, teleportation, the Baron and `A_BossDeath`. Step 6 (hard skill): 92b6369, f9128c8, d199d01. **Step 7 (decisions I11 and I5) done, all four sub-items:** (a) the M5WALK3 refusal explained and fixed (2dd986c: wire 24 admits a Demon or Spectre bite on a body carrying `A_Saw`'s `MF_JUSTATTACKED`, p_pspr.c:542), reproduced by the second-set scene SAWBITE (663d92b, 3444334) and covered by `saw_bite_tests.rs` (56d3551, 87fded2); (b) the finished M9WALK3 run; (c) `check_player_episode1v2.py --require` re-run at 87fded2: 50 of 50 scripts, 19,933 tics, 0 pending; (d) the results file merged (cc37283): 61 real-map runs (the 30 older scripts, 30 new E1M5/E1M8/E1M9 scripts on skills 2 and 3, and `DEMO1`), all matched tic for tic against the genuine engine under wire 24 over DSIM40, `ASAN_OPTIONS=detect_leaks=0` (no leak detection); 26b24e6 is a Clippy 1.100 `needless_range_loop` fix with no behaviour change. The evidence record is [validation/M4B.md](validation/M4B.md#real-maps-and-demo1); counts are there only. **Open divergences: none.** Unobserved: the lethal flagged bite (the body dies while `MF_JUSTATTACKED` is set) is reasoned from the source, not seen in any run. CI on cc37283 passed on all five jobs; CI on 95f5d59 failed Clippy (1.100 `field_reassign_with_default` in a step 9 test), fixed by e8bb8f6; GitHub is the record for later heads. **Step 8 (carry-ins) done:** 61c8379 (the `barrels.rs:362:19` mutant proved equivalent, comment only; finding 27) and 750b307 (the boss-floor height-bound test; finding 28); owner rulings 2026-11-13: no mutant exclusion unless it appears in the workflow, no change to the map-8 height rule. **Step 9 done:** d961c86 (wire-24 rejection, rollback and isolation tests), 0448962 and a1df9e2 (the wire-24 soak, 64 runs over the sixteen synthetic WADs), f640ade (fuzz target `player_episode1_v2` and its seeder), e8bb8f6, 8223b49. **Step 10 (documentation) done; the fireball tunnel is ruled (finding 29).** The soak found that a Baron fireball (speed 15, radius 6; info.c:1524–1548) can step wholly past a one-sided wall in one unsplit step, because `P_CheckPosition` tests only the destination box (p_map.c:191–195, 374–442); the step bound 2r − 2 = 10 of M3h5 Theorem B does not hold for it (15 > 10; see the dated correction after §3 part A fact 9). The genuine engine keeps the fireball flying in the void. Owner ruling 2026-11-13: keep the behaviour (the next tic is refused with `Monster(Arithmetic)`, state preserved) and document it as a declared wire-24 refuse-the-tic witness; it is not a difference from the original, because Rust either matches the genuine engine or refuses. Recorded in [COMPATIBILITY](COMPATIBILITY.md#the-baron-fireball-wall-tunnel-wire-24), the [evidence record](validation/M4B.md#the-baron-fireball-wall-tunnel) and finding 29. **Step 11 done (mutation testing):** the I12 workflow (ce00f06) ran green on 8 shards; every one of its 63 missed mutants is addressed (55 killed by new tests, 8 proved equivalent in comments); counts in the [evidence record](validation/M4B.md#mutation-testing-step-11). **Step 12 done (fuzz) and step 13 done (matched timings):** [fuzz](validation/M4B.md#fuzzing-step-12) and [timings](validation/M4B.md#matched-performance); the timing run found a short-workload slowdown with equal instruction counts; a 2026-11-14 re-time with the binaries on ext4 showed no slowdown there (it reproduces only from the 9p `/mnt/d` mount), so it is not a code regression; carried to the proposed performance milestone. **Step 15 local gates done on 58b7f94** (all from clean; counts in the [evidence record](validation/M4B.md#final-gates-from-clean-step-15); exact-head CI is GitHub's record). **Next:** step 16 (the PR), per §6. **Open owner questions: none** (the fireball tunnel is resolved, ruling 2026-11-13). Scratch root: /mnt/d/scratch/m4b (I13); the step 7 authoring files, logs and result JSONs are under `s7/`; keep corpora and dumps out of the tree and delete dumps once digests are extracted. **Disk rule (owner, 2026-11-10):** `df -h /` shows the WSL virtual disk's ceiling, not real space, and is not a stop condition. `/mnt/c` is the only real meter: run `df -h /mnt/c` after every build and every real-map run, and stop, push and report if it shows under 20 GB free. Keep one cargo target directory per tree (the repo's `target/` and `fuzz/target`); a second build tree puts its target directory under /mnt/d/scratch/m4b. Latest: /mnt/c 72 GB free (step 15 start, 2026-11-14). **CPU (owner, 2026-11-13, again 2026-11-14):** the machine is free and the owner may use it for play; there are no CPU caps or job limits, including C reference runs (this replaces the 2026-11-11 `CARGO_BUILD_JOBS=6`, `NEXTEST_TEST_THREADS=4` and two-job limits). If a process is OOM-killed (WSL has 7.7 GB of memory), halve its parallelism and re-run it. A timing run needs its own idle check (see the [matched performance record](validation/M4B.md#matched-performance)). **Pushes:** code commits are pushed as soon as verified, tests and docs follow as their own commits, and CI on each pushed head SHA is checked and reported (owner, 2026-11-10); push only when the previous head's CI run has finished (`gh run list -R Copgunn/DOOM --branch feat/m4b-episode-1-complete --limit 2`; the workflow cancels a running head). Resume: `git checkout feat/m4b-episode-1-complete && git pull`; read this line, AGENTS.md, §3 part C, §4.4, §5–§7, validation/M4B.md and the STATE_FORMAT DSIM40 section. Reproduce the synthetic evidence with the commands in [TESTING](TESTING.md#m4b-synthetic-mechanism-wads).

## Summary

- Wire 23 refuses E1M5, E1M8 and E1M9 at load with `Runtime(Pursuit(Specials))`. The cause is line special 97 (teleport,
  walk-over, repeatable): 10 lines on E1M5, 8 on E1M8, 2 on E1M9. The other six maps load and run (§1.2, §1.3).
- Episode 1 has exactly one unported line special (97) and two unported things: the teleport landing (doomednum 14, one
  on each of the three maps) and the Baron of Hell (doomednum 3003, two on E1M8 on both skills) (§1.6, §1.10).
- A player telefrag calls `P_DamageMobj` with damage 10,000. Its thrust expression overflows signed `int`. The genuine
  `strict` and `strict32` builds both wrap to −397,934,592 before the division; the `overflow` and `address` builds abort.
  Decision I3 recommends a declared adapter in those two builds plus a Rust wrapping rule under wire 24 only (§2 T8, §5).
- Skill 3 (hard) differs from skill 2 in single player only in the THING filter (option bit 4 instead of bit 2). Hard adds
  monsters (41 handled things on E1M5, 14 on E1M8, 31 on E1M9) and no doomednum outside wire 23's set (§3 part C, §1.6).
- `DEMO1` is E1M5, skill 3, one player, 6,854 bytes, 1,710 tic commands, ending with the 0x80 marker, and its version byte
  is 109. linuxdoom-1.10 plays only 110, so the genuine run needs a scratch copy with byte 0 patched; the Rust reader also
  refuses skill 3 today (§1.9, §3 part D).
- E1M8 ends through `A_BossDeath` (tag 666 floor, E1M8 with a Baron only) and the already-ported sector special 11.
  Rust has no map number today, so wire 24 must carry it as canonical state (§3 part B, §4.2.7).
- Size estimate: about 7k–12k lines of code, tools and docs against the 30k cap (generated data reported separately);
  fallback is to move hard skill and DEMO1 to a follow-up (§6).

## 1. Census (counts only)

Counts only. Produced Tue Nov 10 09:09:12 PST 2026 (America/Los_Angeles) on branch feat/m4b-episode-1-complete at b28d4cf. No lump content, name, coordinate or geometry is recorded.

### 1.1 Source, method and commands

The IWAD directory holds DOOM.WAD and DOOM2.WAD; this census uses DOOM.WAD only.

| fact | value |
| --- | --- |
| file | /mnt/d/DoomIWADs/DOOM.WAD (read in place, never copied) |
| size | 12,408,292 bytes |
| SHA-256 | 6fdf361847b46228cfebd9f3af09cd844282ac75f3edbb61ca4cb27103ce2e7f |
| WAD header | IWAD, 2306 lumps (observed); it has episode 4 maps and DEMO4, so it is the four-episode (Ultimate DOOM) WAD, not the three-episode registered one. The genuine engine nevertheless runs it as `registered`: `IdentifyVersion` picks the game mode from the file name alone, trying `doomu.wad` (retail, linuxdoom-1.10/d_main.c:587-588, 690-695) before `doom.wad` (registered, 591-592, 697-702), so a file named `DOOM.WAD` gets `gamemode = registered`, as the C driver sets it (tools/reference/m4a_driver.c:573). Version identification beyond size and contents was not attempted |
| other file in the directory | DOOM2.WAD, 14,604,584 bytes, SHA-256 10d67824b11025ddd9198e8cfc87ca335ee6e2d3e63af4180fa9b8a471893255 (not used) |

Skill naming: this document uses the engine's 0-based gameskill. Skill 2 is "hurt me plenty" (THING option bit 2) and skill 3 is "ultra-violence" (bit 4); that is the brief's "skill 2 (medium) and skill 3 (hard)". A THING is handled when it is a start (types 1-4 and 11, every skill) or when bit 16 (multiplayer only) is clear and the skill bit is set (p_mobj.c:715-752, read from linuxdoom-1.10/p_mobj.c). The skill 2 numbers match the M4a census (docs/M4A_REFERENCE_INVESTIGATION.md §2) for every map.

Tools reused: the M4a census tool tools/iwad_counts.py (MapCounts, read_wad, map_lumps, mobj_table, reach_text, MapCounts.window_specials for the 2x2 spechit window) imported read-only; no repository file was changed. A scratch driver (not committed) only formats the numbers for skill 3 and the wire-23 flags. The wire-23 admitted sets were taken from the code and docs: line specials = wire 22's {0, 1, 11, 36, 38, 48, 63, 88, 109} plus the 29 of docs/COMPATIBILITY.md "Admitted line specials" (matches the arms in crates/doom-sim/src/mover_rules.rs); sector specials = wire 22's plus 11; things = crates/doom-sim/src/episode1.rs (monsters 3004, 9, 65, 3001, 3002, 58; 25 pickups; barrel 2035; statics = E1M1's seven plus 34 and 46) with player start 1 spawned and 2-4 and 11 stored; 14 is documented as not admitted.

Commands:

```
PYTHONDONTWRITEBYTECODE=1 python3 -I <scratch census driver> > <scratch output file>
python3 tools/iwad_counts.py /mnt/d/DoomIWADs --movers-markdown      # cross-check of the 2x2 window and mover columns
sha256sum /mnt/d/DoomIWADs/*.WAD
cd /home/copgunn/src/DOOM && CARGO_BUILD_JOBS=3 cargo build -p doom-sim --release --locked --example player_level_run
player_level_run --write-idle 5 --demo runs/idleN.lmp --policy episode1 --map E1MN
player_level_run --wad /mnt/d/DoomIWADs/DOOM.WAD --map E1MN --demo runs/idleN.lmp --out runs/outN --policy episode1
```

The 2x2-window values equal the M4a values (E1M5 14, E1M9 9) and the other seven maps equal the M4a note's §2.8 table, so the method was reused exactly. Disk note: /mnt/c had 15 GB free (below the AGENTS.md 20 GB floor) when this ran; only an incremental 14 s rebuild of doom-sim was done and free space did not change.

### 1.2 What refuses E1M5, E1M8 and E1M9 under wire 23

Result of loading each map with the Rust runner under --policy episode1 (five idle tics, release build of this branch's HEAD, built 09:07 PST).

| map | result | error kind |
| --- | --- | --- |
| E1M1 | loads, 5 of 5 tics admitted | none |
| E1M2 | loads, 5 of 5 tics admitted | none |
| E1M3 | loads, 5 of 5 tics admitted | none |
| E1M4 | loads, 5 of 5 tics admitted | none |
| E1M5 | refused at load | Runtime(Pursuit(Specials)) |
| E1M6 | loads, 5 of 5 tics admitted | none |
| E1M7 | loads, 5 of 5 tics admitted | none |
| E1M8 | refused at load | Runtime(Pursuit(Specials)) |
| E1M9 | refused at load | Runtime(Pursuit(Specials)) |

The runner reports the first failing check only. Section 1.3 shows special 97 is the only line special wire 23 does not admit on any E1 map, and it is present on exactly these three maps (E1M5 10 lines, E1M8 8, E1M9 2), consistent with `Specials`; what the loader would check next (for example thing 14 or 3003, or sector rules) cannot be seen without changing code, so was not determined. Sector special 11 (E1M8) is already admitted.

Demo reader, observed with the same runner: DEMO1 as stored in DOOM.WAD is refused with `Header { index: 0, value: 109 }` (the Rust reader requires version 110, crates/doom-sim/src/demo.rs:21). With only the version byte patched to 110 in a scratch copy it is refused with `Header { index: 1, value: 3 }` (skill 3 is outside the admitted header). Both are expected until M4b changes them.

### 1.3 Line specials

Count of linedefs per special on the three target maps and on all of E1 (E1M1-E1M9), independent of skill.

| special | wire 23 | E1M5 | E1M8 | E1M9 | E1 total |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 | admitted | 752 | 310 | 596 | 7066 |
| 1 | admitted | 22 | 2 | 3 | 87 |
| 2 | admitted | 2 | 3 | 17 | 43 |
| 5 | admitted | 0 | 0 | 0 | 1 |
| 7 | admitted | 0 | 1 | 0 | 1 |
| 8 | admitted | 0 | 0 | 0 | 1 |
| 9 | admitted | 0 | 0 | 0 | 1 |
| 11 | admitted | 1 | 0 | 1 | 8 |
| 16 | admitted | 0 | 0 | 0 | 1 |
| 18 | admitted | 0 | 0 | 0 | 2 |
| 20 | admitted | 0 | 0 | 1 | 3 |
| 22 | admitted | 1 | 0 | 0 | 2 |
| 23 | admitted | 0 | 1 | 0 | 2 |
| 26 | admitted | 3 | 0 | 0 | 12 |
| 27 | admitted | 2 | 0 | 0 | 18 |
| 28 | admitted | 0 | 0 | 0 | 8 |
| 31 | admitted | 6 | 4 | 2 | 23 |
| 32 | admitted | 0 | 0 | 2 | 2 |
| 33 | admitted | 0 | 0 | 4 | 4 |
| 34 | admitted | 0 | 0 | 4 | 7 |
| 35 | admitted | 0 | 0 | 0 | 3 |
| 36 | admitted | 0 | 0 | 1 | 5 |
| 38 | admitted | 0 | 0 | 0 | 1 |
| 46 | admitted | 0 | 0 | 0 | 1 |
| 48 | admitted | 0 | 0 | 0 | 12 |
| 51 | admitted | 0 | 0 | 0 | 1 |
| 62 | admitted | 3 | 2 | 6 | 35 |
| 63 | admitted | 0 | 0 | 0 | 4 |
| 70 | admitted | 2 | 0 | 0 | 3 |
| 76 | admitted | 0 | 0 | 0 | 2 |
| 82 | admitted | 0 | 1 | 0 | 1 |
| 86 | admitted | 0 | 0 | 0 | 4 |
| 88 | admitted | 5 | 0 | 11 | 58 |
| 90 | admitted | 1 | 0 | 2 | 20 |
| 91 | admitted | 10 | 1 | 0 | 18 |
| 97 | NOT admitted | 10 | 8 | 2 | 20 |
| 98 | admitted | 3 | 0 | 0 | 4 |
| 103 | admitted | 2 | 0 | 1 | 11 |
| 109 | admitted | 0 | 0 | 0 | 1 |

Same counts for every E1 map (for completeness).

| special | wire 23 | E1M1 | E1M2 | E1M3 | E1M4 | E1M5 | E1M6 | E1M7 | E1M8 | E1M9 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | admitted | 464 | 991 | 974 | 789 | 752 | 1299 | 891 | 310 | 596 |
| 1 | admitted | 8 | 15 | 12 | 6 | 22 | 12 | 7 | 2 | 3 |
| 2 | admitted | 0 | 0 | 9 | 1 | 2 | 8 | 3 | 3 | 17 |
| 5 | admitted | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |
| 7 | admitted | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 |
| 8 | admitted | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 |
| 9 | admitted | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 11 | admitted | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 0 | 1 |
| 16 | admitted | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 |
| 18 | admitted | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 |
| 20 | admitted | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 1 |
| 22 | admitted | 0 | 0 | 0 | 0 | 1 | 0 | 1 | 0 | 0 |
| 23 | admitted | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 0 |
| 26 | admitted | 0 | 0 | 2 | 2 | 3 | 3 | 2 | 0 | 0 |
| 27 | admitted | 0 | 0 | 2 | 2 | 2 | 6 | 6 | 0 | 0 |
| 28 | admitted | 0 | 2 | 0 | 0 | 0 | 4 | 2 | 0 | 0 |
| 31 | admitted | 0 | 2 | 4 | 2 | 6 | 0 | 3 | 4 | 2 |
| 32 | admitted | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 |
| 33 | admitted | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 4 |
| 34 | admitted | 0 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 4 |
| 35 | admitted | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 0 |
| 36 | admitted | 1 | 0 | 0 | 2 | 0 | 1 | 0 | 0 | 1 |
| 38 | admitted | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 46 | admitted | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 48 | admitted | 8 | 0 | 0 | 0 | 0 | 0 | 4 | 0 | 0 |
| 51 | admitted | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 |
| 62 | admitted | 0 | 6 | 4 | 3 | 3 | 4 | 7 | 2 | 6 |
| 63 | admitted | 1 | 0 | 1 | 0 | 0 | 0 | 2 | 0 | 0 |
| 70 | admitted | 0 | 0 | 0 | 0 | 2 | 0 | 1 | 0 | 0 |
| 76 | admitted | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 |
| 82 | admitted | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 |
| 86 | admitted | 0 | 0 | 0 | 4 | 0 | 0 | 0 | 0 | 0 |
| 88 | admitted | 1 | 11 | 7 | 3 | 5 | 8 | 12 | 0 | 11 |
| 90 | admitted | 0 | 0 | 2 | 12 | 1 | 0 | 3 | 0 | 2 |
| 91 | admitted | 0 | 0 | 0 | 0 | 10 | 0 | 7 | 1 | 0 |
| 97 | NOT admitted | 0 | 0 | 0 | 0 | 10 | 0 | 0 | 8 | 2 |
| 98 | admitted | 0 | 0 | 0 | 0 | 3 | 0 | 1 | 0 | 0 |
| 103 | admitted | 0 | 3 | 1 | 0 | 2 | 3 | 1 | 0 | 1 |
| 109 | admitted | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| linedefs |  | 486 | 1033 | 1026 | 830 | 825 | 1352 | 958 | 333 | 653 |

Lines with a special that wire 23 does not admit, per map.

| map | specials (special x lines) |
| --- | ---: |
| E1M1 | none |
| E1M2 | none |
| E1M3 | none |
| E1M4 | none |
| E1M5 | 97 x10 |
| E1M6 | none |
| E1M7 | none |
| E1M8 | 97 x8 |
| E1M9 | 97 x2 |

### 1.4 Sector specials

Count of sectors per special per E1 map.

| special | wire 23 | E1M1 | E1M2 | E1M3 | E1M4 | E1M5 | E1M6 | E1M7 | E1M8 | E1M9 | E1 total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | admitted | 77 | 163 | 145 | 116 | 113 | 204 | 142 | 63 | 130 | 1153 |
| 1 | admitted | 1 | 3 | 8 | 4 | 2 | 5 | 2 | 4 | 0 | 29 |
| 2 | admitted | 0 | 1 | 0 | 1 | 4 | 1 | 1 | 0 | 0 | 8 |
| 3 | admitted | 0 | 5 | 0 | 0 | 0 | 1 | 3 | 0 | 0 | 9 |
| 7 | admitted | 4 | 5 | 8 | 8 | 7 | 30 | 8 | 1 | 10 | 81 |
| 8 | admitted | 2 | 13 | 4 | 4 | 8 | 2 | 7 | 4 | 4 | 48 |
| 9 | admitted | 3 | 6 | 7 | 3 | 9 | 4 | 4 | 1 | 2 | 39 |
| 11 | admitted | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 1 |
| 12 | admitted | 1 | 1 | 0 | 1 | 0 | 3 | 3 | 0 | 0 | 9 |
| 13 | admitted | 0 | 3 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 5 |
| 16 | admitted | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 1 | 6 |
| sectors |  | 88 | 200 | 177 | 139 | 143 | 250 | 170 | 74 | 147 | 1388 |

### 1.5 Tags

Tag counts per E1 map (tag 666 is the tag A_BossDeath uses).

| count | E1M1 | E1M2 | E1M3 | E1M4 | E1M5 | E1M6 | E1M7 | E1M8 | E1M9 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| distinct nonzero line tags | 5 | 10 | 19 | 9 | 9 | 14 | 14 | 6 | 12 |
| distinct nonzero sector tags | 5 | 11 | 20 | 8 | 9 | 14 | 14 | 10 | 12 |
| distinct nonzero tags (lines and sectors) | 5 | 11 | 20 | 9 | 9 | 14 | 14 | 10 | 12 |
| sectors with tag 666 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 |
| lines with tag 666 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| lines with a nonzero tag | 5 | 22 | 30 | 30 | 39 | 27 | 39 | 17 | 41 |
| sectors with a nonzero tag | 5 | 13 | 28 | 12 | 12 | 20 | 16 | 17 | 17 |

### 1.6 THING doomednums

Spawned THING counts on E1M5 (starts 1-4 and 11 are handled on every skill; others need the skill bit and not bit 16).

| doomednum | info.c name | wire 23 | skill 2 | skill 3 | delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | player 1 start | admitted | 1 | 1 | +0 |
| 2 | player 2 start | admitted | 1 | 1 | +0 |
| 3 | player 3 start | admitted | 1 | 1 | +0 |
| 4 | player 4 start | admitted | 1 | 1 | +0 |
| 5 | MT_MISC4 | admitted | 1 | 1 | +0 |
| 6 | MT_MISC6 | admitted | 1 | 1 | +0 |
| 8 | MT_MISC24 | admitted | 1 | 1 | +0 |
| 9 | MT_SHOTGUY | admitted | 24 | 46 | +22 |
| 11 | deathmatch start | admitted | 8 | 8 | +0 |
| 14 | MT_TELEPORTMAN | NOT admitted | 1 | 1 | +0 |
| 35 | MT_MISC50 | admitted | 2 | 2 | +0 |
| 58 | MT_SHADOWS | admitted | 9 | 14 | +5 |
| 2001 | MT_SHOTGUN | admitted | 1 | 1 | +0 |
| 2002 | MT_CHAINGUN | admitted | 1 | 1 | +0 |
| 2003 | MT_MISC27 | admitted | 1 | 1 | +0 |
| 2005 | MT_MISC26 | admitted | 1 | 1 | +0 |
| 2007 | MT_CLIP | admitted | 2 | 2 | +0 |
| 2008 | MT_MISC22 | admitted | 16 | 16 | +0 |
| 2010 | MT_MISC18 | admitted | 3 | 3 | +0 |
| 2011 | MT_MISC10 | admitted | 14 | 14 | +0 |
| 2012 | MT_MISC11 | admitted | 12 | 13 | +1 |
| 2013 | MT_MISC12 | admitted | 1 | 1 | +0 |
| 2014 | MT_MISC2 | admitted | 11 | 11 | +0 |
| 2015 | MT_MISC3 | admitted | 14 | 14 | +0 |
| 2018 | MT_MISC0 | admitted | 2 | 2 | +0 |
| 2019 | MT_MISC1 | admitted | 2 | 2 | +0 |
| 2024 | MT_INS | admitted | 1 | 1 | +0 |
| 2025 | MT_MISC14 | admitted | 1 | 1 | +0 |
| 2026 | MT_MISC15 | admitted | 1 | 1 | +0 |
| 2035 | MT_BARREL | admitted | 28 | 28 | +0 |
| 2045 | MT_MISC16 | admitted | 1 | 1 | +0 |
| 2046 | MT_MISC19 | admitted | 1 | 1 | +0 |
| 2048 | MT_MISC17 | admitted | 2 | 2 | +0 |
| 2049 | MT_MISC23 | admitted | 2 | 2 | +0 |
| 3001 | MT_TROOP | admitted | 26 | 31 | +5 |
| 3002 | MT_SERGEANT | admitted | 4 | 12 | +8 |
| 3004 | MT_POSSESSED | admitted | 28 | 28 | +0 |
|  | **handled total** |  | 227 | 268 | +41 |
|  | **records in lump** |  | 293 | 293 |  |

Spawned THING counts on E1M8 (starts 1-4 and 11 are handled on every skill; others need the skill bit and not bit 16).

| doomednum | info.c name | wire 23 | skill 2 | skill 3 | delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | player 1 start | admitted | 1 | 1 | +0 |
| 2 | player 2 start | admitted | 1 | 1 | +0 |
| 3 | player 3 start | admitted | 1 | 1 | +0 |
| 4 | player 4 start | admitted | 1 | 1 | +0 |
| 9 | MT_SHOTGUY | admitted | 6 | 6 | +0 |
| 11 | deathmatch start | admitted | 9 | 9 | +0 |
| 14 | MT_TELEPORTMAN | NOT admitted | 1 | 1 | +0 |
| 46 | MT_MISC43 | admitted | 6 | 6 | +0 |
| 58 | MT_SHADOWS | admitted | 0 | 10 | +10 |
| 2001 | MT_SHOTGUN | admitted | 2 | 2 | +0 |
| 2002 | MT_CHAINGUN | admitted | 1 | 1 | +0 |
| 2011 | MT_MISC10 | admitted | 11 | 11 | +0 |
| 2012 | MT_MISC11 | admitted | 3 | 3 | +0 |
| 2013 | MT_MISC12 | admitted | 1 | 1 | +0 |
| 2018 | MT_MISC0 | admitted | 1 | 1 | +0 |
| 2024 | MT_INS | admitted | 1 | 1 | +0 |
| 2026 | MT_MISC15 | admitted | 1 | 1 | +0 |
| 2035 | MT_BARREL | admitted | 28 | 28 | +0 |
| 2048 | MT_MISC17 | admitted | 11 | 11 | +0 |
| 2049 | MT_MISC23 | admitted | 4 | 4 | +0 |
| 3001 | MT_TROOP | admitted | 5 | 5 | +0 |
| 3002 | MT_SERGEANT | admitted | 14 | 18 | +4 |
| 3003 | MT_BRUISER | NOT admitted | 2 | 2 | +0 |
|  | **handled total** |  | 111 | 125 | +14 |
|  | **records in lump** |  | 126 | 126 |  |

Spawned THING counts on E1M9 (starts 1-4 and 11 are handled on every skill; others need the skill bit and not bit 16).

| doomednum | info.c name | wire 23 | skill 2 | skill 3 | delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | player 1 start | admitted | 1 | 1 | +0 |
| 2 | player 2 start | admitted | 1 | 1 | +0 |
| 3 | player 3 start | admitted | 1 | 1 | +0 |
| 4 | player 4 start | admitted | 1 | 1 | +0 |
| 5 | MT_MISC4 | admitted | 1 | 1 | +0 |
| 6 | MT_MISC6 | admitted | 1 | 1 | +0 |
| 8 | MT_MISC24 | admitted | 1 | 1 | +0 |
| 9 | MT_SHOTGUY | admitted | 21 | 29 | +8 |
| 11 | deathmatch start | admitted | 10 | 10 | +0 |
| 13 | MT_MISC5 | admitted | 1 | 1 | +0 |
| 14 | MT_TELEPORTMAN | NOT admitted | 1 | 1 | +0 |
| 34 | MT_MISC49 | admitted | 7 | 7 | +0 |
| 35 | MT_MISC50 | admitted | 1 | 1 | +0 |
| 46 | MT_MISC43 | admitted | 4 | 4 | +0 |
| 48 | MT_MISC48 | admitted | 2 | 2 | +0 |
| 58 | MT_SHADOWS | admitted | 5 | 14 | +9 |
| 2001 | MT_SHOTGUN | admitted | 1 | 1 | +0 |
| 2002 | MT_CHAINGUN | admitted | 1 | 1 | +0 |
| 2003 | MT_MISC27 | admitted | 1 | 1 | +0 |
| 2005 | MT_MISC26 | admitted | 1 | 1 | +0 |
| 2007 | MT_CLIP | admitted | 4 | 4 | +0 |
| 2008 | MT_MISC22 | admitted | 1 | 1 | +0 |
| 2011 | MT_MISC10 | admitted | 6 | 6 | +0 |
| 2012 | MT_MISC11 | admitted | 6 | 6 | +0 |
| 2014 | MT_MISC2 | admitted | 16 | 16 | +0 |
| 2015 | MT_MISC3 | admitted | 27 | 27 | +0 |
| 2018 | MT_MISC0 | admitted | 1 | 1 | +0 |
| 2028 | MT_MISC31 | admitted | 5 | 5 | +0 |
| 2035 | MT_BARREL | admitted | 20 | 20 | +0 |
| 2046 | MT_MISC19 | admitted | 2 | 2 | +0 |
| 2048 | MT_MISC17 | admitted | 6 | 6 | +0 |
| 2049 | MT_MISC23 | admitted | 2 | 2 | +0 |
| 3001 | MT_TROOP | admitted | 30 | 40 | +10 |
| 3002 | MT_SERGEANT | admitted | 11 | 15 | +4 |
| 3004 | MT_POSSESSED | admitted | 5 | 5 | +0 |
|  | **handled total** |  | 205 | 236 | +31 |
|  | **records in lump** |  | 237 | 237 |  |

Skill 3 minus skill 2, per doomednum, for all nine maps (only doomednums whose count differs; `+` = more on hard).

| map | handled skill 2 | handled skill 3 | net | per-doomednum delta |
| --- | ---: | ---: | ---: | ---: |
| E1M1 | 100 | 124 | +24 | 9 +16, 2014 +1, 3001 +2, 3004 +5 |
| E1M2 | 213 | 252 | +39 | 9 +8, 2049 +1, 3001 +10, 3004 +20 |
| E1M3 | 322 | 377 | +55 | 9 +28, 58 +2, 2028 -2, 3001 +18, 3002 +5, 3004 +4 |
| E1M4 | 213 | 241 | +28 | 9 +7, 2007 -2, 2025 -1, 3001 +16, 3002 +8 |
| E1M5 | 227 | 268 | +41 | 9 +22, 58 +5, 2012 +1, 3001 +5, 3002 +8 |
| E1M6 | 384 | 454 | +70 | 9 +21, 58 +13, 2011 -4, 2012 -3, 2014 +4, 2015 +10, 2018 +1, 2035 +2, 2049 +1, 3001 +14, 3002 +11 |
| E1M7 | 280 | 343 | +63 | 9 +37, 2001 -1, 2012 -2, 3001 +9, 3002 +4, 3004 +16 |
| E1M8 | 111 | 125 | +14 | 58 +10, 3002 +4 |
| E1M9 | 205 | 236 | +31 | 9 +8, 58 +9, 3001 +10, 3002 +4 |

Things not admitted by wire 23, per map, on each skill (doomednum x count).

| map | skill 2 | skill 3 |
| --- | ---: | ---: |
| E1M1 | none | none |
| E1M2 | none | none |
| E1M3 | none | none |
| E1M4 | none | none |
| E1M5 | 14 x1 | 14 x1 |
| E1M6 | none | none |
| E1M7 | none | none |
| E1M8 | 14 x1, 3003 x2 | 14 x1, 3003 x2 |
| E1M9 | 14 x1 | 14 x1 |

Union over E1 of non-admitted doomednums: skill 2 (medium): 14, 3003; skill 3 (hard): 14, 3003.

Doomednums unknown to info.c on either skill: none.

### 1.7 Special-line window (spechit bound)

Largest number of distinct two-sided special lines in any 2x2 window of BLOCKMAP cells (`MapCounts.window_specials`, tools/iwad_counts.py:710-722, the M4a method), against MAXSPECIALCROSS 8.

| map | max window | <= 8 |
| --- | ---: | ---: |
| E1M1 | 8 | yes |
| E1M2 | 4 | yes |
| E1M3 | 6 | yes |
| E1M4 | 6 | yes |
| E1M5 | 14 | NO (over by 6) |
| E1M6 | 6 | yes |
| E1M7 | 8 | yes |
| E1M8 | 8 | yes |
| E1M9 | 9 | NO (over by 1) |

### 1.8 Scale and mover bounds

Map size and mover facts per E1 map (from `MapCounts`, skill independent).

| map | vertices | linedefs | sidedefs | sectors | THING records | static floor range | static ceiling range | reachable height range (admitted) | mover sectors | mover families | multi-family sectors | plat sectors | reusable switches | scrollers (two-sided) | sky movers | max plane step |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| E1M1 | 470 | 486 | 666 | 88 | 143 | [-136, 136] | [-40, 264] | [-136, 264] | 9 | door 6, floor 2, plat 1 | 0 | 1 | 1 | 8 (8) | 0 | 8 |
| E1M2 | 942 | 1033 | 1323 | 200 | 262 | [-232, 272] | [-144, 512] | [-232, 512] | 19 | door 13, floor 1, plat 5 | 0 | 5 | 7 | 0 (0) | 0 | 4 |
| E1M3 | 946 | 1026 | 1326 | 177 | 380 | [-32, 368] | [64, 528] | [-32, 528] | 30 | door 20, floor 1, plat 9 | 0 | 9 | 5 | 0 (0) | 3 | 4 |
| E1M4 | 780 | 830 | 1054 | 139 | 254 | [-64, 328] | [0, 400] | [-64, 400] | 17 | door 11, floor 4, plat 3 | 1 | 3 | 3 | 0 (0) | 0 | 4 |
| E1M5 | 746 | 825 | 1053 | 143 | 293 | [-216, 112] | [-184, 248] | [-216, 248] | 27 | door 21, floor 2, plat 4 | 0 | 4 | 5 | 0 (0) | 0 | 4 |
| E1M6 | 1207 | 1352 | 1727 | 250 | 463 | [-80, 192] | [-24, 304] | [-80, 304] | 34 | door 30, floor 1, plat 3 | 0 | 3 | 4 | 0 (0) | 2 | 4 |
| E1M7 | 896 | 958 | 1223 | 170 | 358 | [-32, 264] | [-16, 320] | [-32, 320] | 26 | door 18, floor 2, plat 6 | 0 | 6 | 10 | 4 (4) | 0 | 4 |
| E1M8 | 328 | 333 | 511 | 74 | 126 | [-136, 208] | [-104, 240] | [-136, 240] | 9 | door 5, floor 3, plat 1 | 0 | 1 | 2 | 0 (0) | 2 | 2 |
| E1M9 | 581 | 653 | 902 | 147 | 237 | [-168, 208] | [-72, 304] | [-168, 304] | 23 | door 14, floor 3, plat 6 | 0 | 6 | 6 | 0 (0) | 4 | 4 |

Live-actor bound inputs per skill, counted with the wire-23 class sets (`Episode1Bounds` rule: starts excluded, bit 16 excluded). K monsters, R droppers, P pickups, C keycards, B barrels, D statics; `spawned` = all things handled other than starts (player 1 excluded, as the Rust runner's `spawned` line counts); `other` = spawned things of types wire 23 does not admit (not counted in K..D).

| map | skill | K | R | P | C | B | D | other | spawned (starts excluded) | actor slots |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| E1M1 | skill 2 (medium) | 6 | 4 | 52 | 0 | 6 | 27 | 0 | 91 | 1024 |
| E1M1 | skill 3 (hard) | 29 | 25 | 53 | 0 | 6 | 27 | 0 | 115 | 1024 |
| E1M2 | skill 2 (medium) | 41 | 33 | 106 | 1 | 24 | 28 | 0 | 199 | 1024 |
| E1M2 | skill 3 (hard) | 79 | 61 | 107 | 1 | 24 | 28 | 0 | 238 | 1024 |
| E1M3 | skill 2 (medium) | 74 | 43 | 175 | 2 | 28 | 32 | 0 | 309 | 1024 |
| E1M3 | skill 3 (hard) | 131 | 75 | 175 | 2 | 28 | 30 | 0 | 364 | 1024 |
| E1M4 | skill 2 (medium) | 54 | 31 | 89 | 2 | 32 | 25 | 0 | 200 | 1024 |
| E1M4 | skill 3 (hard) | 85 | 38 | 86 | 2 | 32 | 25 | 0 | 228 | 1024 |
| E1M5 | skill 2 (medium) | 91 | 52 | 93 | 2 | 28 | 2 | 1 | 215 | 1024 |
| E1M5 | skill 3 (hard) | 131 | 74 | 94 | 2 | 28 | 2 | 1 | 256 | 1024 |
| E1M6 | skill 2 (medium) | 118 | 57 | 214 | 3 | 22 | 15 | 0 | 369 | 1024 |
| E1M6 | skill 3 (hard) | 177 | 78 | 223 | 3 | 24 | 15 | 0 | 439 | 1024 |
| E1M7 | skill 2 (medium) | 84 | 50 | 146 | 3 | 16 | 20 | 0 | 266 | 1024 |
| E1M7 | skill 3 (hard) | 150 | 103 | 143 | 3 | 16 | 20 | 0 | 329 | 1024 |
| E1M8 | skill 2 (medium) | 25 | 6 | 36 | 0 | 28 | 6 | 3 | 98 | 1024 |
| E1M8 | skill 3 (hard) | 39 | 6 | 36 | 0 | 28 | 6 | 3 | 112 | 1024 |
| E1M9 | skill 2 (medium) | 72 | 26 | 79 | 3 | 20 | 19 | 1 | 191 | 1024 |
| E1M9 | skill 3 (hard) | 103 | 34 | 79 | 3 | 20 | 19 | 1 | 222 | 1024 |

### 1.9 Demo lumps

Header facts of each DEMOn lump (skill is 0-based: 2 = hurt me plenty, 3 = ultra-violence; 0x80 marker checked at the expected end).

| lump | bytes | version | skill | episode | map | deathmatch | respawn | fast | nomonsters | consoleplayer | playeringame[0..3] | ticcmds | ends with 0x80 | stray bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| DEMO1 | 6854 | 109 | 3 | 1 | 5 | 0 | 0 | 0 | 0 | 0 | 1/0/0/0 | 1710 | yes | 0 |
| DEMO2 | 9402 | 109 | 3 | 2 | 2 | 0 | 0 | 0 | 0 | 0 | 1/0/0/0 | 2347 | yes | 0 |
| DEMO3 | 15466 | 109 | 3 | 3 | 5 | 0 | 0 | 0 | 0 | 0 | 1/0/0/0 | 3863 | yes | 0 |
| DEMO4 | 3286 | 109 | 3 | 4 | 2 | 0 | 0 | 0 | 0 | 0 | 1/0/0/0 | 818 | yes | 0 |

Command-domain bounds (counts and extremes only; `change` shows the weapon index seen x number of commands).

| lump | ticcmds | min fwd | max fwd | min side | max side | fwd = -128 | side = -128 | BT_ATTACK | BT_USE | BT_CHANGE | weapon indices | buttons & 0x40 | BT_SPECIAL 0x80 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| DEMO1 | 1710 | -50 | 50 | -50 | 50 | 0 | 0 | 412 | 4 | 0 | none | 0 | 0 |
| DEMO2 | 2347 | -50 | 50 | -50 | 50 | 0 | 0 | 438 | 8 | 0 | none | 0 | 0 |
| DEMO3 | 3863 | -50 | 50 | -50 | 50 | 0 | 0 | 994 | 34 | 0 | none | 0 | 0 |
| DEMO4 | 818 | -50 | 50 | -50 | 50 | 0 | 0 | 198 | 5 | 0 | none | 0 | 0 |

### 1.10 Observations worth the owner's attention

- DEMO1 is E1M5 (not E1M1), skill 3, one player, 1710 ticcmds, ending with 0x80, and its version byte is 109. doomdef.h defines VERSION as 110 and G_DoPlayDemo (g_game.c:1589) returns with "Demo is from a different game version!" when the byte differs, so the genuine 1.10 engine does not play this lump as stored. DEMO2, DEMO3 and DEMO4 also carry 109. How the harness feeds the C side (header rewrite or direct command injection) is a design question for M4b; it was not tested here.
- All four demos stay within forwardmove and sidemove of -50..50, never use -128, BT_CHANGE, bit 0x40 or BT_SPECIAL, and use only attack and use buttons.
- Unported items on E1: line special 97 (teleport, E1M5/E1M8/E1M9), thing 14 (once each on those three maps) and thing 3003 (E1M8, 2 on both skills). Nothing else is outside wire 23 on skill 2 or skill 3. No doomednum is unknown to info.c.
- Tag 666: one sector on E1M8, no line with tag 666 on any E1 map (E1M8 has no exit line, per M4a).
- Skill 3 adds Spectres (58) on E1M3 (+2) and E1M8 (+10, none on skill 2), otherwise only raises counts of 9, 3001, 3002, 3004 and 58; no new doomednum outside wire 23 appears (section 1.6). Hard also has fewer of a few pickups and decorations on E1M3, E1M4, E1M6 and E1M7.
- The reachable-height ranges and mover facts do not depend on skill. The M4a numbers are reproduced exactly (E1M3 height 528 is the only value above 512).

## 2. C facts: teleportation

Source: `linuxdoom-1.10/` in this repository (id Software revision `a77dfb96cb91780ca334d0d4cfd86957558007e0`, the runner's `REVISION`, tools/reference/m3h9c_build.py:45). Every path below is relative to the repository root. Facts marked *observed* come from a scratch gcc probe compiled on this host (GCC 15.2.0, x86-64) outside the tree; nothing in the tree was built, changed or run for this section. The IWAD was not read for it; per-map counts are quoted from the committed M4a census (docs/M4A_REFERENCE_INVESTIGATION.md §2) and the census of §1 of this note. Where this section cites a section number of the M4a note it says so; "T" numbers (T1–T10) are this section's own subsections.

### T1. Dispatch: which specials teleport, and who may trigger them

1. `EV_Teleport` is called only from `P_CrossSpecialLine` (linuxdoom-1.10/p_spec.c:492–950), at four cases: 39 (p_spec.c:658–662),
   125 (765–772), 97 (904–907) and 126 (934–938). No other function calls it: `P_ShootSpecialLine` handles only 24, 46 and 47
   (p_spec.c:959–1000) and `P_UseSpecialLine` (linuxdoom-1.10/p_switch.c:276 onward) has no teleport case. Every call site ignores
   `EV_Teleport`'s return value.
2. `P_CrossSpecialLine` is reached only from `P_TryMove`'s crossing loop, called with `oldside`, the side of the thing's **old**
   position (linuxdoom-1.10/p_map.c:511). Inside `EV_Teleport` the parameter is named `side`.
3. A non-player first passes a **type** filter: `MT_ROCKET`, `MT_PLASMA`, `MT_BFG`, `MT_TROOPSHOT`, `MT_HEADSHOT` and `MT_BRUISERSHOT`
   return at once (p_spec.c:503–518). It then passes a special filter that admits 39, 97, 125, 126, 4, 10 and 88 (520–534). So
   monsters, **corpses** and **barrels** (anything with no `player` whose type is not in that list) can trigger 97 if they have
   momentum or walk across it.
4. Semantics of the four specials:
   - **97 (WR)**: `EV_Teleport`, the line keeps its special (p_spec.c:904–907). This is the only teleport special in Episode 1 (M4a
     census §2.2: E1M5 ×10, E1M8 ×8, E1M9 ×2; the aggregated 39/97/125/126 row in §2.6 has the same counts, so 39, 125 and 126 are
     absent).
   - **39 (W1)**: `EV_Teleport`, then `line->special = 0` **whatever happened**, so a back-side crossing, a missing landing or a
     blocked monster still uses up the line (p_spec.c:658–662).
   - **125 (W1, monsters only)**: when `!thing->player`, it calls `EV_Teleport` and then clears the special whatever the result. A player
     crossing does nothing and the line stays (p_spec.c:765–772).
   - **126 (WR, monsters only)**: `EV_Teleport` for a non-player, otherwise nothing (p_spec.c:934–938).
   Episode 1 needs only 97. A port can refuse 39, 125 and 126 at load, or model them exactly as above.
5. A use trace stops at the first line that has **any** special, teleport lines included: `PTR_UseTraverse` calls `P_UseSpecialLine`
   and returns false (p_map.c:1095–1123). A 97 line therefore blocks a use press from reaching a usable line behind it, and the press does
   nothing (no `sfx_noway`, which is played only for a non-special line with no opening, p_map.c:1099–1110).

### T2. `EV_Teleport` (linuxdoom-1.10/p_telept.c:47–131), in execution order

1. `if (thing->flags & MF_MISSILE) return 0` (p_telept.c:65–66). In Episode 1 every missile type (troop shot, baron shot, rocket) is
   already rejected by type in `P_CrossSpecialLine` (T1.3), so this test never decides anything there.
2. `if (side == 1) return 0` (70–71): a thing whose **old** position was on the back side does nothing. A teleport happens only when
   the thing goes from the front side (0) to the back side.
3. `tag = line->tag` (74). There is **no tag-0 guard**, so a tag-0 teleport line searches every tag-0 sector.
4. Search: the outer loop runs `i = 0 .. numsectors−1` in index order and keeps sectors with `sectors[i].tag == tag` (75–78). For each
   such sector it walks the **whole thinker list** from `thinkercap.next` (79–83; the assignment at 79 is a dead store). It skips
   thinkers whose `function.acp1 != P_MobjThinker` (85–86), which also skips removed mobjs because they carry `(actionf_v)(−1)`
   (linuxdoom-1.10/p_tick.c:80–84). It skips mobjs whose `type != MT_TELEPORTMAN` (91–92) and landings with
   `m->subsector->sector − sectors != i` (94–97). **The first landing found wins**: the lowest-indexed tagged sector that contains a
   landing (judged by the landing's `subsector`, not by distance), and inside it the earliest landing in thinker order. Landings are
   spawned at level load in THINGS-lump order and never move or get removed, so that is THINGS order. The cost is
   `O(tagged sectors × thinkers)` per crossing, with no array behind it. EV_Teleport never reads sector `thinglist`s: in linuxdoom-1.10
   no compiled gameplay code reads them (the only reader, p_doors.c:669, sits inside `#if 0`, p_doors.c:557–764).
5. `oldx/oldy/oldz = thing->x/y/z` (99–101).
6. `if (!P_TeleportMove (thing, m->x, m->y)) return 0` (103–104). A failed move ends the whole teleport: no other landing is tried, no
   fog spawns, angle and momentum are unchanged. The thinker scan stops at the first match whether the move succeeds or fails, so it never
   continues over lists that `P_TeleportMove` changed.
7. `thing->z = thing->floorz` (106), for players and monsters alike. For a player, `player->viewz = z + viewheight` (107–108).
   `viewheight` and `deltaviewheight` are **not** reset.
8. Fog 1: `P_SpawnMobj (oldx, oldy, oldz, MT_TFOG)`, then `S_StartSound (fog, sfx_telept)` (111–112).
9. Fog 2: `an = m->angle >> ANGLETOFINESHIFT` (113; `an` is `unsigned`, 57). Then
   `P_SpawnMobj (m->x + 20*finecosine[an], m->y + 20*finesine[an], thing->z, MT_TFOG)` and `S_StartSound (fog, sfx_telept)`
   (114–118). `finecosine = &finesine[FINEANGLES/4]` (linuxdoom-1.10/r_main.c:113). Landing angles are multiples of `ANG45`
   (linuxdoom-1.10/p_mobj.c:794), so `an` ∈ {0, 1024, …, 7168}. The table is offset by half a step (`finesine[0] = 25`), so the offset is
   **not exactly 20 units**. *Observed* from tables.c: angle 0 gives (+1310700, +500), i.e. +19.9997 and +0.0076 units; 45° gives
   (+926460, +927160); 90° gives (−500, +1310700); the other octants follow by symmetry. Fog 2's position is not checked: it can fall in
   another sector or behind a wall.
10. Player only: `thing->reactiontime = 18` (121–122).
11. `thing->angle = m->angle` (124); `momx = momy = momz = 0` (125); `return 1` (126). If no tagged sector holds a landing, it returns 0
    (130).

### T3. `P_TeleportMove` and `PIT_StompThing` (linuxdoom-1.10/p_map.c:81–177)

1. Globals set, in this order (p_map.c:130–152): `tmthing = thing`, `tmflags = thing->flags`, `tmx/tmy` = destination, `tmbbox`
   = destination ± `thing->radius`. `newsubsec = R_PointInSubsector (x, y)`. `ceilingline = NULL`.
   `tmfloorz = tmdropoffz = newsubsec->sector->floorheight` and `tmceilingz = ceilingheight` come from the **centre subsector's sector
   only**, read live, so a landing in a moving sector uses the current heights. Then `validcount++` and **`numspechit = 0`**. All of this
   happens **before** the stomp loop, so it happens on failure too.
2. There is **no line pass and no fit test**. Nearby lines never adjust `tmfloorz/tmceilingz`; neither `tmceilingz − tmfloorz ≥ height`
   nor any step is checked; `MF_NOCLIP`/`tmflags` are not consulted. `validcount++` is consumed for nothing.
3. Cell range: `xl/xh/yl/yh = (tmbbox ± MAXRADIUS − bmaporg) >> MAPBLOCKSHIFT` (155–158; `MAXRADIUS` = 32 units,
   linuxdoom-1.10/p_local.h:51). Iteration runs `bx` outer and `by` inner (160–161). In each cell `P_BlockThingsIterator` walks
   `blocklinks[cell]` by `bnext` and returns true, iterating nothing, for a cell outside the blockmap (linuxdoom-1.10/p_maputl.c:512–537).
   On the first false the move fails (162–163).
4. `PIT_StompThing` (81–108):
   - skip anything not `MF_SHOOTABLE` (85–86). That includes corpses: `P_KillMobj` clears `MF_SHOOTABLE`, linuxdoom-1.10/p_inter.c:675.
     It also includes items and solid decorations.
   - skip unless `|thing.x − tmx| < blockdist` **and** `|thing.y − tmy| < blockdist`, with `blockdist` = the summed radii (88–95).
     There is **no z test**.
   - skip itself (98–99).
   - **`if (!tmthing->player && gamemap != 30) return false`** (102–103). `gamemap` is 1–9 in Episode 1, so a monster, corpse or barrel
     teleport **fails** whenever any other shootable thing overlaps the landing, the player included. A non-player never telefrags in
     Episode 1.
   - otherwise (the mover is a player-linked mobj), **`P_DamageMobj (thing, tmthing, tmthing, 10000)`** (105) and return true, which
     continues the iteration.
5. On success: `P_UnsetThingPosition`, `floorz = tmfloorz`, `ceilingz = tmceilingz`, `x`, `y`, then `P_SetThingPosition`
   (167–174). `z` is left to the caller (T2.7). `P_SetThingPosition` puts the thing at the **head** of its new sector list and block cell
   (p_maputl.c:396–450).
6. On failure, the globals of T3.1 already hold the destination values (`tmx/tmy/tmbbox/tmfloorz/tmceilingz/tmdropoffz`,
   `ceilingline = NULL`, `numspechit = 0`). The thing itself is unchanged.
7. Stale globals afterwards. The teleport returns into `P_TryMove`, which returns true (p_map.c:516). None of the consumers of
   `tmfloorz`, `floatok`, `ceilingline` or `numspechit` reads them on a true return: `P_Move`'s failure branch (linuxdoom-1.10/p_enemy.c:295–325)
   and the missile branch of `P_XYMovement` (linuxdoom-1.10/p_mobj.c:173–187). Every later movement calls `P_CheckPosition`, which
   resets them (p_map.c:388–410). `floatok` stays true from the outer `P_TryMove` (471). After a successful teleport, however,
   **`thing->floorz/ceilingz` ignore nearby lines** until the thing's next `P_CheckPosition`. So a thing landing half over a step keeps the
   centre sector's floor, and a thing taller than the landing sector keeps a `ceilingz − floorz` smaller than its height.
   `P_ZMovement` then clamps `z = ceilingz − height` (p_mobj.c:328–335), which can be **below** `floorz`. That is defined vanilla
   behaviour.
8. Iterating during damage. Inside the stomp loop, `P_DamageMobj` → `P_KillMobj` → `P_SetMobjState (deathstate or xdeathstate)`.
   For every shootable type in Episode 1 the first death state has nonzero tics and no action: `S_POSS_XDIE1`, `S_SPOS_XDIE1`,
   `S_TROO_XDIE1`, `S_SARG_DIE1` (also used by the Spectre), `S_BOSS_DIE1`, `S_BEXP` (linuxdoom-1.10/info.c:330, 363, 598, 626, 678, 944).
   So nothing is removed, no action runs and `tm*` is not reused during the loop. The drop item of `MT_POSSESSED`/`MT_SHOTGUY`
   (p_inter.c:739–757) is linked at the **head** of the victim's cell, behind the iterator's position, and it is `MF_SPECIAL`, not
   shootable. The iteration continues from `victim->bnext`, unchanged.
9. A player victim reaches `P_DamageMobj`'s player branch only when a player-linked mobj is stomped. In single player that needs a second
   mobj with `player` set: a "voodoo doll" left by a duplicate player-1 start, because `P_SpawnPlayer` re-points `p->mo` but the old mobj
   keeps `mobj->player = p` (p_mobj.c:665–675). Whether an Episode 1 map has duplicate starts is a census question not answered here.

### T4. The fog actor `MT_TFOG` and order consequences

1. `MT_TFOG` (info.c:2122–2146): spawn state `S_TFOG`, radius 20, height 16, mass 100, flags `MF_NOBLOCKMAP|MF_NOGRAVITY`. It has
   `MF_NOSECTOR` clear, so it **is** linked into its sector's thing list. Its states are `S_TFOG`, `S_TFOG01`, `S_TFOG02`, `S_TFOG2` …
   `S_TFOG10`: twelve states of 6 tics each, all with no action, ending at `S_NULL` (info.c:266–277).
2. `P_SpawnMobj` (p_mobj.c:479–534): `memset`, then `reactiontime = info->reactiontime` (8) unless the skill is nightmare (503–504).
   **`lastlook = P_Random () % MAXPLAYERS`** (506) is one gameplay draw per fog. The state is set directly (no action), then
   `P_SetThingPosition`, then `floorz/ceilingz` from the fog's own subsector (519–520), then `z` as passed (526–527), then
   `P_AddThinker` at the **tail** (531; p_tick.c:65–71). There is no spawn-tics draw: that happens only in `P_SpawnMapThing`,
   p_mobj.c:787–788.
3. Order within one successful teleport: any stomp kills and their drops, then the relink, then fog 1 (thinker *n*), then fog 2
   (thinker *n+1*). `P_RunThinkers` walks to the live tail (p_tick.c:101–122), so a fog spawned during the thinker pass **thinks in its
   spawn tic** (tics 6→5). Its 72nd think runs `S_TFOG10 → S_NULL` → `P_RemoveMobj`, which unlinks it from the sector, calls `S_StopSound`
   and marks the thinker −1 (p_mobj.c:546–570). It is freed when the next pass reaches it (p_tick.c:108–114). Spawned in the pass of
   tic T, it is marked removed in tic T+71 and freed in tic T+72.
4. Fog z: fog 1 gets the thing's **old** z, fog 2 the **new** z, which is the landing floor. Fog 2's own `floorz` comes from wherever its
   offset point lies. `P_MobjThinker` runs `P_ZMovement` while `z != floorz` (p_mobj.c:428–436). Under `MF_NOGRAVITY`, z changes only
   by clamping: up to `floorz` (284–311) and down to `ceilingz − 16` (328–335). Fogs are not in the blockmap, so `P_ChangeSector`, which
   iterates blockmap cells (p_map.c:1321–1334), never updates their `floorz/ceilingz`. Those stay at their spawn-time values.
5. Sound: `S_StartSound` contains no `P_Random`. It makes at most one `M_Random` pitch draw (linuxdoom-1.10/s_sound.c:327–346), and
   only when the sound is audible or has no origin or the console player as origin (304–320). `M_Random` uses `rndindex`, separate
   from gameplay's `prndindex` (linuxdoom-1.10/m_random.c:53–67). The genuine runner stubs `S_StartSound` (tools/reference/m3h9c_stubs.c:85–86).
   Neither fog sound alerts monsters (`P_NoiseAlert` is not called).

### T5. Freeze, z, view, momentum, angle

1. A player's teleport happens inside its `P_MobjThinker` → `P_XYMovement` → `P_TryMove`, during `P_RunThinkers`, which runs **after**
   that tic's `P_PlayerThink` (p_tick.c:148–152). `P_PlayerThink` → `P_MovePlayer` only applies thrust and turning
   (linuxdoom-1.10/p_user.c:148–171), and the use action (`P_UseLines`) cannot teleport (T1.1). So a player never teleports inside
   `P_PlayerThink`.
2. `reactiontime = 18`. The next 18 `P_PlayerThink` calls (tics T+1 … T+18) decrement it **instead of** calling `P_MovePlayer`
   (p_user.c:267–270), so there is **no thrust and no turning** (`angleturn` is applied at p_user.c:154). `P_CalcHeight`, sector
   specials, weapon changes, use, psprites and powers still run (272 onward). `P_MovePlayer` resumes at T+19. A dead player takes the
   `P_DeathThink` branch before this test (258–262). The freeze does not stop motion from other sources: momentum from later damage
   still moves the player through `P_XYMovement`, which can cross another teleport line.
3. Monsters: `EV_Teleport` sets no `reactiontime` (121–122). A monster's own `reactiontime` keeps its value; it is consumed by
   `A_Chase` (p_enemy.c:676–677) and gates `P_CheckMissileRange` (p_enemy.c:212–213). `P_DamageMobj` zeroes it on a non-lethal hit
   (p_inter.c:902).
4. `z = floorz` (T2.7), `momx = momy = momz = 0`, `angle = m->angle` (T2.11). The landing angle is `ANG45 * (mthing->angle/45)`
   (p_mobj.c:794; the declared ANG45 sanitizer adapter is in tools/reference/m3h9c_build.py:105–110). On the same tic, when no split or
   slide follows, `P_MobjThinker` skips `P_ZMovement`, because `z == floorz` and `momz == 0` (p_mobj.c:428–429).

### T6. Interaction with the movement loops

1. **Crossing loop** (p_map.c:500–514). `spechit` is filled by `PIT_CheckLine` in `P_CheckPosition`'s line pass: `bx` outer, `by`
   inner, blockmap list order, deduplicated by `validcount` (436–439, 240–244). `while (numspechit--)` then visits it **from back to
   front**. Each entry compares `P_PointOnLineSide` of the thing's **current** `x/y` with that of the locals `oldx/oldy` captured at
   490–491.
   - If `EV_Teleport` **reaches `P_TeleportMove`**, success or failure, then `numspechit = 0` (p_map.c:152). The next `numspechit--`
     reads 0, the loop ends and `numspechit` becomes −1. **The remaining entries are never examined.** Example: a blocked monster
     teleport ends the loop too, so a lift line (88) crossed in the same move but visited later is not triggered, even though the monster
     did move.
   - If `EV_Teleport` returns **before** `P_TeleportMove` (missile, `side == 1`, no landing found), `numspechit` is untouched and the
     loop goes on with the unchanged position.
   - `numspechit == −1` is also the normal exit value of every completed crossing loop and of `P_Move`'s loop (p_enemy.c:315), so it is
     not teleport-specific. It is reset by the next `P_CheckPosition` (p_map.c:410).
   - `P_TryMove` returns true after the loop (516).
2. **Split move** (`P_XYMovement`, p_mobj.c:147–191). Locals `xmove/ymove` are clamped copies of the momentum. When
   `xmove > MAXMOVE/2 || ymove > MAXMOVE/2`, a test made only in the positive direction (152), the first try goes to `mo->x + xmove/2`
   and the locals are halved **before** the call (154–157). If that try teleports, the `do … while (xmove || ymove)` loop **continues**
   and tries `mo->x + xmove` from the **landing** with the remaining half (161–163; that half is at most 15 units, so it is not split
   again), even though `momx/momy` are now 0. That second try can itself cross another special, a teleport included, near the
   landing. Friction afterwards sees zero momentum: `momx = momy = 0`, and a player in `S_PLAY_RUN1..4` is set to `S_PLAY` only if the
   cmd has no forward or side move (221–235). No draw.
3. **Slide move** (`P_SlideMove`, p_map.c:695–788). If the teleport happens in the `P_TryMove` at 762, the function goes on:
   `bestslidefrac` remainder (768–774), then `tmxmove = FixedMul (mo->momx, …) = 0` (776–777), `P_HitSlideLine` (779),
   `momx/momy = 0` (781–782), and `P_TryMove (mo, mo->x, mo->y)` **at the landing** (784). That zero-length try runs `P_CheckPosition`
   there, which can pick up items overlapping the landing in the same tic (`PIT_CheckThing` → `P_TouchSpecialThing`, 331–338) and
   crosses nothing (same side). If the teleport happens in the stairstep `P_TryMove` at 750, it returns true and the try at 751 is
   skipped.
4. **Monster walk** (`P_Move`, p_enemy.c:293–334). After a teleporting `P_TryMove`, `try_ok` is true, so the code clears `MF_INFLOAT`
   and sets `z = floorz` for a non-floater (328–333). Then `P_TryWalk` draws `movecount = P_Random()&15` (p_enemy.c:356); that draw is
   part of every successful walk and is not caused by the teleport. `movedir` is unchanged.

### T7. `MT_TELEPORTMAN` (doomednum 14)

1. mobjinfo (info.c:2174–2198): doomednum 14, **spawnstate `S_NULL`** (state 0: `SPR_TROO`, tics −1, no action, info.c:136),
   reactiontime 8, radius 20, height 16, mass 100, flags **`MF_NOBLOCKMAP|MF_NOSECTOR`**. It has neither `MF_COUNTKILL` nor
   `MF_COUNTITEM`, and it is not shootable or solid.
2. Spawn: `P_SpawnMapThing` applies the usual skill-bit and `MTF_NOTSINGLE` (options & 16) filters (p_mobj.c:740–751), so a landing
   missing from the admitted skill is never spawned. Then `P_SpawnMobj` (784): one **`lastlook` draw** (506). `S_NULL` is assigned
   directly, so the landing is **not** removed. `P_SetThingPosition` sets `subsector` but links it into **no** sector list and **no**
   block cell (p_maputl.c:406–407, 409, 425). There is **no** spawn-tics draw because tics = −1 (p_mobj.c:787). Its angle is
   `ANG45 * (angle/45)` (794).
3. Thinker: a normal `P_MobjThinker`. With no momentum and `z == floorz` it does no movement. With tics −1 it takes the nightmare-respawn
   branch, and since it is not `MF_COUNTKILL` it returns there (p_mobj.c:418–471). Its stored `z/floorz` are never refreshed (it is not
   in the blockmap, so `P_ChangeSector` misses it), but `EV_Teleport` reads only `x`, `y`, `angle`, `type` and `subsector`.
   Teleportation uses the landing sector's live floor (T3.1).
4. It adds no hitscan or slide intercepts and blocks nothing (`MF_NOBLOCKMAP`). It does occupy a thinker slot, which gives it a place in
   actor order.
5. M4a deferred all of teleportation (docs/M4A_REFERENCE_INVESTIGATION.md §10 part-1 scope, line ~1702, "everything above except
   teleportation, the Baron and `A_BossDeath`"). E1M5, E1M8 and E1M9 each have one landing (M4a note §2.5; this note §1.6). The Rust registry already carries the
   `MT_TELEPORTMAN` metadata row (crates/doom-sim/src/source_actors.rs:1347).

### T8. Telefrag damage and the thrust overflow (`P_DamageMobj`, linuxdoom-1.10/p_inter.c:774–917)

1. Call: `P_DamageMobj (victim, tmthing, tmthing, 10000)`. Inflictor and source are both the player-linked mover. During
   `PIT_StompThing` the mover's `x/y` are still the **pre-teleport** coordinates (assigned only at p_map.c:171–172).
2. Early returns: not shootable (787–788), `health <= 0` (790–791). `MF_SKULLFLY` zeroes momentum (793–796). Baby skill halves damage
   only for a player victim (798–800).
3. **The thrust block** runs when `inflictor && !(target->flags & MF_NOCLIP) && (!source || !source->player ||
   source->player->readyweapon != wp_chainsaw)` (806–810). For a telefrag the inflictor is never null and no Episode 1 victim has
   `MF_NOCLIP`. So the block runs **unless the stomping player's `readyweapon` is `wp_chainsaw`**. `pendingweapon` does not matter.
4. `ang = R_PointToAngle2 (inflictor->x, inflictor->y, target->x, target->y)` (812–815), the angle from the player's **old** position
   to the victim. `R_PointToAngle2` also writes the renderer globals `viewx/viewy` (r_main.c:378–387), which is presentation state only.
5. **`thrust = damage*(FRACUNIT>>3)*100/target->info->mass`** (817). `FRACUNIT` is `(1<<16)`, an `int`, so the whole expression is
   `int` arithmetic: `((10000 * 8192) * 100) / mass`. `10000 * 8192 = 81,920,000` fits; `× 100 = 8,192,000,000` exceeds `INT_MAX`
   (2,147,483,647). The overflow starts at damage ≥ 2622 (`2622 × 819200 > 2³¹ − 1`). The overflow is **undefined behaviour** in C.
   - **LP64 is no different**: `int` stays 32 bits on x86-64 Linux (*observed* `sizeof(int) = 4`), so the same multiplication
     overflows on LP64 and ILP32 alike.
   - With wrapping (two's complement mod 2³²): `8,192,000,000 − 2·2³² = −397,934,592`. The `int` division then truncates toward zero
     (GCC's choice; it is implementation-defined in C89, and the runner uses `-std=gnu89`). *Observed* thrust: **mass 100 →
     −3,979,345** (−60.72 units; `MT_POSSESSED`, `MT_SHOTGUY`, `MT_TROOP`, `MT_BARREL`, and `MT_PLAYER` for a doll), **mass 400 →
     −994,836** (−15.18; `MT_SERGEANT`, `MT_SHADOWS`), **mass 1000 → −397,934** (−6.07; `MT_BRUISER`). Masses: info.c:1108–1530 and
     1888–1912 (MT_PLAYER 100, POSSESSED 100, SHOTGUY 100, TROOP 100, SERGEANT 400, SHADOWS 400, BRUISER 1000, BARREL 100).
   - `damage < 40` is false, so `&&` short-circuits **before** the fall-forward `P_Random` (820–823). No draw.
   - `momx/momy += FixedMul (thrust, finecosine/finesine[ang>>19])` (829–831). `FixedMul` uses `long long` (linuxdoom-1.10/m_fixed.c:44–49),
     which cannot overflow here. Because the thrust is negative, the victim gets momentum of up to 60.72 units **toward the player's
     pre-teleport position**. Examples (*observed*): mass 100, angle 0 → `momx −3,979,285`; 45° → about −2.81M on each axis. The
     victim's next `P_XYMovement` clamps each axis to ±`MAXMOVE` = ±1,966,080 (p_mobj.c:137–145).
6. Player-victim branch, reached only for a player-linked victim (T3.9), in order:
   - the sector-11 hack: `damage = health − 1` when the victim stands in a special-11 sector (838–842). E1M8 has one such sector, so a
     stomped doll or player there **survives with 1 health**.
   - god mode and invulnerability are **ignored** because `damage ≥ 1000` (847–852).
   - armor: type 1 saves `damage/3` (3333), type 2 saves `damage/2` (5000), capped at `armorpoints`, which zeroes `armortype`
     (854–869).
   - `player->health` is clamped at 0 (870–872); `attacker = source`; `damagecount` is capped at 100 (874–878); `I_Tactile` is called
     (880–883).
7. `target->health -= damage` (887). Every Episode 1 shootable dies: the largest `spawnhealth` is the Baron's 1000 (info.c:1501).
   `P_KillMobj (source, target)` then returns (888–891), so **the pain draw at 894 is never reached** and neither is the
   target/threshold update at 904–915. A stomped barrel's `target` is therefore not set by the telefrag. `P_KillMobj`
   (p_inter.c:667–758):
   - it clears `SHOOTABLE|FLOAT|SKULLFLY` and `NOGRAVITY` (except for the skull), sets `CORPSE|DROPOFF` and quarters `height`
     (675–681). **`MF_SOLID` stays** until `A_Fall` (p_enemy.c:1585–1591), except for players (705).
   - `killcount++` for `MF_COUNTKILL`, because the source is a player (683–691).
   - **xdeath vs death**: `health < −spawnhealth && xdeathstate` (719–725). That gives xdeath (gibs) for `MT_POSSESSED` (20),
     `MT_SHOTGUY` (30) and `MT_TROOP` (60); death for `MT_SERGEANT`, `MT_SHADOWS` and `MT_BRUISER`, whose `xdeathstate` is `S_NULL`;
     and `S_BEXP` for the barrel.
   - then **`tics -= P_Random()&3`, floored at 1** (726–729): one gameplay draw per victim.
   - then a drop: `MT_CLIP` for `MT_POSSESSED` and `MT_SHOTGUN` for `MT_SHOTGUY` (737–757), each a `P_SpawnMobj` with one
     **`lastlook` draw**, flagged `MF_DROPPED`.
   - later tics: `A_XScream` (no draw) for the gibbed types; `A_Scream` for the Demon, Spectre, Baron and barrel, whose death sounds
     `sfx_sgtdth`, `sfx_brsdth` and `sfx_barexp` take the `default` branch with no draw (p_enemy.c:1535–1569; info.c:1435, 1461, 1513,
     1903). The Baron's `A_BossDeath` runs at `S_BOSS_DIE7` as with any Baron death (info.c:684).
8. What the corpse does next depends on geometry. The corpse is still `MF_SOLID` and overlaps the relinked player. If its next
   `P_TryMove` target still overlaps the player, `PIT_CheckThing` blocks it (p_map.c:258–268, 342) and `P_XYMovement` zeroes its
   momentum (p_mobj.c:188–189). Otherwise it slides. Whether it moves this tic or next depends on whether its thinker comes after the
   player's.
9. **The four genuine runner variants at p_inter.c:817** (flags at tools/reference/m3h9c_build.py:53–69, reused unchanged by
   tools/reference/m4a_build.py:24–50):
   - `strict` (`-O0 -fwrapv`, LP64, five ABI adapters): the multiplication **wraps by definition**, giving thrust −397,934,592/mass. It
     runs on.
   - `strict32` (`-fwrapv -m32`, no adapters; opt-in): the same wrapped value (*observed* in the probe: −3,979,345 / −994,836 /
     −397,934). `-m32` builds and runs on this host.
   - `overflow` (`-fno-wrapv -fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow -fno-sanitize-recover=all`):
     **aborts**, with `runtime error: signed integer overflow: 10000 * 819200 cannot be represented in type 'int'` (*observed* on the
     identical statement; GCC folds the two constants even at `-O0`), exit 1.
   - `address` (`-fsanitize=address,undefined -fno-sanitize-recover=all`): **aborts** the same way (*observed*), because `undefined`
     includes signed-integer-overflow.
   - The 1993 binaries were built with `gcc -g -Wall` (linuxdoom-1.10/Makefile:7–9) for i386. What they did cannot be observed from this
     repository. A 32-bit `imul` wraps, which is consistent with the strict and strict32 values, but that is an inference, not evidence.
10. How declared adapters work: `SANITIZER_ADAPTERS` are exact one-line text replacements applied only to the `overflow` and `address`
    builds. The build fails unless the original text occurs exactly once at its declared line (m3h9c_build.py:168–172, 180–182), and the
    adapted line computes the wrapped value in `unsigned`. The precedent is p_map.c:1221 (`P_RadiusAttack`) and the ANG45 lines
    (m3h9c_build.py:104–113). A matching adapter here would be:
    original `\tthrust = damage*(FRACUNIT>>3)*100/target->info->mass;`, adapted
    `\tthrust = (int)((unsigned)damage*(FRACUNIT>>3)*100u)/target->info->mass;`. The unsigned product wraps by definition. The explicit
    `(int)` conversion is implementation-defined (GCC: modulo) and is not flagged by `-fsanitize=undefined`. The division stays signed.
    *Observed* under `-fsanitize=address,undefined`: no report, the values match strict exactly (−3,979,345 / −994,836 / −397,934), and
    non-overflowing damages are unchanged (damage 40 → 327,680 at mass 100).
11. Rust today: the shared thrust uses `checked_mul`/`checked_div` and returns `CombatError::Metadata` on overflow
    (crates/doom-sim/src/original_player.rs:705–709). A telefrag reaching that service under the current policies therefore errors; it
    does not wrap.

### T9. Overflow hazards beyond the thrust

1. **The ninth `spechit`.** `line_t* spechit[MAXSPECIALCROSS]` with `MAXSPECIALCROSS = 8` and `int numspechit` (p_map.c:67–70). It is
   written as `spechit[numspechit] = ld; numspechit++` **with no bound check** (240–244). The write happens only after the line passed
   the bbox, box-side, one-sided, `ML_BLOCKING` and `ML_BLOCKMONSTERS` tests and after `P_LineOpening`'s adjustments (191–237). It can
   happen on any `P_CheckPosition` caller: `P_TryMove` (463, from `P_XYMovement`, `P_Move`, `P_SlideMove` and `P_CheckMissileSpawn`
   p_mobj.c:880), `P_ThingHeightClip` (536, from `P_ChangeSector`), `G_CheckSpot` (linuxdoom-1.10/g_game.c:868) and others.
   - What follows the array in memory **cannot be known from the source**. The declaration order (`numspechit` follows `spechit`
     textually, p_map.c:46–70) does not fix the layout: these are uninitialised file-scope definitions that the compiler and linker place
     (in `.bss` or as common symbols, in an order of their choosing), and on LP64 each element is 8 bytes, so a ninth entry covers 8
     bytes past the array, versus 4 on the 32-bit original.
   - Expected but **not observed** here: the `address` variant (ASan global redzones, plus `-fsanitize=bounds` inside `undefined`) would
     abort at p_map.c:242; `overflow` would not detect it; `strict` writes silently over unknown data.
   - Community ports emulate a DOS-executable-specific layout; that cannot be verified from this repository and is not evidence for
     linuxdoom.
   - **H3 witness**: refuse the tic, with canonical state preserved, at the exact point where a ninth special line would be appended
     (`numspechit == 8` and the line reaches 240), in every `P_CheckPosition` caller. Wire 23 already has this witness under the Episode 1
     policy, `MapScaleError::SpecialCross` (crates/doom-sim/src/player_map_scale.rs:159–164; emitted at
     crates/doom-sim/src/movement.rs:611–617; refused at crates/doom-sim/src/thinkers.rs:302–306), and drops the load-time 2×2 window rule
     for Episode 1 (crates/doom-sim/src/mover_rules.rs:358–359, 566–569). For M4b, check that it covers every caller listed above, the
     height-clip path included. That coverage was not verified.
2. **`intercepts[MAXINTERCEPTS]`** (128; linuxdoom-1.10/p_local.h:154–156; p_maputl.c:544). `P_PathTraverse` resets `intercept_p` per
   trace (p_maputl.c:775). For each block on the trace it calls `P_BlockLinesIterator(PIT_AddLineIntercepts)` and then, with
   `PT_ADDTHINGS`, `P_BlockThingsIterator(PIT_AddThingIntercepts)` (850–857). Both append with **no bound check** (603–606, 668–671);
   only `frac < 0` (591, 665) and the early-out (595–600) skip the write.
   - Things count: every blockmap-linked thing whose corner diagonal crosses the trace, **not only shootables**. That includes items,
     solid decorations and corpses. `MF_NOBLOCKMAP` actors (fogs, landings) are never added.
   - Slides and use traces add lines only (p_map.c:738–743, 1147). Hitscans (`P_LineAttack`/`P_AimLineAttack`, 1045, 1082) add lines
     and things.
   - Wire 23 has a per-trace witness that counts lines and things alike and refuses at the 129th
     (crates/doom-sim/src/combat_trace.rs:350–356 → `CombatError::InterceptLimit`, mapped to `MapScaleError::Intercepts`,
     crates/doom-sim/src/demon_expanded_map.rs:712–719). There is also a load-time slide-window rule (lines only;
     crates/doom-sim/src/player_map_scale.rs:1217–1222).
   - More monsters, items and decorations on E1M5, E1M8 and E1M9 raise the thing count along long hitscans (range up to
     `MISSILERANGE` = 2048 units, p_local.h:58), so the risk grows per trace. Reaching 128 still needs a very dense line of things. The
     existing witness refuses rather than diverges, so nothing new is required beyond keeping it active under wire 24.
3. **Teleport scan cost** (T2.4): no array to overflow, but unbounded work per crossing. It needs a budget sized from the map: tagged
   sectors × the thinker bound.
4. **Fog spawns** add 2 actors per successful teleport, each live for 72 thinks, plus drops from stomp kills. One thing can teleport
   **more than once per tic**: a split move or a slide can cross a second teleport line from the landing (T6.2–3). The actor-store
   transient allowance and the store-full witness (H3/H4) must count this.

### T10. Every draw on the teleport paths, in call order

1. Crossing, search, side and missile tests, `P_TeleportMove`'s setup and a **failed** teleport (monster blocked): **no draws**.
2. A successful **player** teleport:
   (a) for each stomped victim, in `bx`-outer, `by`-inner, `bnext` order: `P_KillMobj`'s `tics` draw (p_inter.c:726), then for
   `MT_POSSESSED`/`MT_SHOTGUY` the drop's `lastlook` draw (p_mobj.c:506 via p_inter.c:756). There is no fall-forward draw (820–823 short-
   circuits) and no pain draw (894 is not reached). A player-linked victim that survives through the sector-11 hack would reach 894
   (`P_Random() < painchance`, 255 for the player); that is not reachable without a doll.
   (b) fog 1 `lastlook` (p_mobj.c:506).
   (c) fog 2 `lastlook`.
   `S_StartSound` makes no gameplay draws (T4.5).
3. A successful **monster, corpse or barrel** teleport: (b) and (c) only. In the same `P_Move`, the usual `movecount` draw follows
   (p_enemy.c:356).
4. Later tics: `A_XScream` and the Episode 1 `A_Scream` death sounds do not draw (T8.7). A stomped barrel's later `A_Explode` chain
   follows the M3h9b barrel rules.

### Hazards and witnesses (teleportation)

- **Telefrag thrust overflow** (p_inter.c:817): signed overflow for damage ≥ 2622 on LP64 and ILP32 alike. The strict builds wrap to
  a defined value; the overflow and address builds abort. A decision is needed (see Recommendation).
- **Ninth `spechit`** (p_map.c:242): unchecked write; memory layout unknowable. H3 witness `SpecialCross` at the exact append, in every
  `P_CheckPosition` caller (exists in wire 23; keep it and verify caller coverage).
- **129th intercept** (p_maputl.c:603–606, 668–671): unchecked. The existing per-trace witness counts lines and things; denser E1M5,
  E1M8 and E1M9 raise the per-trace risk but need no new mechanism.
- **Teleport search budget** (p_telept.c:75–128): unbounded sectors × thinkers per crossing. Preflight it within a declared work
  budget and refuse the tic over budget.
- **Store pressure**: 2 fogs per teleport, possibly several teleports per thing per tic. Covered by the store-full witness, with the
  allowance re-derived.
- **Defined-but-surprising states a port must accept, not "fix"**: a player overlapping solid corpses after a telefrag; a teleported
  thing overlapping solid non-shootable decorations; `floorz/ceilingz` from the centre sector only; `z = ceilingz − height < floorz` in
  a short landing sector; stale fog and landing `floorz/ceilingz`; a split or slide move continuing from the landing. If Rust's existing
  Position or Vertical invariants would reject any of these, each must be either modelled exactly or turned into an explicit
  refuse-the-tic witness, never silently adjusted.
- **Crossing-loop termination** must follow T6.1 exactly: drop the remaining crossings whenever `P_TeleportMove` was reached,
  successful or not, and keep them when `EV_Teleport` returned earlier.
- **Tag 0**: `EV_Teleport` has no tag-0 guard. Either model it or add a load rule.
- **Voodoo dolls**: only with duplicate player-1 starts (census question). In that case a telefrag can kill the player through a doll,
  or leave it at 1 health in a sector-11 sector. Refuse at load unless modelled.

### Recommendation on the telefrag overflow

Use a **declared wrapping adapter in the genuine overflow and address builds plus a matching Rust wrapping rule for wire 24 only**:
`thrust = ((damage as u32).wrapping_mul(8192).wrapping_mul(100) as i32) / mass`, with Rust's truncating `/`. Reasons:

1. The `strict` (`-fwrapv`) and `strict32` builds already define and agree on the value. It is the two's-complement result on every
   target observed, and it equals what a 32-bit `imul` gives.
2. The adapter follows the established mechanism and precedent (p_map.c:1221, the ANG45 lines). The probe showed it reproduces the
   strict values exactly and is clean under `-fsanitize=address,undefined`.
3. The M4a note already assigned "the telefrag overflow adapter" to M4b (docs/M4A_REFERENCE_INVESTIGATION.md §3.3.6, §3.13 and the
   §10 closing paragraph), so this is consistent with prior owner-confirmed text.
4. A refuse-the-tic witness would make every player telefrag (any non-chainsaw stomp of a shootable) unmatchable. The overflow and
   address builds would abort at the same tic anyway, so they could not confirm the rest of such a run.

Caveat for the owner: H3's text names checked arithmetic that refuses the tic on overflow. Wrapping here is a deliberate exception,
justified because the wrapped value is the one the strict genuine builds define. It needs explicit owner confirmation as a new M4b
decision, and it must apply only to the new wire. Older wires keep today's `CombatError::Metadata` refusal (T8.11). The fallback, if
the owner prefers H3 literally, is a `MapScaleError::Arithmetic`-style witness at the thrust, before any stomp mutation is
committed. In that case scripts must avoid telefrags.

### Corrections to the M4a note (teleportation)

Each item cites the section of docs/M4A_REFERENCE_INVESTIGATION.md that it corrects.

1. §3.3.2, "Barrels, monsters and corpses-in-pain (any shootable) are stomped": corpses are **not** shootable (`P_KillMobj` clears
   `MF_SHOOTABLE`, p_inter.c:675), so they are never stomped and never block a monster teleport. "Corpses-in-pain" has no meaning in the
   source. The correct set is live shootables: monsters, barrels and player-linked mobjs.
2. §3.3.3, "The crossing loop stops after a teleport": the loop stops whenever `P_TeleportMove` is **called**, including a failed
   (blocked) monster teleport, because `numspechit = 0` is set before the stomp loop (p_map.c:152). It does **not** stop when
   `EV_Teleport` returns before that (missile, back side, no landing).
3. §3.14 item 5 / §3.3.3, "`numspechit` after a teleport is −1": −1 is the normal exit value of **every** completed crossing loop
   (p_map.c:502) and of `P_Move`'s loop (p_enemy.c:315). It is not teleport-specific.
4. §3.3.6, "a large thrust **toward** the stomping player": the thrust points toward the player's **pre-teleport** position, because the
   inflictor's `x/y` are updated only after the stomp loop (p_map.c:160–172). "The corpse then slides" is geometry-dependent: the
   corpse keeps `MF_SOLID` until `A_Fall` and its first move can be blocked by the overlapping player, which zeroes its momentum (T8.8).
   Missing facts: gibbing (xdeath) for zombieman, shotgun guy and imp; the barrel's `target` is not set; magnitudes per mass (T8.5).
5. §3.3.6, "`S_StartSound` draws `M_Random` only": at most one `M_Random`, and only when audible (s_sound.c:304–346). The runner stubs it.
6. §3.3.3: also record that `P_SlideMove` continues after a teleport inside its `P_TryMove` with a zero-length try at the landing
   (p_map.c:762–787), which can pick up items there, and that the split move's second half can cross another special from the landing
   (T6.2–3).
7. §3.3.1 item 6: fog 2's offset is not exactly 20 units, because `finesine[0] = 25` (T2.9). "No check" is correct.
8. §3.13, `spechit` row, "overflow writes past the array into `numspechit` and following globals": declaration order does not
   determine memory layout, so what is overwritten cannot be known from the source (T9.1). The row's "a per-call witness … is needed" is
   already implemented in wire 23 (`MapScaleError::SpecialCross`) under the Episode 1 policy.
9. §3.3.5: add the skill and `MTF_NOTSINGLE` filtering of landings (p_mobj.c:740–751), and that their `z/floorz` go stale but are never
   read by `EV_Teleport`.
10. §3.3.1 item 3: add that `EV_Teleport` has no tag-0 guard, and that sector `thinglist`s are never read by compiled linuxdoom gameplay
    code.

Not run or not verified: no cargo build or test was run. The spechit ASan/bounds behaviour is expected, not observed. Whether wire 23's
`SpecialCross` covers the `P_ThingHeightClip` caller was not checked. No IWAD census was taken: duplicate player starts, tag-0
teleport lines and landing skill bits are open questions.

## 3. C facts: the Baron, A_BossDeath, hard skill and demo version

Source: linuxdoom-1.10 in this repository (id Software, commit a77dfb96). Paths are relative to the repository root; `ld/` abbreviates `linuxdoom-1.10/`. Investigation date 2026-11-10 (America/Los_Angeles). Read-only; nothing was built or run except Python byte counts over a read-only IWAD (counts and header bytes only, quoted below). Facts are numbered 1–36 across parts A–E; "fact N" refers to that numbering. Section numbers of the form §3.4.x in the corrections list are those of the M4a note.

### A. The Baron of Hell (`MT_BRUISER`)

1. **mobjinfo** (ld/info.c:1498–1522): doomednum 3003; spawnstate `S_BOSS_STND`; spawnhealth 1000; seestate
   `S_BOSS_RUN1`; seesound `sfx_brssit`; reactiontime 8; attacksound 0; painstate `S_BOSS_PAIN`; painchance 50;
   painsound `sfx_dmpain`; meleestate `S_BOSS_ATK1`; missilestate `S_BOSS_ATK1` (the same state); deathstate
   `S_BOSS_DIE1`; xdeathstate `S_NULL`; deathsound `sfx_brsdth`; speed 8; radius 24·FRACUNIT; height 64·FRACUNIT;
   mass 1000; damage 0; activesound `sfx_dmact`; flags `MF_SOLID|MF_SHOOTABLE|MF_COUNTKILL`; raisestate
   `S_BOSS_RAISE1`. The Rust registry row matches (crates/doom-sim/src/source_actors.rs:1321).
2. **States** (ld/info.c:663–691; tics, action, next):
   `STND` 10 A_Look → `STND2` 10 A_Look → `STND` (663–664);
   `RUN1`…`RUN8` 3 A_Chase each, `RUN8` → `RUN1` (665–672);
   `ATK1` 8 A_FaceTarget → `ATK2` 8 A_FaceTarget → `ATK3` 8 A_BruisAttack → `RUN1` (673–675);
   `PAIN` 2 (none) → `PAIN2` 2 A_Pain → `RUN1` (676–677);
   `DIE1` 8 (none) → `DIE2` 8 A_Scream → `DIE3` 8 → `DIE4` 8 A_Fall → `DIE5` 8 → `DIE6` 8 → `DIE7` −1 A_BossDeath →
   `S_NULL` (678–684);
   `RAISE1`…`RAISE7` 8 tics each, no actions, → `RUN1` (685–691; reached only through the arch-vile, absent).
   Every action runs on state entry (`P_SetMobjState`, ld/p_mobj.c:76–78), so an A_Chase decision to attack runs
   `ATK1`'s A_FaceTarget in the same call.
3. **Compared with the Imp** (`MT_TROOP`, ld/info.c:1394–1418; states 580–600): the same shape (melee = missile =
   `ATK1`, two A_FaceTarget states), but the Baron's `ATK3` lasts 8 tics, not 6 (675 vs 590); **`A_BruisAttack` does
   not call `A_FaceTarget`** whereas `A_TroopAttack` does (ld/p_enemy.c:920), so a Baron attack makes two
   `A_FaceTarget` calls, the Imp's three; no xdeath (always `DIE1`…`DIE7`); `DIE3`–`DIE6` are 8 tics (the Imp's
   `DIE3`/`DIE4` are 6, `DIE5` is final); painchance 50 vs 200; health 1000 vs 60; radius 24 vs 20; height 64 vs 56;
   mass 1000 vs 100; see/death sounds take the no-draw default branch (the Imp's `bgsit`/`bgdth` draw, ld/p_enemy.c:643–645,
   1550–1552).
4. **`A_BruisAttack`** (ld/p_enemy.c:979–996): `if (!actor->target) return` (983–984); `if (P_CheckMeleeRange
   (actor))` (986): `S_StartSound (actor, sfx_claw)` (988), **`damage = (P_Random()%8+1)*10`** (989; one draw;
   10–80), `P_DamageMobj (actor->target, actor, actor, damage)` (990; inflictor = source = the Baron), return; else
   `P_SpawnMissile (actor, actor->target, MT_BRUISERSHOT)` (995). The melee/missile choice is re-made here, 16 tics
   after A_Chase chose the attack: a "missile" decision can end in a claw and a "melee" decision in a fireball.
5. **`P_CheckMeleeRange`** (ld/p_enemy.c:174–191): false without a target; `dist = P_AproxDistance (target − actor)`;
   **false if `dist >= MELEERANGE − 20·FRACUNIT + target->info->radius`** (185; `MELEERANGE` = 64·FRACUNIT,
   ld/p_local.h:57: under 60 units against a player, radius 16; under 44 + r against a monster of radius r); then
   `P_CheckSight`. No draw.
6. **`MT_BRUISERSHOT`** (ld/info.c:1524–1548): doomednum −1; spawnstate `S_BRBALL1`; spawnhealth 1000; seesound
   `sfx_firsht`; reactiontime 8; painchance 0; deathstate `S_BRBALLX1`; deathsound `sfx_firxpl`; **speed 15·FRACUNIT**;
   radius 6·FRACUNIT; height 8·FRACUNIT; mass 100; **damage 8**; flags `MF_NOBLOCKMAP|MF_MISSILE|MF_DROPOFF|
   MF_NOGRAVITY`; all other states/sounds null. States (ld/info.c:658–662): `BRBALL1` 4 → `BRBALL2` 4 → `BRBALL1`
   (no actions); `BRBALLX1` 6 → `X2` 6 → `X3` 6 → `S_NULL`; **no `A_Explode`** (no splash). The Imp's `MT_TROOPSHOT`
   (ld/info.c:1914–1938, states 233–237) is identical except **speed 10 and damage 3**. The Rust registry row matches
   (source_actors.rs:1322). `-fast`/nightmare would set the speed to 20 (ld/g_game.c:1421–1427); not reachable on
   skills 2–3 with `fastparm` 0 (see part C below).
7. **`P_SpawnMissile`** (ld/p_mobj.c:889–927), as for the Imp ball: `P_SpawnMobj` at `source->z + 32·FRACUNIT`
   (one `lastlook` draw, ld/p_mobj.c:506; `reactiontime` from info except on nightmare, 503–504); seesound;
   `th->target = source`; `an = R_PointToAngle2 (source, dest)`; **if `dest->flags & MF_SHADOW`: `an +=
   (P_Random()−P_Random())<<20`** (909–910); `momx/momy = FixedMul (speed, finecosine/finesine[an])`; `dist =
   P_AproxDistance (dest − source) / speed`, at least 1; `momz = (dest->z − source->z) / dist` (917–923);
   `P_CheckMissileSpawn`. **What differs from the Imp ball is only the info values**: momentum components are
   `FixedMul(15·FRACUNIT, ·)`, the time divisor uses speed 15, the half-step in `P_CheckMissileSpawn` is up to 7.5
   units, and contact damage is `((P_Random()%8)+1)*8` = 8–64 (ld/p_map.c:323) instead of 3–24.
8. **`P_CheckMissileSpawn`** (ld/p_mobj.c:868–881): `tics -= P_Random()&3` (870), at least 1; moves x/y/z by half the
   momentum; `if (!P_TryMove (th, x, y)) P_ExplodeMissile (th)` (880–881). `P_ExplodeMissile` (ld/p_mobj.c:90–104):
   zero momentum, death state, `tics -= P_Random()&3` (96), at least 1, clear `MF_MISSILE`, death sound.
   A contact with a shootable thing during that `P_TryMove` draws the damage (ld/p_map.c:323), calls `P_DamageMobj`
   and returns false (327), **so `P_TryMove` fails and `P_ExplodeMissile` draws as well**.
9. **No move split**: `P_XYMovement` splits a step only when `xmove > MAXMOVE/2 || ymove > MAXMOVE/2`
   (ld/p_mobj.c:152; `MAXMOVE` = 30·FRACUNIT, ld/p_local.h:54; positive values only). The largest `finesine`
   entry is 65535 (ld/tables.c:584 onward; checked by parsing all 10,240 entries), so
   `FixedMul(15·FRACUNIT, 65535)` = 983025 < 983040 = `MAXMOVE/2`: **a Baron ball never splits** (strictly below,
   not equal). Missiles are filtered from line crossings (`MT_BRUISERSHOT` at ld/p_spec.c:513).
   **Correction (2026-11-14, owner ruling 2026-11-13; finding 29).** Facts 6 to 9 describe the move as if the
   M3h5 step bound covered it. It does not. [M3h5 Theorem B](M3H5_REFERENCE_INVESTIGATION.md) proves that a
   `P_TryMove`-accepted step stays inside the walkable region only for steps of at most 2r − 2 units per axis;
   for `MT_BRUISERSHOT` (radius 6) that is 10, and its step is up to 15 (speed 15, `FixedMul(15·FRACUNIT, ·)`,
   never split, as fact 9 shows). `P_CheckPosition` tests only the destination box (p_map.c:191–195,
   374–442), so a ball can step wholly past a one-sided wall thinner than the step and be accepted. The text of
   facts 6 to 9 is kept as written; this note supersedes the unstated assumption that the step bound holds for
   the ball. (Wire 23's missiles all meet the bound, so wire 23 cannot reach the case.)
10. **`A_Chase` for a monster with both melee and missile states** (ld/p_enemy.c:672–776), in order:
    `reactiontime--` if nonzero (677–678); threshold: cleared if the target is missing or dead, else decremented
    (682–690); turn toward `movedir` (693–702); no shootable target → `P_LookForPlayers (actor, true)` or
    `P_SetMobjState (spawnstate)` (704–712); **`MF_JUSTATTACKED` → clear it, `P_NewChaseDir` (skill ≠ nightmare and
    !fastparm), return** (716–722); **melee: `meleestate && P_CheckMeleeRange` → attacksound (0 for the Baron),
    `P_SetMobjState (meleestate)`, return — no `movecount` test and no `MF_JUSTATTACKED`** (725–733); **missile:
    skipped while `movecount != 0` (below nightmare, !fastparm)** (738–742), then `P_CheckMissileRange`, then
    `P_SetMobjState (missilestate)` **and `flags |= MF_JUSTATTACKED`** (744–749); `nomissile`: the netgame retarget
    (753–759, not single player); `if (--movecount < 0 || !P_Move) P_NewChaseDir` (762–766); **active sound
    `P_Random () < 3`** (770–775; the Baron's `dmact` is nonzero, so every fall-through call draws).
    For the Baron both branches enter `S_BOSS_ATK1`; the difference is that only the missile branch needs
    `movecount == 0` and sets `MF_JUSTATTACKED`, which makes the A_Chase on re-entry to `RUN1` (after `ATK3`) do
    only `P_NewChaseDir`. A melee-branch attack is followed by a normal A_Chase that may attack again at once.
11. **`P_CheckMissileRange`** (ld/p_enemy.c:197–256): `P_CheckSight` (201–202); `MF_JUSTHIT` → clear, true (204–210;
    no draw); `reactiontime` → false (212–213); `dist = P_AproxDistance − 64·FRACUNIT` (216–217); **no −128** because
    `meleestate` is set (219–220); `dist >>= 16` (222); type cases for vile/undead/cyborg/spider/skull only (224–244);
    cap 200 (246–247); **`if (P_Random () < dist) return false`** (252–253). `MF_JUSTHIT` (set by pain, ld/p_inter.c:897)
    survives any A_Chase that takes the melee branch or skips the missile branch on `movecount`.
12. **Pain** (ld/p_inter.c:894–900): after a non-lethal hit, `P_Random () < painchance` (50) and not skull-flying →
    `MF_JUSTHIT`, `P_SetMobjState (S_BOSS_PAIN)`. The draw is made on every non-lethal hit. `reactiontime = 0` (902).
    Then `S_BOSS_PAIN` 2 → `PAIN2` A_Pain (sound only, ld/p_enemy.c:1577–1581) 2 → `RUN1` (A_Chase on entry).
13. **Retarget / threshold** (ld/p_inter.c:904–915): `if ((!target->threshold || target->type == MT_VILE) && source &&
    source != target && source->type != MT_VILE)`: `target->target = source`, `threshold = BASETHRESHOLD` (100,
    ld/p_local.h:61), and spawnstate → seestate. This runs only for a surviving target (a kill returns at 888–892).
    `A_Look` zeroes the threshold (ld/p_enemy.c:608); `A_Chase` counts it down (682–690).
14. **Mass** 1000: thrust `damage*(FRACUNIT>>3)*100/mass` (ld/p_inter.c:817) is a tenth of a 100-mass monster's.
    The fall-forward test (820–827) needs `damage > target->health`, so it can draw on a Baron only below 40 health.
15. **Same-species missile rule** (ld/p_map.c:299–313): if the missile's owner has the victim's type, **or the
    owner/victim pair is `MT_KNIGHT`/`MT_BRUISER` either way** (301–302): the owner itself is passed (305–306); any
    other non-player victim stops the missile **without damage** (308–312, `return false` → `P_ExplodeMissile`, one
    tics draw, no damage draw). With no Hell Knights (doomednum 69, ld/info.c:1550–1551) the extra clause only
    repeats "Baron ball vs Baron". Census (DOOM.WAD THINGS, counts only): type 69 occurs 0 times in E1M1–E1M9;
    type 3003 occurs only on E1M8, 2 on medium and 2 on hard (single-player filter). The Rust movement code already
    includes the Knight/Baron clause (crates/doom-sim/src/projectiles.rs:413–415).
16. **Barons can still fight Barons**: a Baron ball never damages a Baron, but splash damage carries a source.
    `PIT_RadiusAttack` calls `P_DamageMobj (thing, bombspot, bombsource, …)` (ld/p_map.c:1194) and exempts only the
    cyborg and spider (1175–1177); a barrel's `A_Explode` passes `thingy->target` (ld/p_enemy.c:1598–1601), which
    is the last **non-lethal** damager's source (ld/p_inter.c:904–911 is not reached on the killing hit). So a
    barrel first hurt (not killed) by a Baron ball (8 or 16 damage against 20 health, ld/info.c:1891) and later
    exploded makes the other Baron, if it survives, target the first (threshold permitting); it then claws it
    (melee damages: no species rule on `P_DamageMobj`) or fires balls that burst harmlessly on it. E1M8 has 28
    barrels on medium and 28 on hard (count only). Whether this happens on a run is not known here.
17. **Spectres on E1M8 hard** (count only): type 58 occurs 10 times on hard and 0 on medium after the single-player
    filter. A Baron targeting a spectre pays the `MF_SHADOW` draws in `A_FaceTarget` (two per call,
    ld/p_enemy.c:794–795) and `P_SpawnMissile` (two, ld/p_mobj.c:909–910).
18. **Death** (`P_KillMobj`, ld/p_inter.c:668–758): clears `MF_SHOOTABLE|MF_FLOAT|MF_SKULLFLY`, clears
    `MF_NOGRAVITY`, sets `MF_CORPSE|MF_DROPOFF`, `height >>= 2` (64 → 16) (675–681); kill count to the source player,
    else `players[0]` when !netgame (683–697); xdeath impossible (no xdeathstate, 719–724); `S_BOSS_DIE1` (no
    action), **`tics -= P_Random()&3`, at least 1** (726–729: `DIE1` lasts 5–8 tics); no drop (the switch at 737–754
    names only `MT_WOLFSS`, `MT_POSSESSED`, `MT_SHOTGUY`, `MT_CHAINGUY`). `A_Scream` at `DIE2`: `sfx_brsdth` is the
    default branch, no draw, positional sound (ld/p_enemy.c:1535–1569). `A_Fall` at `DIE4` clears `MF_SOLID`
    (1585–1592).

### B. `A_BossDeath` and the E1M8 ending

19. **When it runs.** Only on entry to `S_BOSS_DIE7` (ld/info.c:684; tics −1, so it never runs again for that
    corpse). With `t1` = 8 − (draw & 3) ∈ 5…8 and the kill at tic T: if the Baron's thinker runs after the killing
    call in tic T (every kill made inside `P_PlayerThink` — hitscan, fist, chainsaw — since `P_PlayerThink` runs
    before `P_RunThinkers`, ld/p_tick.c:148–152; or a kill by a thinker earlier in the list), the first decrement is
    in tic T (ld/p_mobj.c:441–448), `DIE2` (A_Scream) is entered in tic T+t1−1, `DIE4` (A_Fall) in T+t1+15 and
    **`DIE7` in T+t1+39, i.e. 44–47 tics after the kill tic**. If the kill is made by a thinker after the Baron
    (e.g. a rocket, appended when fired, hit or splash), everything is one tic later: **45–48**.
20. **Checks in order** (ld/p_enemy.c:1609–1756):
    (a) `gamemode == commercial` (1616–1624): only map 7, only `MT_FATSO`/`MT_BABY`, else return.
    (b) Otherwise `switch (gameepisode)` (1627–1680): **E1: `gamemap == 8` and `MT_BRUISER`** (1629–1635); E2: map 8,
    `MT_CYBORG` (1637–1643); E3: map 8, `MT_SPIDER` (1645–1652); E4: map 6 `MT_CYBORG` or map 8 `MT_SPIDER`, other
    maps return (1654–1671); other episodes: map 8, any type (1673–1677). Registered (`registered`) and Ultimate
    (`retail`) take the same non-commercial branch; the source has no other gamemode test here. Nothing in the
    source shows a 1.9 difference; the 1.9 executable is not in this repository.
    (c) **A live player** (1683–1688): the first `i` with `playeringame[i] && players[i].health > 0` (the
    `player_t` health, not the mobj's); none → return. Because `DIE7` never re-enters, a player who is dead at that
    moment means this corpse never triggers.
    (d) **Other boss alive** (1692–1705): walk the whole thinker list from `thinkercap.next`, skipping non-mobj
    thinkers (1694–1695; removed thinkers have function −1, ld/p_tick.c:83, and are skipped too); any `mo2 != mo`
    with `mo2->type == mo->type && mo2->health > 0` → return. "Alive" is `health > 0` only: a Baron in pain, asleep
    or anywhere counts; a corpse at any death state does not. The walk has no side effects, so its order does not
    matter.
    (e) **Victory, E1** (1729–1734): `junk.tag = 666; EV_DoFloor (&junk, lowerFloorToLowest); return;` — **no
    `G_ExitLevel`**. One line each for the rest: commercial map 7: `MT_FATSO` → tag 666 `lowerFloorToLowest`,
    `MT_BABY` → tag 667 `raiseToTexture` (1708–1725); E4M6 → tag 666 `EV_DoDoor (blazeOpen)` (1739–1743); E4M8 → tag
    666 `lowerFloorToLowest` (1745–1749); E2M8, E3M8 and every case not returned above fall to **`G_ExitLevel ()`**
    (1755).
21. **`junk`** is an uninitialised stack `line_t` (1613). `EV_DoFloor (lowerFloorToLowest)` reads only `line->tag`
    (through `P_FindSectorFromLineTag`, ld/p_spec.c:434–445, read at 441); the `lowerFloorToLowest` case reads no
    other line field (ld/p_floor.c:299–305).
22. **`EV_DoFloor (lowerFloorToLowest)`** (ld/p_floor.c:260–444): for every sector with `tag == 666` in index order
    (`P_FindSectorFromLineTag`, ld/p_spec.c:439–444): **if `sec->specialdata` is set, skip it** (277–278); else a new
    `floormove_t` thinker appended to the thinker list (`P_AddThinker`, 282–283), `specialdata` set, `T_MoveFloor`,
    `crush = false`, direction −1, **speed `FLOORSPEED` = 1 unit/tic** (ld/p_spec.h:600), destination
    **`P_FindLowestFloorSurrounding (sec)`** (299–305): the minimum of the sector's own floor and every two-sided
    neighbour's current floor (ld/p_spec.c:270–288), so never above the floor. No sound at start; `T_MoveFloor`
    (ld/p_floor.c:209–253) plays `stnmov` on `leveltime&7 == 0` and `pstop` at the end, clears `specialdata` and
    removes itself on `pastdest`; `lowerFloorToLowest` changes no texture or special. Because the thinker is
    appended and `P_RunThinkers` walks to the current tail (ld/p_tick.c:101–121), **the floor first moves in the same
    tic** as `A_BossDeath`. A floor already at its destination ends at the first call (`floorheight − 1 < dest`,
    ld/p_floor.c:66–79). No draws (crush false; `PIT_ChangeSector` draws only when crushing, ld/p_map.c:1296–1308).
    E1M8 census (counts only): 1 sector has tag 666; 0 linedefs carry tag 666; the 30 linedefs bounding that sector
    have no special (so no manual door or other mover can hold its `specialdata`); 1 sector has special 11 and it is
    not the tag-666 sector.
23. **Both Barons.** If both are corpses when the first reaches `DIE7`, **each** passes (d) on its own `DIE7`
    entry. The second call finds the sector busy if the first floor is still moving (nothing happens), or starts a
    new floor to the then-lowest neighbour (which equals the current floor unless a neighbour has since moved
    lower). If both enter `DIE7` in one tic, the first's floor thinker sits at the list tail, after the second Baron,
    so the second sees `specialdata` set and skips.
24. **A corpse gibbed before `DIE7`** (`PIT_ChangeSector`, ld/p_map.c:1268–1279: a corpse that no longer fits gets
    `S_GIBS`, `MF_SOLID` cleared, height and radius 0) never runs `A_BossDeath`; if it was the last Baron the floor
    never lowers. Type and health are unchanged, so it still counts as dead for the other Baron's (d).
25. **How E1M8 ends: sector special 11** (`P_PlayerInSpecialSector`, ld/p_spec.c:1009–1069, called from
    `P_PlayerThink` after `P_CalcHeight` when the player's subsector sector has a special, ld/p_user.c:274–275):
    nothing unless `player->mo->z == sector->floorheight` (1016–1017); case 11 (1054–1063): `cheats &= ~CF_GODMODE`
    (1056); **20 damage when `leveltime & 0x1f == 0`** via `P_DamageMobj (mo, NULL, NULL, 20)` (1058–1059); **then,
    on every tic on the floor, `if (player->health <= 10) G_ExitLevel ()`** (1061–1062). In `P_DamageMobj` the "end
    of game hell hack" caps any blow to a player whose subsector's sector has special 11 at `health − 1` before
    armor (ld/p_inter.c:837–842; no z test, any source — Baron balls and claws included), so the player cannot die
    there; invulnerability still blocks blows below 1000 (847–851), god mode is stripped only while on the floor.
    With no inflictor there is no thrust and no fall-forward draw; the player's pain draw `P_Random () < 255`
    (painchance 255, ld/info.c:1117) is made because the player always survives.
26. **What the Rust port already has**: special 11 is admitted under the Episode 1 policy at load
    (crates/doom-sim/src/demon_expanded_map.rs:314–325; no thinker, sector_thinkers.rs:440–442), damages on the gate
    and requests the exit (sector_thinkers.rs:545–548, 584–592), and the hell-hack cap is in the shared damage plan
    (crates/doom-sim/src/combat.rs:412, 503). **E1M8 adds no new special-11 code**: it adds the Baron, the ball,
    `A_BruisAttack`, `A_BossDeath`, and a tag-driven `EV_DoFloor` entry (the Rust `do_floor` takes a line index,
    crates/doom-sim/src/movers.rs:676–690; `A_BossDeath` has no line). `A_BossDeath` and `A_BruisAttack` exist in
    Rust only as names (source_actors.rs:9, 16).
27. **The exit tic**: `G_ExitLevel` sets `secretexit = false; gameaction = ga_completed` (ld/g_game.c:1002–1006) inside
    `P_PlayerThink`; the rest of that `P_Ticker` runs (other players, `P_RunThinkers`, `P_UpdateSpecials`,
    `P_RespawnSpecials`, `leveltime++`, ld/p_tick.c:148–157). The next `G_Ticker` runs `G_DoCompleted`
    (ld/g_game.c:636–637), which on map 8 (non-commercial) sets `ga_victory` and returns before any intermission
    (1033–1038); the same `G_Ticker` calls `F_StartFinale` (639–641). The M4a C driver stops after the tic whose
    `gameaction` became `ga_completed` (tools/reference/m4a_driver.c:606–609). **Correction to the brief's wording**:
    the Rust port does not refuse the exit tic; it completes it, sets `level.exit`, and refuses every **later** tic
    with `SimError::LevelExited` (crates/doom-sim/src/lib.rs:608, 1285; test
    crates/doom-sim/src/player_episode1_movers_tests.rs:974–1011).

### C. Hard skill (3) versus medium (2) in single player

28. **THING filter** (`P_SpawnMapThing`, ld/p_mobj.c:708–797): type 11 (deathmatch starts) and types ≤ 4 (player
    starts) return before any skill test (718–737). Then `if (!netgame && (mthing->options & 16)) return` (740–741;
    16 has no `MTF_` name, ld/doomdef.h:140–145); `bit` = 1 for baby, 4 for nightmare, else `1<<(gameskill−1)`
    (743–748): **medium 2 (`MTF_NORMAL`), hard 4 (`MTF_HARD`)**; `if (!(options & bit)) return` (750–751). The Rust
    spawner already maps Hard to 4 (crates/doom-sim/src/spawning.rs:496–503), but the Episode 1 bounds count uses a
    fixed `options & 2` (crates/doom-sim/src/episode1.rs:85).
29. **Every other `gameskill`/skill site** (grep of all `.c`/`.h` for `gameskill`, `sk_*`, `fastparm`,
    `respawnmonsters`, `respawnparm`, `skill`):
    - ld/p_inter.c:95–100 `P_GiveAmmo`: doubles ammo for **baby or nightmare only**. Same on 2 and 3.
    - ld/p_inter.c:799–800 `P_DamageMobj`: halves player damage for **baby only**. Same.
    - ld/p_mobj.c:503–504 `P_SpawnMobj`: `reactiontime` from info **unless nightmare**. Same.
    - ld/p_mobj.c:456 nightmare respawn: needs `respawnmonsters`, set only for nightmare or `-respawn`
      (ld/g_game.c:1416–1419; the demo's respawn byte, 1600). Same (byte 0).
    - ld/p_enemy.c:719 (`P_NewChaseDir` after an attack) and 738–739 (`movecount` gate on missiles): differ only for
      **nightmare or `fastparm`**. Same.
    - ld/g_game.c:1421–1435 `G_InitNew`: halves `S_SARG_RUN1`…`S_SARG_PAIN2` tics and sets ball speeds 20 only for
      `fastparm` or a switch **to** nightmare, restores on a switch **from** nightmare. Same on 2 and 3 with
      `fastparm` 0.
    - ld/g_game.c:1379–1380 clamps skill > 4 to nightmare; 1450 stores it; 1220/1293 savegame byte; 1556/1596 demo
      byte; ld/p_setup.c:588 takes `skill` but its body does not use it (no other `skill` hit in the file).
    - ld/p_enemy.c:1909 `A_BrainSpit` (Doom II): easy or below. ld/m_menu.c:903–911 nightmare confirmation;
      ld/d_main.c:961–970 start skill; ld/st_stuff.c:546 is a comment; ld/d_net.c:500, 521 netgame `respawnparm`.
    **Nothing other than the THING filter differs between skills 2 and 3 in single player.**
30. The first `G_InitNew` of a process sees `gameskill` = 0 (static zero, ld/g_game.c:99), so its nightmare
    restore branch (1429) cannot fire for skill 3; the fast branch needs `fastparm` (0 in all seven IWAD demos).

### D. Demo version

31. **Header** (`G_DoPlayDemo`, ld/g_game.c:1582–1620): `if (*demo_p++ != VERSION)` (1589) with `VERSION` = 110
    (ld/doomdef.h:33); on mismatch it prints "Demo is from a different game version!" to stderr, sets
    `gameaction = ga_nothing` and returns (1590–1594): no `I_Error`, no level load, `demoplayback` stays false
    (ld/ChangeLog:59–63 records that the abort was replaced by this). Then skill, episode, map, deathmatch,
    respawnparm, fastparm, nomonsters, consoleplayer, `playeringame[0..3]` one byte each (1596–1606); a nonzero
    `playeringame[1]` makes it a netgame demo (1607–1611); `G_InitNew (skill, episode, map)` (1615). No field is
    range-checked here (`G_InitNew` clamps skill, episode and map, 1379–1414). Recording writes the same 13 bytes
    (`G_BeginRecording`, 1549–1567).
32. **Ticcmds** (`G_ReadDemoTiccmd`, ld/g_game.c:1491–1503), per tic per player in game: if the next byte is
    `DEMOMARKER` 0x80 (1488) → `G_CheckDemoStatus` and return, leaving `cmd` as copied from `netcmds` (1493–1497);
    else `forwardmove` signed byte, `sidemove` signed byte, `angleturn = byte<<8`, `buttons` byte (1499–1502).
    `consistancy` and `chatchar` are not stored. `G_CheckDemoStatus` with `singledemo` calls `I_Quit` (1660–1661);
    otherwise it resets globals and `D_AdvanceDemo` (1663–1674) and the calling `G_Ticker` continues with the
    non-demo command. No statement anywhere else reads the version: the other `VERSION` uses are the startup
    banner (ld/d_main.c:824–868), the network setup packet (ld/d_net.c:495, 524) and the savegame string
    (ld/g_game.c:1215, 1289).
33. **DOOM.WAD DEMO1** (read-only, header bytes only): 6,854 bytes; header `109, 3, 1, 5, 0, 0, 0, 0, 0, 1, 0, 0, 0`
    (version 109, skill 3, E1M5, one player); last byte 0x80; 6,854 = 13 + 4 × 1,710 + 1; byte 0x80 occurs exactly
    once after the header (the marker), so no `forwardmove` byte would end it early.
34. **Is the 1.9 layout identical?** The 1.9 executable's source is not in this repository, so this cannot be shown
    from source. What can be said: the lump's length, its single marker at the end and its header values are all
    consistent with the 13-byte header and 4-byte ticcmd layout that linuxdoom-1.10 reads, and the version byte is
    the only header byte linuxdoom-1.10 tests. So **patching byte 0 from 109 to 110 in a scratch copy is sufficient
    for the genuine linuxdoom-1.10 code to accept and play it**, and necessary: unpatched, the M4a C driver would
    report "level did not load" (tools/reference/m4a_driver.c:583), the Rust readers refuse it as
    `DemoError::Header { index: 0 }` (crates/doom-sim/src/demo.rs:20–24, 111–150), and
    tools/reference/level_snapshot.py:242 asserts byte 0 == 110.
35. **What a patched run proves.** A Rust-vs-C match proves agreement between Rust and linuxdoom-1.10 on that command
    stream, whatever DOS 1.9 did. It does **not** prove that linuxdoom-1.10 reproduces the DOS 1.9 recording: if the
    engines differ on that path, the command stream is replayed in a diverging world and both C and Rust would
    agree on a playthrough that never happened. Staying plausibly in sync through the end (for example the player
    reaching the exit near the end of the input rather than walking into walls or dying early) is evidence of
    1.9/1.10 equivalence on that path, not proof; there is no 1.9 state trace to compare with. The Linux port was
    edited after the DOS code base (ld/ChangeLog:64–75 records a gamemode boolean-to-enum rewrite), which is one
    possible source of differences; this note does not claim any specific difference.
36. **Rust and tool handling today**: the Rust reader admits exactly `DEMO_HEADER` = `110, 2, 1, 1, 0, 0, 0, 0, 0, 1,
    0, 0, 0` (crates/doom-sim/src/demo.rs:24), or for Episode 1 the same with map 1–9 (`parse_demo_episode1`,
    127–150; decision H10). **Skill 3 is refused** (`Header { index: 1 }`); M4b must admit skill byte 3 for the new
    wire only. The fuzz seeders and script generator write version 110, skill 2 (tools/seed_player_level_fuzz.py:25,
    tools/seed_player_episode1_fuzz.py:39, tools/e1m1_scripts.py:2). The C driver uses `gamemode = registered`
    (m4a_driver.c:29, 573) and takes skill, episode and map from the header (40–41).

### E. Every RNG draw on these paths, in order

`P_Random` only; sound pitch uses `M_Random` (ld/s_sound.c:330, 340), presentation, never `P_Random`.

| Path | Draws in order |
| --- | --- |
| Map spawn (Baron) | `P_SpawnMobj` `lastlook` (ld/p_mobj.c:506); spawn tics `1 + P_Random() % 10` (788) |
| `A_Look` (Baron) | none of its own: `P_LookForPlayers` and `P_CheckSight` draw nothing (ld/p_enemy.c:499–560); `brssit` is the default branch (648–650) |
| `A_Chase` | `JUSTATTACKED`: `P_NewChaseDir` draws, return. Melee branch: none in A_Chase, then `ATK1` A_FaceTarget (0, or 2 if target `MF_SHADOW`). Missile branch with `movecount == 0`: `P_CheckMissileRange` one draw (252) unless it returns earlier on sight, `MF_JUSTHIT` or `reactiontime`; then `ATK1` A_FaceTarget (0/2). Fall-through: `P_NewChaseDir` draws if `--movecount < 0` or `P_Move` fails (408 `P_Random() > 200` when reached, 450 `P_Random()&1` when reached, 356 `P_Random()&15` per successful `P_TryWalk`), then the active-sound draw (772) |
| Attack cycle | `ATK1` A_FaceTarget (0/2) → 8 tics → `ATK2` A_FaceTarget (0/2) → 8 tics → `ATK3` A_BruisAttack: **claw**: damage (989) → target's `P_DamageMobj` draws; **ball**: `lastlook` (p_mobj.c:506) → shadow pair if target `MF_SHADOW` (910) → spawn tics (870) → if the half-step hits a shootable non-species thing: damage (p_map.c:323) → `P_DamageMobj` draws → `P_ExplodeMissile` tics (96); if it is blocked otherwise (wall, species, solid non-shootable): `P_ExplodeMissile` tics (96); the sky-hack removal applies only in `P_XYMovement` (p_mobj.c:170–180) |
| Ball in flight | as the Imp ball: contact damage then `P_DamageMobj` draws then explode tics; wall/floor/ceiling: explode tics |
| `P_DamageMobj` on a Baron or player | fall-forward `P_Random()&1` only if inflictor present, target not `MF_NOCLIP`, source not a chainsaw player, `damage < 40`, `damage > health`, `target z − inflictor z > 64` (ld/p_inter.c:806–823, `&&` short-circuit); if the target survives: pain draw (894) |
| Pain | the draw above; `A_Pain` none |
| Death | `P_KillMobj` tics (ld/p_inter.c:726); `A_Scream` none (default branch); `A_Fall` none; `A_BossDeath` none; `EV_DoFloor` and `T_MoveFloor` none |
| Special 11 tic on the gate | `P_DamageMobj(player, NULL, NULL, 20)`: no fall-forward (no inflictor); pain draw (894) |

### Hazards and witnesses (Baron, A_BossDeath, skill, demo)

1. **Shadow fuzz** `(P_Random()−P_Random())<<21` (ld/p_enemy.c:795) and `<<20` (ld/p_mobj.c:910): unspecified
   evaluation order and a left shift of a negative value (undefined in C). Needs the project's declared ordered-draw
   adapter and a C observation; reachable on E1M8 hard (10 spectres) and with an invisible player.
2. **`P_CheckMissileRange` shift** `dist >>= 16` (ld/p_enemy.c:222) of a negative value when the target is under 64
   units but outside melee range (or melee failed): implementation-defined right shift (gcc: arithmetic); then
   `P_Random() < dist` is false and the Baron fires. Declared adapter (arithmetic shift), not a witness.
3. **Uninitialised `junk`** (ld/p_enemy.c:1613): harmless for `lowerFloorToLowest` (only `tag` is read). A Rust port
   must use a tag-only entry and must not read any other line field; any other floor kind reached from a tag-only
   call must be refused rather than invented.
4. **Busy tag-666 sector**: skipped silently in C (ld/p_floor.c:277–278). Faithful, no witness; on E1M8 only the boss
   floor can occupy it (fact 22 counts).
5. **Same-tic floor thinker**: the floor must be appended to the thinker list and run later in the same tic; a port
   that defers it one tic diverges.
6. **Gibbed Baron corpse / dead player at `DIE7`**: faithful outcomes (the floor never lowers); the port must model
   them, not fix them.
7. **Exit tic**: `G_ExitLevel` from inside `P_PlayerThink`; the rest of the tic runs; later tics refused
   (`LevelExited`), as M4a does.
8. **Demo marker**: in C the marker tic is not a demo tic (singledemo: process exit before `P_Ticker`; otherwise a
   stale command runs). Runs must stop at the marker, as both the C driver and the Rust reader do.
9. **Baron-vs-Baron infighting through barrel splash** (fact 16): plausible on E1M8 but not observed; needs a C
   witness before it is called covered.
10. **Thrust overflow** `damage*(FRACUNIT>>3)*100` (ld/p_inter.c:817): not reachable from a Baron (≤ 80 damage), noted
    only because the telefrag adapter is separate.

### Corrections to the M4a note (Baron, A_BossDeath, skill, demo)

Each item cites the section of docs/M4A_REFERENCE_INVESTIGATION.md that it corrects.

1. §3.4.3 "45–48 tics after death": 44–47 tics after the kill tic for kills made in `P_PlayerThink` (hitscan, fist,
   chainsaw) or by a thinker earlier in the list; 45–48 for kills by a later thinker such as a rocket (fact 19).
2. §3.4.4 table, Baron missile: a contact during the spawn half-step makes `P_TryMove` fail, so the damage draw,
   the `P_DamageMobj` draws **and** the `P_ExplodeMissile` tics draw all happen; the table reads as if they were
   alternatives (fact 8).
3. §3.4.2 split test: `FixedMul(15·FRACUNIT, finecosine)` is at most 983025, strictly below `MAXMOVE/2`, not "≤ 15·
   FRACUNIT" with equality possible (largest table entry 65535) (fact 9).
4. §3.4.2 `P_CheckMeleeRange` cited as p_enemy.c:174–195; the function is 174–191 (fact 5).
5. §3.4.2 `A_Chase`: add that only the missile branch needs `movecount == 0` and sets `MF_JUSTATTACKED`; the melee
   branch does neither (fact 10).
6. §3.4.2 species rule: the Baron is not immune to other Barons; splash with a Baron source can start Baron-vs-Baron
   fighting, after which claws hurt (fact 16).
7. §3.4.3 item 1 omits the commercial branch and the E2/E3/E4/default cases; for completeness: E2M8/E3M8 and the
   default case call `G_ExitLevel` (fact 20).
8. The brief's "the Rust port refuses that tic, as with exits": the exit tic completes; later tics are refused
   (fact 27).
9. §6 "On skill 3 only the THING filter differs from medium in single player (`options & 4`)": confirmed by the full
   grep (facts 28–30).

Not done: no C or Rust build or run; no observation of the shadow-fuzz draw order; E1M8 geometry (heights,
positions) deliberately not read or recorded beyond the counts above.

## 4. Rust inventory (wire 23 baseline; what wire 24 and DSIM40 must touch)

This section is a read-only survey of the tree at `b28d4cf` (branch `feat/m4b-episode-1-complete`, the same as master after M4a merged). Every
citation is `path:line` against that tree. No cargo build or test was run, and the IWAD was not read. Per-map counts come only from the
committed census tables in `docs/M4A_REFERENCE_INVESTIGATION.md` §2, which were counted at **medium skill**. "Unverified" marks anything
inferred from reading code that was not checked against a run. The C facts quoted come from the M4a note (§3, §6, §8 and §14) and must be
re-cited from linuxdoom-1.10 in the M4b note before code relies on them.

Names in this section: wire 24 is `PlayerMonstersEpisode1V2` (decision I2, confirmed by the owner on 2026-11-10) with DSIM40;
the predicate name `has_episode1_complete()` is a working name for the implementation step.

### 4.1 What wire 23 touches that wire 24 must extend

#### 4.1.1 The policy enum and its predicates

`OriginalExecutionProfile` is in `crates/doom-sim/src/original_registry.rs`: the variant `PlayerMonstersEpisode1V1` is at :102, `wire_id`
at :106–131 (23 at :130) and `context_schema` at :134–159 (:158). Each predicate is a `matches!` list. Wire 24 is a strict superset of wire
23, so it is added to every list wire 23 is in. The table shows each predicate and what wire 24 does with it.

| Predicate | Line | Wire 23 in it | Wire 24 |
| --- | ---: | --- | --- |
| `has_player_armor` | 161 | yes (:177) | inherit (add variant) |
| `has_player_bonuses` | 180 | yes (:195) | inherit |
| `has_monsters` | 199 | yes (:213) | inherit |
| `has_imps` | 217 | yes (:230) | inherit |
| `has_map_scale` | 235 | yes (:247) | inherit |
| `has_heights` | 253 | yes (:264) | inherit |
| `has_diagonals` | 269 | yes (:279) | inherit |
| `has_real_map` | 284 | yes (:293) | inherit |
| `has_specials` | 299 | yes (:307) | inherit |
| `has_movers` | 314 | yes (:321) | inherit |
| `has_e1m1` | 327 | yes (:333) | inherit |
| `has_barrels` | 340 | yes (:345) | inherit |
| `has_level` | 355 | yes (:358) | inherit |
| `has_episode1` | 368 | **only** wire 23 (:369) | **inherit: widen to both variants** (recommended; see below) |
| `has_world_pickups` | 371 | yes (:388) | inherit |
| `has_player_body` | 391 | yes (:415) | inherit |
| `has_physical_player` | 418 | yes (:441) | inherit |
| `has_player_weapons` | 444 | yes (:463) | inherit |
| `has_weapon_pack` | 468 | derived (`has_player_weapons && !PistolV1`) | inherits automatically |
| `has_player_commands` | 471 | yes (:493) | inherit |
| **new** `has_episode1_complete` | — | — | new: teleport, Baron, `A_BossDeath`, hard skill, the map number |

Other registry sites. `OriginalExecutionRegistry::new` (:880–921) and `validate` (:930–975) each need the new variant in their `match`
(:904 and :960). `is_policy_monster` (:826–831) gains `|| (policy.has_episode1_complete() && is_bruiser(ty))`. `extended_policy`
(:832–842) gains a Baron arm. `require_role_for` (:1022–1082) needs `MT_BRUISER`, `MT_BRUISERSHOT`, `MT_TFOG` and `MT_TELEPORTMAN` behind the
new predicate, in an arm of its own next to the Spectre/rocket arm (:1060–1062). `state_role_for` (:1083–1140) needs a policy for each of
those four types. The player-body option check `options != ActorOptions::default()` (:1195–1201) refuses every skill except medium; wire 24
must admit `Skill::Hard`, and older wires keep this check unchanged.

**Should `has_episode1()` widen to include wire 24?** It has 33 non-test call sites (listed by grep, §4.7). Behaviourally almost all of them
mean "Episode 1 rules", which wire 24 keeps: the vanilla command domain, the chainsaw thrust rule, sector 11, the per-map bounds,
`floorpic`, powers and the turbo message. The exceptions are the sites that select a *schema* or an *exact* version. These must test the new
predicate first:

- `snapshot.rs:732` (version selector);
- `diagnostic.rs:499` (`policy()`) and the 41 `version == 39` tests in diagnostic.rs (§4.6);
- `player_map_scale.rs:31` (`actor_slots`, 1,024 for both, fine) and `:982` (`live_projectile_bound`, fine);
- `original_registry.rs:1229/1242/1265` (certificate, boundary, bounds; fine to inherit).

Widening is the M3h9c pattern (`has_level` covered wires 22 and 23). The alternative, adding `|| has_episode1_complete()` at 33 sites,
gives the same behaviour with more edits.

**Defence-in-depth flag in the movers.** The mover code does not read the policy. It reads `world.episode1.is_some()` as its "wire 23"
test: `line_triggers.rs:166–171` (crossing specials), `movers.rs:377–382` (`Starter::episode1`) and `demon_expanded_map.rs:226`
(sector-slot capacity). Wire 24 would also set `world.episode1 = Some(..)`. So the wire-24-only paths (special 97, the tag-666 boss floor)
need a separate world flag, for example `world.episode1_complete: bool` or a policy field inside `Episode1Bounds`. Without it, the
defence-in-depth refusal cannot tell wire 23 from wire 24.

#### 4.1.2 Admitted line specials, sector specials and thing types

- **Line specials (load rule 1).** `mover_rules::validate_movers_with` (mover_rules.rs:424–504; `Rules` enum :376–385, variants `Movers`,
  `E1M1`, `Episode1`). The Episode 1 set is the arms at :461 (`35 | 51 | 7 | 8 | 9 if episode1`), :464–487 (doors, floors, plats) and
  :488–503 (the wire-22 set). Any other special is `PursuitError::Specials` (:504). Line 97 needs a new `Rules::Episode1Complete` variant
  (or a flag) and an arm next to :461 that moves no tagged family. Do **not** add it under `if episode1`: that would admit 97 under wire 23.
- **Crossing dispatch.** `line_triggers::cross` (line_triggers.rs:155–219) has a wire-23 defence list (:166–172) and the
  `(crosser, special)` match (:173–208). 97 is a monster-allowed special in `P_CrossSpecialLine`, so its arm must cover both
  `Crosser::Player` and `Crosser::Monster`. `clears` (:131–142) is unchanged, because 97 is WR. **`cross` cannot host the teleport**: its
  `Starter` (movers.rs:339–349) has only mover-store access, with no actor store, spatial index or damage service (§4.2.1).
- **Sector specials.** `ADMITTED_SECTOR_SPECIALS` (sector_thinkers.rs:39) is checked by `validate_sector_specials_for(map, episode1)`
  (demon_expanded_map.rs:316–326), which admits 11 only with `episode1` (:321). **Sector special 11 is fully implemented under wire 23**:
  - the light-thinker exclusion at sector_thinkers.rs:442;
  - `P_PlayerInSpecialSector` 20 damage on the gate and exit at health ≤ 10 (sector_thinkers.rs:525–586);
  - the `P_DamageMobj` cap `amount = health − 1` (combat.rs:491–506).

  Wire 24 inherits all of it. The census finds no other new sector special in Episode 1.
- **Thing types.** THING admission is `Simulation::spawn_map_things` (spawning.rs). The skill/single-player filter (:496–503) is already
  generic over `Skill` (bit 1/2/4). The type gate (:518–539) admits only a pickup, a policy monster (`is_extended_monster && is_policy_monster`),
  a decoration, a barrel or the owner slot. Anything else is `MapSpawnError::UnsupportedType`.
  - **Baron (3003):** admitted once `is_extended_monster` and `is_policy_monster` include it. Today `is_extended_monster`
    (original_registry.rs:818–820) is former humans, the Imp and the Spectre.
  - **`MT_TELEPORTMAN` (14):** none of the roles fits. It needs a role of its own, for example `is_teleport_destination(policy, ty)`,
    added to the gate and the `require_role_for`/`state_role_for` arms. It is NOSECTOR|NOBLOCKMAP (flags 24, source_actors.rs:1347).
    **Its spawn state is `S_NULL`** (property 1 is 0). In C it stays forever in `S_NULL` with `tics` −1. The Rust model already
    distinguishes `Some(StateId(0))` from "removed" (`ActorLife::state` doc, lifecycle.rs:1188–1189; `new_actor` sets
    `state: Some(info.states()[0])`, :1374). *Unverified*: `ActorPolicy::source`'s closure walk and the per-tic invariants have never
    been exercised on a live actor whose state is `S_NULL`. Needs a test.
  - **Per-map bounds.** `Episode1Bounds` (episode1.rs:46–140) counts `EPISODE1_MONSTERS` (:20, six doomednums: no 3003),
    `EPISODE1_PICKUPS`, barrels and statics, with the medium filter `options & 2` hard-coded (:85). The teleport destination (14) and the
    Baron are not counted. Changing these constants changes the wire-23 derivation and the DSIM39 reader's answers (§4.7). Wire 24 needs
    policy-keyed lists or a separate constructor, `Episode1Bounds::from_map_for(map, policy, skill)`.

#### 4.1.3 Actor types and state tables

The generated tables already hold every info.c record; nothing new needs generating. `source_actors.rs` has all 967 states and every type:

| Record | Location |
| --- | --- |
| `MT_BRUISER` | :1321 (mass 1000, health 1000, speed 8, radius 24, height 64, no xdeath) |
| `MT_BRUISERSHOT` | :1322 (flags 67088 = MISSILE\|DROPOFF\|NOGRAVITY\|NOBLOCKMAP, speed 15, damage 8) |
| `MT_TFOG` | :1345 (flags 528 = NOGRAVITY\|NOBLOCKMAP) |
| `MT_TELEPORTMAN` | :1347 |
| `S_TFOG`..`S_TFOG10` | from :466, no action |
| `S_BRBALL*` | :858–863 |
| `S_BOSS_*` | :864–887 |

The typed identities are in identity/names.rs: `MT_BRUISER` 15 (:19), `MT_BRUISERSHOT` 16 (:20), `MT_TFOG` 39 (:43), `MT_TELEPORTMAN` 41
(:45), `S_TFOG` 130 (:273), `S_BRBALL1` 522 (:665) and `S_BOSS_DIE1` 542 (:685).

**They are "known but unadmitted"**: no role or policy names them. The only mention outside the tables is the Knight/Baron species comment
at projectiles.rs:413.

The Baron's normal closure, read from the action indices:

| States | Action |
| --- | --- |
| `S_BOSS_STND` | `A_Look` |
| `S_BOSS_RUN*` | `A_Chase` |
| `S_BOSS_ATK1/2` | `A_FaceTarget` |
| `S_BOSS_ATK3` | `A_BruisAttack` |
| `S_BOSS_PAIN2` | `A_Pain` |
| `S_BOSS_DIE2` | `A_Scream` |
| `S_BOSS_DIE4` | `A_Fall` |
| `S_BOSS_DIE7` | `A_BossDeath`; tics −1, so it runs on entry into the corpse state |

The `S_BOSS_RAISE*` states stay inert, like every other raise chain. Admission work:

- a `bruiser_policy()` modelled on `imp_policy` (original_registry.rs:684–713), with `A_BruisAttack` and `A_BossDeath` in its list;
- a `bruisershot_policy()` modelled on `fireball_policy` (:715–730; no actions);
- policies for TFOG (no action, ends in `S_NULL` like the puff) and TELEPORTMAN (the `S_NULL` state only);
- the actions themselves. `actions::OriginalAction` (actions.rs:20–50) lacks `BruisAttack` and `BossDeath`. Naming a variant grants
  nothing; dispatch arms in `LifeHost::action` (lifecycle.rs:1576–1690, `unreachable!` at :1689) and `validate_state_with`'s
  `SpecialActions` (lifecycle.rs:240–270) must admit them.

`ALLOWED_FLAGS` (lifecycle.rs:~224) and its per-profile extensions already include NOSECTOR, NOBLOCKMAP and NOGRAVITY. MISSILE is added
for the Imp profiles. *Unverified*: whether the per-profile extension lists also admit DROPOFF on a missile.

#### 4.1.4 Movers and thinkers

`FloorKind::LowerFloorToLowest` **exists and is admitted under wire 23** (movers.rs:91–107; `Starter::do_floor` movers.rs:682–720; crossing
38/82 at line_triggers.rs:180–182). `A_BossDeath`'s `EV_DoFloor(&junk, lowerFloorToLowest)` with `junk.tag = 666` needs a tag-based entry:
`do_floor` takes a *line index* and reads its tag (`Starter::tagged`, movers.rs:365–374). A `do_floor_tag(tag, kind)` (or a `Tagged` source
enum) is the small change. It must be reachable only under wire 24.

The sector-thinker slot class under wire 23 is already `L + 2·numsectors` (demon_expanded_map.rs:223–234), so the boss floor needs no new
capacity. The reachable-height fixpoint (`reachable`, mover_rules.rs:~565) does not know that tag-666 sectors can lower on E1M8. Under wire
23 `R_min` is read only by V′ case (e), where a value that is too high can only refuse more (M4a §14 A3), so the omission is safe but may
refuse a legal tic. The rule must be added for wire 24 when the map number is 8.

#### 4.1.5 Per-map bounds (H4) as wire 24 needs them

- **Derivation:** `Episode1Bounds::from_map` (episode1.rs:69–100), called once at original_registry.rs:1264–1266. The DSIM39 reader
  recomputes it from `map.things` (`from_thing_records`, episode1.rs:74–81; diagnostic.rs:425).
- **Consumers:**

  | Consumer | Site |
  | --- | --- |
  | `map_demon_limit` | player_roster.rs:82–90 (K) |
  | pickup/bonus/world-item bounds | world_pickups.rs:44, 55, 67 |
  | decorations, barrels | episode1.rs:181–193 |
  | sight queries `max(64, 8(K+1))` | episode1.rs:111–114 |
  | explosion sight queries and visit roster | episode1.rs:120–139 |
  | store cap | `actor_slots` 1,024 (player_map_scale.rs:25–37) |
  | missile bound (the store itself, D10) | player_map_scale.rs:981–989 |

- **Wire 24 changes:**
  - K includes the Baron (3003).
  - A new count for teleport destinations, or leave them to the store-full witness.
  - TFOG spawns (two per teleport, about 60 tics each) and Baron shots need no per-type bound, because D10 already lets the store-full
    witness cover all missiles. Each spawn site still reserves its slots first (`reserve`) and refuses the tic with `StoreError::Capacity`.
  - The skill bit (2 → 4) must be a parameter in `from_things` (:85).

#### 4.1.6 Refuse-the-tic witnesses and their error enums

`SimError` (lib.rs:282ff) wraps the per-domain enums. The wire-23 witnesses are:

| Witness | Error enum and variant | Where it lives |
| --- | --- | --- |
| Ninth `spechit` | `MapScaleError::SpecialCross` | player_map_scale.rs:164 (enum :78–200); raised at line_triggers.rs:44–51, movement.rs:614–619, pursuit.rs:185–190 |
| Store full | `StoreError::Capacity` | store.rs |
| Blocked plat or stair (H7) | `MapScaleError::PlatBlocked`, `StairBlocked` | player_map_scale.rs:184–190 |
| Busy door family | `MapScaleError::MoverFamily` | per tic under wire 23 |
| Donut busy | `MapScaleError::DonutBusy` | player_map_scale.rs:194 |
| Position, escape, vertical | `MapScaleError::Position`, `Escape`, `Vertical` | player_map_scale.rs |
| Checked trace arithmetic and intercepts | `MapScaleError::Arithmetic` / `Intercepts`; `CombatError::Coordinates` | player_map_scale.rs; combat.rs |
| Sector-height domain (A1 fix) | per-tic witness | commit `b7216ce` |
| Sight allowance | `PerceptionError::WorkLimit` | — |
| Level exit | `SimError::LevelExited` | lib.rs:316, 1284–1286 |
| Reborn | `SimError::PlayerReborn` | lib.rs:1287–1289 |

A wire-24 telefrag needs one more decision: damage 10000 meets the 1,000 bounds (§4.2.4). Either they are widened under wire 24 or a new
explicit witness is added.

### 4.2 Where each new path hooks in

#### 4.2.1 Line crossing and `spechit`

- **`spechit` today.** `SpecHit` is a fixed `[u16; MAX_SPECIAL_CROSS]` (line_triggers.rs:22–42; `MAX_SPECIAL_CROSS` = 8,
  mover_rules.rs:27–28). `spechit()` (line_triggers.rs:56–78) collects a position check's two-sided openings with a nonzero runtime special.
  A ninth entry returns `SpecialCrossOverflow`, which is `MapScaleError::SpecialCross`.
- **The "2 × 2 window of 8" load rule.** `special_window_lines` (mover_rules.rs:830–860) counts, for every 2 × 2 window of the map's own
  BLOCKMAP cells, the distinct two-sided lines with a nonzero static special. Load rule 6 (mover_rules.rs:566–570) refuses
  `MapScaleError::SpecialWindow` when the maximum exceeds 8, under the `Movers` and `E1M1` rules (wires 19–22). **Under wire 23 the rule is
  already dropped** (`if episode1 { donut_geometry } else if special_window_lines > 8 ...`, :566–570). The ninth-entry refusal is then
  the only guard, as a per-tic witness (validate_episode1 doc, :358–359). The rule can be bypassed only in `cfg(test)`
  (`with_window_rule_bypassed`, :597–603). So the M4b "MAXSPECIALCROSS witness" is already live under wire 23 and inherited by wire 24.
  What is missing is a synthetic WAD plus a genuine-engine run that reaches it.
- **Crossing loops, three of them.**
  - **Player and monster momentum (`P_XYMovement`).** `movement::Mover::attempt` (movement.rs:586–657) collects the crossings of each
    accepted attempt as `MotionEvent::Cross` (:629–651), **deferred**. The thinker runs `line_triggers::cross` for each one only after the
    whole motion (half-steps and slides) has finished (thinkers.rs:254–300).
  - **Monster `P_Move`.** `pursuit::move_actor` (pursuit.rs:164–211) crosses **immediately** after the accepted move through `cross_lines`
    (pursuit.rs:262–289).
  - **Missiles.** `projectiles::try_move` (projectiles.rs:~197–251, crossings at :238) only records the crossing; a missile returns before
    any action, as in C.
- **Why teleport breaks the deferred model.** In C, `EV_Teleport` runs inside `P_TryMove`'s crossing loop. It moves the thing, zeroes its
  momentum and resets `numspechit`, which aborts the remaining loop (M4a §3.3.3: −1 after the teleport). `P_XYMovement`'s local
  `xmove`/`ymove` halves then continue from the new position (the "split-move continuation"). Rust computes the remaining half-steps and
  slides from the old trajectory before any crossing runs. Teleport therefore has to execute **inside `Mover::attempt`**, through a new
  `MotionAccess` hook.
  - The precedent is the pickup hook `MotionAccess::contacts` (movement.rs:519–528), which mutates the store from inside the motion
    through `world_pickups::Contacts { access: &mut context, .. }` (thinkers.rs:~180–200).
  - Corpses, barrels and other non-player, non-missile things that slide with momentum use `move_xy_multisector`, whose access is a plain
    `Fn` closure (`MotionAccess for F`, movement.rs:556–561, `Infallible`). They cannot host a teleport. Wire 24 needs a store-capable
    access for them, or an explicit refusal for "a non-player, non-monster mover crosses 97".
  - In C, any non-missile thing may teleport through 97: `P_CrossSpecialLine`'s non-player list includes 97, and `EV_Teleport` refuses
    only `MF_MISSILE` and side 1. *C citation required.*
  - Monster `P_Move` already crosses immediately. It needs the teleport hook in `cross_lines` (store access through `h.access`) and
    `a.z = a.floor` from the **new** floor after a teleport (pursuit.rs:197 sets z before crossing).
- **The thinker scan for the destination.** `EV_Teleport` walks sectors in index order. For each sector with the tag it walks the whole
  thinker list for a mobj of type `MT_TELEPORTMAN` whose subsector's sector is that sector. In Rust: `self.actors.order()` (store.rs:452–465,
  actor subsequence of the shared order, **including pending removals**: filter with `is_pending`, since a pending mobj's function is −1
  in C), then `location.sector`. The world's `sector_lines` and `mover_rules::tagged` (used by `Starter::tagged`) give the tagged sectors.

#### 4.2.2 Use and shoot specials

97 is a walk-over special only, so it has no use or shoot hook. The use path (`use_lines::use_special_line`, called from pursuit.rs:236–262 and
the player use trace) and the shoot path (`shoot_special_line` from combat_trace.rs) need no change for M4b. The census finds no new use or
shoot special on E1M5, E1M8 or E1M9 beyond wire 23's set (§2 of the M4a note).

#### 4.2.3 Thing relinking (reuse for `P_TeleportMove`)

`SpatialIndex` (movement.rs:37–54) is one most-recent-link vector. `link(id)` = `unlink` + `insert(0)`, so a relink puts the actor at the
head, as `P_SetThingPosition` does. `unlink(id)` removes it. Cell and sector lists are filters of that one order: cells in
`for_things`, movement.rs:244–270; sectors with `lifecycle::in_sector_list` in snapshot.rs:1040–1058.

`P_TeleportMove`'s unset/set is `actor.spec.position = dest; floor/ceiling/location = …; spatial.link(id)`, the same sequence as
`Mover::attempt` (movement.rs:621–628) and `move_actor` (pursuit.rs:192–201). `PIT_StompThing` can reuse `movement::for_things` (the
`MAXRADIUS`-style cell window over `spatial.order`), with its own predicate:

- skip non-shootable things;
- skip things beyond the radius sum;
- a non-player stomper fails (the MAP30 exception is irrelevant in Episode 1);
- `P_DamageMobj(thing, tmthing, tmthing, 10000)`.

*Unverified*: whether `for_things`' margin (`radius + 32`, movement.rs:254) equals `P_TeleportMove`'s box. `P_BlockThingsIterator` over
`tmbbox ± MAXRADIUS` must be re-cited.

#### 4.2.4 The damage service, thrust and the 10000-damage overflow

- **The bounds that refuse 10000:**

  | Site | Check |
  | --- | --- |
  | `DamagePlan::current_original_in` (combat.rs:415ff) | `!(0..=1000).contains(&request.amount)` at :425 |
  | combat.rs:740 | the same check |
  | combat.rs:873 | the same check on `LineAttack` damage |
  | `preflight_player_thrust` | original_player.rs:603 |
  | `preflight_impulse` | `MAX_PLAYER_DAMAGE` = 1000 (original_player.rs:13, :694) |

  A telefrag is refused with `CombatError::Damage` before anything else.
- **The thrust multiplication.** The applied thrust is `request.amount * THRUST_FRACTION * THRUST_MASS_SCALE / info.mass()` (combat.rs:986;
  `THRUST_FRACTION` = 8192 and `THRUST_MASS_SCALE` = 100, original_player.rs:14–15). This is **plain `i32` arithmetic**: it panics in
  debug and wraps in release. The preflight twin uses `checked_mul` (original_player.rs:705–709), so today it refuses
  (`CombatError::Metadata`) before the apply could overflow.
  - Overflow starts at amount 2,622, because 2,622 · 819,200 > 2³¹ − 1.
  - For 10000 the C expression `damage*(FRACUNIT>>3)*100/mass` overflows signed `int` (undefined behaviour in C). In practice it wraps to
    −397,934,592 before the division. That gives −3,979,345 raw (about −60.7 units) for mass 100, −994,836 raw for mass 400 and
    −397,934 raw for mass 1000.
  - These figures were computed in this survey and not observed here (§2, T8.5 records them as observed in a gcc probe). The M4a note proposes a declared wrapping adapter, which must be confirmed against
    the genuine runner's overflow variant.
- **Downstream bounds.** The resulting momentum must still pass `momentum_admitted` (4,096 units, player_map_scale.rs:874–880) and the
  thrust preflight's `travel_admitted`. The victim's health becomes about −9,990. *Unverified*: whether any DSIM39 reader health domain
  refuses that. The grep found only sign tests at diagnostic.rs:1427, 1619 and 1657–1658, but no 10000-damage capture has ever been read.
- **Who is hit.** `PIT_StompThing` damages any shootable thing (monsters, barrels and, for a monster stomper, nothing, because monsters do
  not stomp). Each kill can drop an item (`P_KillMobj`) and start a barrel explosion chain later. All slots must be reserved
  (`plan.spawn_count()`) before the first telefrag mutates anything.

#### 4.2.5 Missile spawning (the Baron shot)

`A_BruisAttack` has the shape of `A_TroopAttack`:

- melee: `P_CheckMeleeRange` → `sfx_claw` → `(P_Random()%8+1)*10`;
- otherwise `P_SpawnMissile(MT_BRUISERSHOT)`. *C citation required.*

The Rust template is `player_monsters::troop_attack_checked` (player_monsters.rs:497–527), with the Imp constants `IMP_DAMAGE_*`. Its missile
branch calls `projectiles::launch_fireball` (projectiles.rs:550–589), which **hard-wires `FIREBALL`** (`const FIREBALL`, :13) in four places:

- the live bound (:558);
- the speed (:564);
- the `Launch` event (:578);
- the spawn (:582).

`launch_fireball` should take the type as a parameter (`launch_missile(h, source, dest, ty)`). Other `FIREBALL`-only sites:

- `preflight_live_missiles` (projectiles.rs:854–873) counts only `FIREBALL` against the bound at :858. Under wire 23 the bound is the
  store, so this is harmless, but the owner check loop already uses `is_missile_type`.
- `is_missile_type` (original_registry.rs:806–808) is fireball or rocket; add the Baron shot behind the role.
- The thinker dispatch reaches `projectiles::fly` only for `is_missile_type` (thinkers.rs:125–155).
- The DSIM reader's projectile-type check (diagnostic.rs:2025–2029: fireball, or rocket under v39).

Flight, contact damage `((P_Random()%8)+1)*damage` and explosion are already generic over the type definition (projectiles.rs:197–352). The
Knight/Baron species rule is already present (projectiles.rs:410–413).

#### 4.2.6 Chase and attack dispatch

`LifeHost::action` (lifecycle.rs:1576–1690). `A_Chase` → `pursuit::chase::chase` (:1642–1645). `A_TroopAttack` → `troop_attack` (:1622–1625).
Add `Some(OriginalAction::BruisAttack) => player_monsters::bruis_attack(self, id)`. `A_Look`, `A_Chase`, `A_FaceTarget`, `A_Pain`,
`A_Scream` and `A_Fall` are shared and need nothing beyond the closure admission. The Baron's sounds come from its type record.

#### 4.2.7 Death-sequence dispatch (the `A_BossDeath` hook)

`A_Fall` (lifecycle.rs:1649–1653), `A_Scream` (:1669–1687) and `A_XScream` (:1660) are in the same match. `A_BossDeath` is a new arm. Under
the episode-1 branch of the C body (re-cite p_enemy.c):

- return unless `gamemap == 8` and the type is `MT_BRUISER`;
- return unless some player in game has health > 0;
- scan every thinker for a mobj other than this one of the same type with health > 0, and return if one is found;
- on E1M8, `EV_DoFloor(junk.tag = 666, lowerFloorToLowest)`.

**The Rust simulation has no map number.** `doom_map::Map` carries no name (crates/doom-map/src/parse.rs:80–91), and nothing in doom-sim
stores `gamemap` or `gameepisode` (grep finds none). The wire-24 constructor must take the episode and map, for example
`original_player_episode1_complete_map(map, episode, map_number, skill, options)`. The value becomes canonical state, encoded in DSIM40
(§4.6). The runner gets it from `--map` and the demo header.

The scan for a living Baron uses `self.access` (`ActorAccess`) over the actor order, skipping pending entries and this actor. Order is
irrelevant to the answer (a pure existence test). The floor starter needs a `movers::Starter` built as in `pursuit::cross_lines`
(pursuit.rs:271–282). Its store is `h.access.movers()`, with `specials: None` (`lowerFloorToLowest` needs no specials).

#### 4.2.8 Thinker-list scans

- `ThinkerStore::order()` (store.rs:452–465) walks the actor subsequence of the shared run order, pending included.
- `shared_order()` (:439–447) walks both kinds.
- `run()` (:550–567) is the append-sensitive tic traversal: spawns during the tic are visited in the same tic, as in `P_RunThinkers`.
- Both teleport scans (destination, living Baron) are read-only during an action. They must not call `run`; they iterate `order()` and
  read through `get`/`is_pending`.

### 4.3 Thinker order and actor order

- **Append.** `ThinkerStore::spawn` (store.rs:398–409) takes the lowest free (nonretired) slot and appends at the tail of the shared order,
  which is `P_AddThinker`. `reserve_spawns` (:384–386) is the preflight. Spawns go through `LifeHost::spawn` (used by `launch_fireball`,
  projectiles.rs:581). The two TFOG spawns must happen in C order (old position, then new position) **after** the relink. Each
  `P_SpawnMobj` draws `P_Random` for `lastlook` (M4a §14 B6 bullet), so the order of the fogs relative to any telefrag drop spawns is
  RNG-visible. *C citation required for the exact `EV_Teleport` sequence.*
- **Removal.** `mark_remove` (store.rs:425–431) marks only. `next_visit` (:524–548) unlinks a marked entry when the traversal reaches it,
  and the slot's generation is bumped on retire. TFOG ends at `S_NULL`: state entry calls `ProbeAction::RemoveSelf` → `mark`, like the puff.
- **Teleport is a move without removal.** The actor keeps its slot, generation and position in the thinker order. Only `spatial.order`
  changes (relink to the head) along with position, angle, momentum (zeroed) and z, plus `reaction_time = 18` for the player
  (`life.reaction_time`; the player decrement is lib.rs:520–523; the invariant `0..=255` at original_player.rs:416 admits 18).
  `spatial.link_order` in DSIM records the relink. Sector lists follow from `location.sector`.
- **Boss death** adds one floor mover through `spawn_sector` (store.rs:412–423, tail of the shared order) during the actor's own
  state-entry action, so it runs in the same tic, as in C (`P_RunThinkers` reaches the new tail). This path already exists for crossings
  and uses.
- **Telefrag kills** run `P_KillMobj` inside another actor's thinker: drops are appended and the victim's death state is entered. The
  same paths are used today by hitscan kills during the player phase. The order-sensitive part is that drops and fogs interleave by
  call order. *Unverified* whether the kill's state entry (`A_Scream`/`A_Fall` draws) runs before or after the fog spawns. Follow the C
  sequence.

### 4.4 Skill: every place medium is hard-coded or assumed

| Site | What it fixes | Wire 24 needs |
| --- | --- | --- |
| original_registry.rs:1195–1201 | player-body policies require `ActorOptions::default()` (skill Medium) | admit `Skill::Hard` under wire 24 only |
| player_monsters.rs:280–294 `original_player_episode1_map` | builds with default options | new constructor taking skill (and map number) |
| episode1.rs:85 `from_things` | `options & 2` (medium bit) | skill bit parameter (bit 4 for skill 3) |
| world_pickups.rs:1341–1360 `counted_things` / `counted_map_things` | `options & 2` for `totalkills`/`totalitems` (used by the preflight at world_pickups.rs:1234 and the reader at diagnostic.rs:2065, 2079) | skill bit parameter; older callers keep 2 |
| demon_expanded_map.rs:80–82, demon_lifecycle.rs:132, demon.rs:192 | nightmare refused | unchanged (hard is admitted) |
| spawning.rs:496–503 | the THING filter is already generic in `actor_options.skill` | none |
| monsters.rs:314, pursuit/chase.rs:51, lifecycle.rs:1379 | nightmare-only rules (fast, reaction time 0) | none for hard |
| world_pickups.rs:701 (doc) | "no doubling at the medium skill": ammo doubling is baby/nightmare in C | confirm no code assumes medium; *unverified* that `ammo()` has no skill branch at all |
| snapshot.rs:808–815 | `actors.options` = `[skill, 0, no_monsters]`, already serialised | none |
| diagnostic.rs:1463–1469 `validate_decorations` | requires `actors.options[0] == Medium` for every reader from DSIM36 (v36–39) | DSIM40 admits 2 or 3; v36–39 unchanged |
| diagnostic.rs:425 `from_thing_records` | medium bit inside `from_things` | DSIM40 passes the skill read from `actors.options` |
| context string player_monsters.rs:42 `PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1` | "MEDIUM" in the schema name | a new constant for wire 24; recommend no skill in the name, since skill is canonical in `actors.options` |
| demo.rs:22–30 `DEMO_HEADER` | skill byte 2, episode 1; `parse_demo_episode1` admits only the map byte varying (:131–157) | a wire-24 parser admitting skill 2 or 3 (version still 110) |
| examples/player_level_run.rs:5–26, 123–129, 172–176 | `--policy level\|episode1`; default options | `--policy <wire24>`; skill from the header |
| examples/player_level_advance.rs:15–45, 88 | same | same |
| tools/e1m1_scripts.py:66 `HEADER` (skill 2) | script lumps are always medium | an optional `skill N` directive; the default must keep every committed lump byte-identical |
| tools/seed_player_episode1_fuzz.py:39 `DEMO_HEADER` | skill 2 | a new seeder for the new target |
| tools/reference/m4a_wad.py:549 `spawned(plan, skill=2)` | already parameterised | none |
| tools/reference/level_snapshot.py:240–243 `demo_skill`, :377, :594 | the projector reads `H.gameskill` (`actors.options` = gameskill) | none for skill; a new profile (§4.6) |
| tools/iwad_counts.py:22, 195–197 | the census tool knows `SKILL_BITS`; the M4a tables are medium only | a hard-skill census of E1M5/E1M8/E1M9 (and E1M1–E1M9 for DEMO1's map) |
| C driver (m4a_driver.c:577–582) | skill comes from the demo header via `G_DoPlayDemo`; `H.gameskill` is dumped (:446–448) | none |

What a skill-3 admission needs, in short:

- a skill-carrying constructor and context;
- the option check relaxed for wire 24;
- the medium bit replaced by a skill bit in `Episode1Bounds` and `counted_things`, for wire 24 only;
- the DSIM40 reader admitting `actors.options[0]` ∈ {2, 3} and deriving bounds with that bit;
- a skill-3 demo header parser;
- the runner flag;
- a script-tool directive;
- a hard-skill census.

In single player, C differs between medium and hard only in the THING filter (M4a §6). That must be re-confirmed in the M4b note, for
example that `G_InitNew`'s `fastparm`/`respawnmonsters` are tied to nightmare only.

### 4.5 Demo reader and runner

- **Header validation.** `parse_demo` (demo.rs:116–126) is exact `DEMO_HEADER`. `parse_demo_episode1` (demo.rs:133–157) is `DEMO_HEADER`
  except a map byte in 1..=9 equal to the caller's map. Both check version 110, skill 2, episode 1, deathmatch/respawn/fast/nomonsters 0,
  consoleplayer 0 and `playeringame` [1,0,0,0], byte by byte (`DemoError::Header{index,value}`). The commands (`parse_commands`,
  :161–186) stop at the 0x80 marker and refuse trailing bytes (`TrailingBytes`) and a truncated ticcmd.
- **DEMO1.** The M4a demo census (§6) records 6,854 bytes, version 109, skill 3, E1M5, 1 player, 1,710 ticcmds, "no stray bytes" and every
  command inside the wire-23 command domain. A scratch copy with byte 0 patched 109 → 110 would therefore fail only the skill check today.
  The map check passes with `--map E1M5`, and the policy then refuses E1M5's teleport specials at load.
- **The genuine runner.**
  - m4a_driver.c takes `--wad IWAD --wad DEMO.lmp --demo NAME` (:554–564). It adds the lump with `W_AddFile`'s single-lump naming, the
    upper-cased base name (comment :28–29), and reads skill, episode and map from the header (:41–42).
  - Recommendation: name the scratch file something other than `DEMO1.lmp`, for example `M5DEMO1.lmp`. Otherwise the scratch lump shadows
    the IWAD's own DEMO1. That would still work, because the last lump wins, but it is confusing and the encoder's
    `MapData(.., demo_lumps(path))` adds the same name to its flat directory (level_snapshot.py:236–238).
  - `p_telept.c` and `p_enemy.c` are compiled whole (`WHOLE`, m3h9c_build.py:48–49), so the genuine `EV_Teleport` and `A_BossDeath`
    already run.
- **The Rust runner.** `player_level_run` reads any `--demo` path (examples/player_level_run.rs:158–168); nothing ties it to
  `tests/reference`. The comparator `level_compare.py --demo LMP` (level_compare.py:182–186) and the encoder CLI take any path.
- **Tools that assume committed scripts.** `tools/local/episode1_reference.py`:
  - discovers only `tests/reference/e1mN_scripts/*.lmp` (:63, 168–175) and has `MAPS` without E1M5/E1M8/E1M9 (:54);
  - its `header_map` checks byte 0 == 110 (:149–151) and it records `lmp_sha256` in each result (:204).
  - An external DEMO1 run needs an `--lmp PATH` option (or a new M4b local tool) that accepts a scratch path outside the repository.
  - Whether a SHA-256 of a patched commercial lump may appear in a committed result is an **owner question**. AGENTS.md permits "counts,
    pass/fail facts and state digests".
  - The M4a synthetic tool `m4a_reference.py` is manifest-driven over `tests/reference/m4a_scripts` (:5–10) and is frozen M4a evidence.
    M4b needs its own manifest and results.
- **End conditions.** Both sides support three ends:

  | End | C driver | Rust |
  | --- | --- | --- |
  | Marker | `end=marker` (G_CheckDemoStatus → stub `I_Quit` longjmp; m4a_driver.c:44–48, 593–596) | the runner stops after the last command |
  | Exit | `end=exit` (gameaction `ga_completed`/`ga_victory`/`ga_worlddone` after the dump; :49–58, 606–609) | the next tic is `SimError::LevelExited` (lib.rs:1284–1286) |
  | Reborn | `end=reborn` (a player `PST_REBORN`; :59–66, 610–612) | `SimError::PlayerReborn` (lib.rs:1287–1289; set at demon_expanded_map.rs:905–908) |

  The comparator treats the exit and reborn ends as complete when Rust refuses the following tic (level_compare.py:33–37).
  - A player who dies and never presses use continues to the marker on both sides: death is admitted (`P_DeathThink`).
  - E1M8 ends through `A_BossDeath`'s floor plus sector 11's `G_ExitLevel` (health ≤ 10). That is the exit end.
  - How DEMO1 ends on E1M5 is unknown here; it is any of the three.
- **Pistol start.** `G_DoPlayDemo` → `G_InitNew` gives a pistol start, which the runner's `WeaponInventory::default()` matches
  (player_level_run.rs:178).

### 4.6 The DSIM schema

- **Where the version lives.**
  - Rust writer: `capture` (snapshot.rs:702ff), version selector :729–763 (39 at :732–733, newest predicate first). Range gates: 14
    `..=39` ranges and 5 `version == 39` tests (snapshot.rs:776–1876).
  - Rust reader: `Diagnostic::decode_episode1` → `decode_selected(.., Some(39))` (diagnostic.rs:303–308); `policy()` (diagnostic.rs:498–500).
    The reader has **44 `..=39` ranges and 41 `version == 39` tests**, plus `Some(39)` at :307. Each `== 39` must become a v39-or-v40
    test (a helper such as `episode1_layout(v)` is cleaner), and each `..=39` widen to `..=40`. That is about 105 edits that must not
    change a v≤39 answer.
  - Python encoder: `level_snapshot.py` `VERSION = 38` (:147), the `Profile` class (:181–189) and `EPISODE1 = Profile('episode1', 39, 23, b'PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1', 1024, ...)`
    (:201), `PROFILES` (:202), the CLI `--profile` (:1235–1238); level_compare.py `--profile` (:186).
- **Per actor, DSIM39 serialises** (snapshot.rs:876–1000, `life_fields` :1614–1652):
  - store fields: `generation`, `retired`, `occupied`, `pending_removal`, `previous`, `next`;
  - position: `x`, `y`, `z`, `momx`, `momy`, `radius`, `height`, `floor`, `ceiling`, `angle`;
  - `lastlook`, `flags`;
  - life fields: `type`, `state`, `tics`, `health`, **`reaction_time`**, `sprite`, `frame`, `target`, `death_requested`, `spawnpoint`;
  - **`threshold`**, `movedir`, `movecount`, `player`, `momz`, `subsector`, `sector`.

  `tracer` is **not** encoded. The encoder refuses any actor with a non-NULL tracer (level_snapshot.py:1020). Neither the Baron shot nor
  teleport uses `tracer`. So teleport (angle, position, momentum zeroed, z, reaction time 18) and the Baron need **no new per-actor field**.
  Types are `u16` ordinals, so new types need no widening.
- **Per thinker:** `thinker.order` and `thinker.shared_order` (snapshot.rs:855–873); sector-thinker kinds 1–7 and kind 8 for a removed mover
  (snapshot.rs:1300–1430). No new thinker kind (the boss floor is a kind-7 `lowerFloorToLowest`).
- **Per sector:** `floor`, `ceiling`, `actors`, `soundtarget`, `light`, `special`, and (v39) `floorpic` (snapshot.rs:1030–1084).
- **Player:** the `original_player.0.*` block including weapons, powers, cards and backpack (snapshot.rs:1129–1948); `player.{i}` commands.
- **What DSIM40 must add.**
  1. **The map number.** For example `level.map` (episode, map) or a field next to `actors.options`, because `A_BossDeath`'s outcome
     depends on `gamemap`, so it is canonical state. The driver has no `gamemap` column in `H`. The encoder can take it from the demo
     header bytes 2–3 (constant for a run), so no driver change is needed for it.
  2. **Skill 3 admitted in `actors.options`** (layout unchanged).
  3. **The version word 40 and the new context.**
  4. **Reader domains:**
     - type roles through the registry (automatic once `policy()` maps 40 to wire 24);
     - the projectile event type check (diagnostic.rs:2025–2029) with `MT_BRUISERSHOT`;
     - the DSIM39-only state lists (diagnostic.rs:845–870) extended with `MT_BRUISER`, `MT_BRUISERSHOT`, `MT_TFOG` and `MT_TELEPORTMAN`;
     - health below −9,000 after a telefrag (verify);
     - the bounds from `from_thing_records` with the Baron and the skill bit.
  5. **Events (stripped by the comparator, but encoded and read):** a teleport event and a telefrag damage event. The new event tags must
     follow the existing per-tag validators (diagnostic.rs:~700–810).
- **Does the C driver need a fork?** Probably **no new dump columns**.
  - `print_mobj` already writes every mobj field the projection reads: type, state (S_NULL → 0), reactiontime, threshold, tracer, flags
    and the spawn point (m4a_driver.c:381, 424–436).
  - The boss floor is an `FL` row (:468); `H.gameskill` is dumped (:446–448); TFOG and TELEPORTMAN are ordinary `M` rows; there is no
    new message string (`message_index` :298–311).
  - The m3h9c and m4a driver, contract and result files are frozen. If a new column turns out to be needed, it goes in a new fork
    (`m4b_driver.c` + `m4b_build.py` + its own contract), exactly as M4a forked m3h9c. *Unverified until the M4b C phase confirms that
    the encoder can project TFOG and TELEPORTMAN rows, especially the `S_NULL` state and `NOSECTOR` (`sector_pos` returns −1,
    m4a_driver.c:416–421).*
  - The encoder needs a new `Profile` for DSIM40 (version 40, policy 24, new context). Its type and state domains, the map number field
    and possibly a telefrag health domain must be added *after* the frozen v38/v39 profiles. The per-tag domain sets live in
    `Projector.episode1_*` and helpers (:1020–1100).

### 4.7 Frozen-wire hazards

- **Shared functions wire 24 would be tempted to change, which older wires also run:**
  - `line_triggers::cross` and `crossings` (line_triggers.rs:110–219), `movement::Mover::attempt` (movement.rs:586–657) and
    `MotionAccess` (:516–555). Every policy since the movers wire runs them. A teleport hook must be a no-op default, like `contacts`.
  - `pursuit::move_actor` and `cross_lines` (pursuit.rs:164–289): every monster policy.
  - `DamagePlan::current_original_in` and `apply_damage` (combat.rs:415–560, 953–1010): the 1,000 bound and the `i32` thrust expression.
    Changing the bound or the arithmetic for all policies changes refusal answers for wires ≤ 23. Key a widened bound on the new predicate.
  - `projectiles::launch_fireball` (projectiles.rs:550–589): parameterising the type must keep the Imp's event bytes and draw order.
  - `preflight_live_missiles` (projectiles.rs:854–873): the `FIREBALL` count is the wire 12–22 schema bound (208 or 64). Leave it.
  - `is_missile_type`, `is_extended_monster`, `is_policy_monster` (original_registry.rs:806–831) are policy-free helpers used by the
    readers of older versions. Adding the Baron shot to `is_missile_type` without a role gate would change v29–v39 reader answers on a
    crafted capture that names that type.
  - `Episode1Bounds` constants (episode1.rs:20–39) and `counted_things` (world_pickups.rs:1347–1360): these feed the wire-23 runtime and
    the DSIM39 reader.
  - `parse_demo_episode1` and `DEMO_HEADER` (demo.rs).
  - `mover_rules::validate_movers_with` (Rules::Episode1 at :427).
  - `validate_sector_specials_for`.
  - `state_role_for` and `require_role_for` (original_registry.rs:1022–1140), which run every tic for every policy (below).
- **Table extensions that would shift indices or change answers.**
  - `OriginalAction` (actions.rs:20–50) is a compile-time name-to-ordinal map. Appending variants is safe, but inserting mid-list is
    harmless only if nothing serialises the variant index. *Unverified*: grep shows it is used only for dispatch. Append anyway.
  - The pickup-message codes: DSIM39 codes 14–30 (level_snapshot.py:197–201; the reader at diagnostic.rs:950). New codes, if any, must be
    appended for v40 only. M4b needs none.
  - Event tags: append, never renumber.
  - The sector-thinker kind codes: 8 is reserved for a removed mover (M4a §4.5).
  - `EPISODE1_MONSTERS` gaining 3003 would change `Episode1Bounds` (K, the sight allowance, the visit roster) for any wire-23 map containing
    a Baron. Such a map is refused at spawn under wire 23, but the DSIM39 reader recomputes the bounds from `map.things`, so its answer on
    a crafted capture would change.
- **The shared-validation slowdown pattern to avoid.** docs/validation/M4A.md:406–418 records that wire 23 extended validation shared with
  older policies:
  - `require_role_for` (+24.27M self Ir in dense64_fight);
  - `OriginalActorType::descriptor` (+18.08M);
  - `decoration_states` building a `Vec` (+11.38M with malloc/free);
  - `PlayerWeaponState::validate_for`.

  Older policies preserved their answers at a per-tic cost. New role arms for four types in `require_role_for`/`state_role_for` repeat
  that pattern. Put the wire-24 arms behind a single early `has_episode1_complete()` test placed *after* the older policies' fast paths.
  Avoid allocation in per-tic validation (no `Vec`s; static or `OnceLock` tables, as `fireball_policy` does). Re-run the matched
  Callgrind workloads.
- **Fixtures that pin older bytes.** `tests/reference` has 6,323 entries. Among them are the M3h9c level goldens
  (`m3h9c_level_*.bin`), the M4a DSIM39 captures (`m4a_episode1_*.bin`, 18 files), `m4a_manifest.json`, `m4a_runner_contract.json`,
  `m4a_synthetic_goldens.json`, `m4a_synthetic_results.json`, `m4a_wads/`, `m4a_scripts/` and `e1m{2,3,4,6,7}_scripts/`. The tests that
  read them are crates/doom-sim/tests/episode1_mechanism_goldens.rs (405 lines), episode1_reader_domains.rs (1,280, which decodes every
  committed DSIM39 fixture) and episode1_synthetic.rs (145), level_fixtures.rs and the m3*/ reference tests.
  - **No existing file under `tests/reference` may change.** M4b adds new files only: its own WAD generator output, scripts, manifest,
    goldens and contract if forked.
  - `m4a_wad.py` writes `tests/reference/m4a_wads`, so new mechanism WADs need a new generator (or a new output set). The M4a WAD bytes must
    stay identical under the generator's `--check`.
  - The same applies to `tools/e1m1_scripts.py`: an added `skill` directive must leave every committed lump byte-identical.

### 4.8 Size estimate

Measured M4a pieces (`git show --numstat`; "src" is crates/ outside test files, "tests" is `*_tests.rs` and `crates/*/tests/`):

| M4a commit | What | Rust src | Rust tests | tools | tests/reference |
| --- | --- | ---: | ---: | ---: | ---: |
| 7783839 | wire 23 plumbing, DSIM39, per-map admission (27 files) | 1,342 | 964 | — | — |
| 3e91625 | every Episode 1 door/floor/plat/stair/donut/light/exit special | 1,195 | 1,027 | — | — |
| 38471c9 | keys, locked doors, items, powers, sector special 11 | 846 | 463 | — | — |
| 191b8fe | chainsaw, rocket launcher, generalised missiles, Spectre | 748 | 335 | — | — |
| 9f5c86b | vanilla-range commands, Episode 1 demo header (demo.rs +235, runner +62) | 305 | 297 | — | — |
| 965b7a6 | real-run fixes | 55 | 67 | — | — |
| 9e323ca | independent DSIM39 projection, goldens | — | — | 773 | 14,859 |
| 2a27e58 | synthetic mechanism WADs, scripts, manifest, reference run | — | — | 1,392 | 2,914 |
| 71f1cb7 | driver fork (m4a_driver.c 621, m4a_build.py 58) and contract | — | — | 798 | 1,352 |
| 3122fee | fuzz target and seeder | 294 (fuzz) | — | 155 | — |
| 5fd50d6 | local Episode 1 runner and real-map scripts | — | — | 229 | 10,024 |

Within 191b8fe, the registry, projectiles and thinker files together add 143 lines. That covers the rocket and Spectre policies, the missile
generalisation and the dispatch, so the Spectre alone is a few dozen lines.

Estimate per M4b item (lines added; ranges, because the teleport restructure has no M4a analogue):

| M4b item | Rust src | Rust tests | Python/C tools | Basis |
| --- | ---: | ---: | ---: | --- |
| Wire 24 / DSIM40 plumbing (enum, ~20 predicate lists, registry arms, context, constructor with skill and map number, about 105 reader and 19 writer ladder edits, `level.map` field) | 600–900 | 400–700 | 150–300 (encoder profile, comparator) | 7783839 without its per-map-bound and certificate work |
| Teleport (97: `EV_Teleport`, `P_TeleportMove`, `PIT_StompThing`, two TFOG, 18-tic freeze, in-motion hook for player, monster and sliding things, crossing-loop abort, split-move continuation, destination role with `S_NULL` state, load rule, defence flag) | 700–1,200 | 500–900 | 300–600 (synthetic WADs and scripts for teleport, monster teleport, telefrag, blocked monster teleport) | No analogue; larger than any one M4a mechanism because it restructures the deferred crossing model |
| Telefrag damage (10000 bound under wire 24, the declared wrapping thrust, health domain, drops) | 100–200 | 150–250 | included above | combat.rs bound sites (5) |
| Baron (role, closure, `A_BruisAttack`, `MT_BRUISERSHOT` via a type-parameterised `launch_fireball`, reader type checks, bounds K) | 150–300 | 150–300 | 50–150 | Spectre/rocket share of 191b8fe |
| `A_BossDeath` (dispatch, living-Baron scan, tag-666 floor starter, map-number gate, reachable rule for tag 666 on map 8) | 120–250 | 150–250 | 100–200 (E1M8-slot synthetic WAD) | sector-11 share of 38471c9 |
| Hard skill (option check, skill bit in bounds and counts, reader, demo header, runner flag) | 100–200 | 150–250 | 30–80 (script directive, seeder) | 9f5c86b |
| MAXSPECIALCROSS witness (already a wire-23 per-tic refusal; WAD plus genuine run plus test) | 0–30 | 50–150 | 50–150 | — |
| E1M5, E1M8, E1M9 real-map runs (fixes found during runs) | 50–200 | 50–150 | 50–150 (local tool maps, `--lmp`) | 965b7a6, 5fd50d6 |
| DEMO1 replay (external path, version patch in scratch, compare) | 0–50 | 0–50 | 100–200 | — |
| Fuzz target and seeder | 250–300 (fuzz) | — | 150 | 3122fee |
| **Total (code and tests, excluding docs and `tests/reference`)** | **≈ 2.1k–3.6k** | **≈ 1.6k–3.0k** | **≈ 1.0k–2.0k** | |

So code, tests and tools total roughly **4.7k–8.6k lines**. M4a's step 6 size check measured 12.8k lines of code, tools and docs, and
39k lines under `tests/reference`. M4b's goldens and scripts would add on the order of 15–30k lines under `tests/reference`, depending on
how many synthetic scripts and committed captures are kept; that figure is an extrapolation, not a measurement. The M4a note's own forecast
for M4b was 8–12k lines (M4A_REFERENCE_INVESTIGATION.md:1704). These figures are consistent with it once docs are included. If the in-motion
teleport hook cannot reuse the `MotionAccess`/`Contacts` pattern and the movement pass must be restructured, the teleport row could double.

## 5. Owner decisions (I1–I13) with recommendations

Each is tested against four questions: (a) consistent with prior owner decisions (E1–E9, F1–F11, G1–G13, H1–H12)? (b) no silent divergence? (c) no new kind of fidelity gap beyond refuse-the-tic witnesses? (d) bounded stores?

**I1. Scope.** M4b ports teleport line special 97 (the only teleport special Episode 1 uses; 39, 125 and 126 stay unadmitted), the landing thing (doomednum 14), the teleport fog, telefrags, the Baron of Hell and its fireball, `A_BossDeath` for E1M8 (tag 666), hard skill, and with them E1M5, E1M8 and E1M9, then replays `DOOM.WAD` `DEMO1`. Pause, easy skill, nightmare, the reborn reload and level transitions stay later work. (a)–(d) yes. **Recommendation:** as stated. Confirmed by the owner on 2026-11-10.

**I2. Policy identity.** A new policy `PlayerMonstersEpisode1V2`, wire 24, DSIM40, a strict superset of wire 23's behaviour. Its constructor takes the map number (1–9) and the skill (2 or 3), because `A_BossDeath` tests the map and the THING filter tests the skill; DSIM40 records both. Every new behaviour gets its own predicate, so wires ≤23 cannot change a byte. (a)–(d) yes. **Recommendation:** as stated. Confirmed by the owner on 2026-11-10.

**I3. The telefrag thrust overflow.** `P_DamageMobj` computes `damage*(FRACUNIT>>3)*100/mass` in `int`; for a telefrag's 10,000 this overflows. The genuine `strict` (LP64) and `strict32` builds both wrap to −397,934,592 before the division (observed in a probe); the `overflow` and `address` variants abort there. Two ways to handle it: Confirmed by the owner on 2026-11-10.
- (i) a declared adapter in the `overflow` and `address` variants only, computing the same product in unsigned arithmetic, plus a Rust wrapping rule for exactly this product under wire 24 only. `strict` and `strict32` stay genuine text and anchor the value.
- (ii) a refuse-the-tic witness, which would make every player telefrag unmatchable, and E1M5's and DEMO1's teleporters only work while nothing stands on a landing.

(a) (i) is an exception to H3's checked-arithmetic wording, but the M4a note already assigned this adapter to M4b, and the value is what both genuine unsanitised builds produce; (b) yes; (c) yes: no gap at all under (i); (d) yes. **Recommendation:** (i). Damage above 1,000 stays refused everywhere except the stomp call's 10,000 under wire 24.

**I4. Hard skill.** Wire 24 admits skill 2 and skill 3. The source shows the THING filter (option bit 4 instead of bit 2) is the only single-player difference between them; per-map bounds are derived for the admitted skill. Skills 1, 4 and 5 are refused. The demo reader accepts skill byte 3 only under wire 24. (a)–(d) yes. **Recommendation:** as stated. Confirmed by the owner on 2026-11-10.

**I5. DEMO1.** `DEMO1` is version 109; linuxdoom-1.10 plays only 110. The lump is extracted to scratch with byte 0 patched to 110 and run through the genuine runner and the Rust runner, compared tic for tic. The Rust reader keeps accepting 110 only. Nothing from the demo is committed except its header values, command counts and bounds, the tic count and state digests, plus (owner ruling, 2026-11-10) the SHA-256 of the original unpatched lump as an input-identity digest and, if useful, that of the patched scratch copy. A match proves Rust agrees with linuxdoom-1.10 on that input; it does not prove the run reproduces the DOS 1.9 recording. (a)–(d) yes. **Recommendation:** as stated. Confirmed by the owner on 2026-11-10.

**I6. The ±1,024 sector-height bound stays.** It refuses earlier than vanilla misbehaves; widening it is a later C-evidence follow-up. Step 8 confirms it covers every new mover path (the tag-666 floor). (a)–(d) yes. **Recommendation:** stays. Confirmed by the owner on 2026-11-10.

**I7. Teleport as vanilla runs it, with H3 witnesses.** The teleport runs inside the move, as in `P_TryMove`: it ends the crossing loop, and a split move or slide continues from the landing, so a thing can teleport twice in a tic. Witnesses, each refusing the tic with state preserved: the ninth special line of a move (already wire 23's `SpecialCross`), a full actor store when the two fogs spawn, and the intercept bound (inherited). The landing search is bounded work (sectors × thinkers) preflighted before mutation. If routing the teleport through the sliding path of barrels and corpses proves to need a restructured movement pass, the fallback is a witness that refuses the tic when such a slider crosses a line 97; the note records which was done. (a)–(d) yes. **Recommendation:** faithful port, with the named fallback. Confirmed by the owner on 2026-11-10.

**I8. Stomping.** As vanilla on Episode 1 maps: a player telefrags whatever shootable overlaps the landing; a monster's or other thing's teleport fails if any shootable overlaps. No z test, as in C. (a)–(d) yes. **Recommendation:** as stated. Confirmed by the owner on 2026-11-10.

**I9. `A_BossDeath` and the E1M8 ending.** Under wire 24 the action runs its C checks in order; only E1M8 with a Baron acts (lower the tag-666 floor to the lowest neighbour). On other Episode 1 maps it returns, as in C. E1M8 then ends through sector special 11, already ported. As in M4a, the tic on which the level ends completes and every later tic is refused with `LevelExited`. (a)–(d) yes. **Recommendation:** as stated. Confirmed by the owner on 2026-11-10.

**I10. CI evidence.** A new generator `tools/reference/m4b_wad.py` (the M4a generator stays frozen for its WADs) writes synthetic WADs for teleport, telefrag, blocked landing, monster teleport, Baron melee, missile and infighting (including Baron against Baron through a barrel), `A_BossDeath`, the ninth spechit and the skill-3 filter; a new manifest drives genuine runs in the strict, overflow and address variants, goldens from an independent DSIM40 projection, and the self-test. The driver is forked to `m4b_driver.c` only if a new dump column or the I3 adapter needs it; M3h9c and M4a driver, contract and result files are never edited. (a)–(d) yes. **Recommendation:** as stated. Confirmed by the owner on 2026-11-10.

**I11. Real-map evidence.** Five scripts (idle, walk_to_exit, fighting, chaos, death) for each of E1M5, E1M8 and E1M9 on skill 2; all 30 real-map scripts of E1M1-E1M4, E1M6 and E1M7 (25 M4a real-map scripts plus the five E1M1 scripts; not the 30 synthetic M4a scripts, which `check_player_episode1v2.py` gates) re-run under wire 24 and must match unchanged (reconciled 2026-11-13, [evidence record](validation/M4B.md#real-maps-and-demo1)); skill-3 runs on at least the three new maps; then DEMO1. Dumps are deleted once digests are extracted. (a)–(d) yes. **Recommendation:** as stated. Confirmed by the owner on 2026-11-10.

**I12. Mutation testing in CI.** A workflow that is not a required check runs cargo-mutants `--in-diff` against master in 8 shards. GitHub only offers `workflow_dispatch` for a workflow file that exists on the default branch, and this branch must not touch master, so the workflow also triggers when the label `run-mutants` is put on the pull request; that is how it is triggered before merge. (a)–(d) yes. **Recommendation:** `workflow_dispatch` plus the label trigger. Confirmed by the owner on 2026-11-10.

**I13. Scratch on D:.** On 2026-11-10 the owner directed scratch to `D:\` because `/mnt/c` had 15 GB free, below the 20 GB floor. Scratch lives in `/mnt/d/scratch/m4b`; `/mnt/c` is still checked after every step and the work stops if it falls further. Recorded as given, not a proposal. Confirmed by the owner on 2026-11-10.

**Unattended-rule verdict.** Every decision above is consistent with prior decisions, has one clear recommendation and admits no silent divergence: each gap is a refuse-the-tic witness, and I3's adapter reproduces what both genuine unsanitised builds compute. The owner confirmed all thirteen decisions on 2026-11-10 (I2's name `PlayerMonstersEpisode1V2` and I3's adapter included); Phase 2 proceeds on them.
## 6. Plan, size estimate and cap

### 6.1 Steps

The steps are the sixteen of the milestone brief, in order: synthetic WADs and genuine runs; the DSIM40 projection and goldens; wire 24 plumbing; teleportation; the Baron, then `A_BossDeath`; hard skill; real runs and DEMO1; carry-ins (the open mutant `barrels.rs:362:19`, the height bound on new mover paths); tests, rejections, rollback, soaks and a fuzz target; docs; mutation testing in CI; fuzz campaigns; matched timings; independent review; full gates; the PR.

### 6.2 Size

From the inventory (§4.8): Rust source 2.1k–3.6k lines, Rust tests 1.6k–3.0k, Python and C tools 1.0k–2.0k, docs about 2k–3k: **about 7k–12k lines** against the 30k cap, which applies to code, tools and docs. Generated data verified by `--check` tools (goldens, scripts, manifests, contracts) is reported separately, as in M4a's §14 ruling; the extrapolation is 15k–30k lines.

### 6.3 Fallback

If the projection passes the cap (most likely because teleport forces a restructured movement pass), hard skill and DEMO1 move to a follow-up milestone and M4b closes with teleporters, the Baron, `A_BossDeath` and the three maps on medium skill.
## 7. Corrections and findings from Phase 2

Findings of Phase 2 step 1 (2026-11-10, America/Los_Angeles), observed in the genuine engine on the synthetic WADs of
`tools/reference/m4b_wad.py` (strict, overflow and address variants, and strict32 locally, all byte-identical); the per-scene
observations are in [the evidence record](validation/M4B.md#synthetic-mechanism-wads-ci-evidence).

1. **Shadow-fuzz draw order (§3 hazard 1, new observation).** For `A_FaceTarget` at a `MF_SHADOW` target
   (linuxdoom-1.10/p_enemy.c:794–795), the Baron's angle after the call matched `(r1 − r2) << 21`, r1 being the first of
   the two consecutive draws, at all four calls observed (BINF1); never `(r2 − r1)`. GCC 15.2.0 evaluates the left
   `P_Random()` first in every variant, `-m32` included. The `P_SpawnMissile` pair (p_mobj.c:909–910) was not reached in
   these scenes.
2. **Baron against Baron through barrel splash (§3 fact 16, hazard 9): observed.** In BVB1 a Baron fireball wounds a barrel
   (target that Baron), the next kills it, the splash (source = the barrel's target, p_enemy.c:1598–1601) wounds the
   other Baron, which retargets the first; their fireballs then burst on each other without damage (p_map.c:299–313) and
   their claws do damage. New detail seen while authoring: a monster hurt while in a standing state other than its
   spawn state keeps the new target but does not wake, because `P_DamageMobj` enters `seestate` only from `spawnstate`
   (p_inter.c:904–914); one authoring run left the second Baron standing with the first as its target.
3. **Telefrag (§2 T8): confirmed.** The victims' momenta in the telefrag tic are (−3,979,285, −1,518), (−994,821, −380)
   and (−397,928, −152) for masses 100, 400 and 1,000, the wrapped thrusts of T8.5 applied through `FixedMul`, pointing
   towards the player's pre-teleport position (correction 4 of §2). The draws in the tic are 4 (zombieman: death tics,
   clip, two fogs) and 3 (Demon, Baron), as T10.2. Each corpse's momentum is 0 one tic later: its move is blocked by the
   overlapping player (T8.8).
4. **Split and slide continuation (§2 T6.2–T6.3): reachable and observed.** The command domain reaches the split: at
   forwardmove 127 the player's momentum before the move is 27.19 units/tic, the first half crosses the line, and the
   second half continues from the landing (x = landing + 15) and picks up an item there in the same tic. A slide whose
   move up to the wall (p_map.c:762) teleports runs the zero-length try at the landing (p_map.c:784), which also picks up
   an item in the same tic. After an ordinary teleport an item on the landing is picked up only when the player next
   moves (`P_TeleportMove` touches only shootable things, p_map.c:85–86).
5. **The ninth `spechit` (§2 T9.1): observed in the address variant.** The report comes from the bounds check of
   `-fsanitize=undefined`, `p_map.c:242:9: runtime error: index 8 out of bounds for type 'line_t *[8]'`, before ASan's
   redzone, in the tic after the last dump. The strict and overflow variants were not run past that tic.
6. **`A_BossDeath` and the busy sector (§3 facts 19–23): confirmed.** On E1M8 the first Baron's DIE7 with the other alive
   does nothing; the second's DIE7 starts `lowerFloorToLowest` (type 1) in the same tic's dump. With the tag-666 sector
   already moving (a W1 raiseFloor crossed 8 tics earlier) nothing is started. In the E1M1 slot nothing happens. DIE7
   came 48 tics after each telefrag tic here (kill tic 64 → 112, 175 → 223): the telefrag is made by the player's thinker
   after the Baron's (the Barons precede the player in THINGS), so this is fact 19's later case with `t1` = 8.

Findings of Phase 2 step 2 (2026-11-10), from projecting the step-1 dumps:

7. **The frozen DSIM39 projection already accepts every M4b dump.** `level_snapshot.py`'s
   `episode1` profile has no type or state domain. Projected unchanged, it encodes the
   Baron, the fireball, both fogs and the landings without complaint. The DSIM40 module
   therefore adds its own refusals:
   - the driver's exact row kinds and columns;
   - a single-player level;
   - every actor state inside its type's normal closure.

   One state outside every info.c closure is reached in ordinary play: the player's
   `S_PLAY_ATK2`, which `A_GunFlash`, `A_FirePistol`, `A_FireShotgun`, `A_FireShotgun2` and
   `A_FireCGun` set by name (p_pspr.c:453, 653, 676, 706, 742). It is admitted explicitly.
   With it, the 30 M4a scripts (15,116 snapshots, locally) and the 18 M4b scripts project
   with no stop.
8. **The landing in the canonical orders.** A live `MT_TELEPORTMAN` is state 0 (`S_NULL` as an
   index, distinct from the removal sentinel). It appears in `spatial.link_order`, because
   `P_SpawnMobj` ran `P_SetThingPosition` for it (p_mobj.c:517, link serials in the dumps). It
   appears in no sector thing list, because of NOSECTOR. A teleport relinks the thing ahead
   of the two fogs that are spawned after it.
9. **Telefrag health domain (§4.6 "verify").** A telefragged monster's health lies between
   −9,999 and its spawn health minus 10,000. The observed values were −9,980, −9,850 and
   −9,000. Only the player stomps on maps 1–9 (p_map.c:102). No knockback applies when the
   player's ready weapon is the chainsaw (p_inter.c:806–810); this was not exercised by the
   synthetic scripts.

Findings of Phase 2 step 3 (2026-11-10), from wiring wire 24:

10. **The DSIM40 reader admits skill 2 only until step 6.** STATE_FORMAT's DSIM40 section
    describes the finished format: `actors.options` is `[2, 0, 0]` or `[3, 0, 0]`. The v40
    reader of step 3 keeps the DSIM36-39 check (skill 2), because the per-map bounds for
    skill 3 (option bit 4) arrive with hard skill in step 6. This is not an ambiguity in the
    format. Step 6 widens the check and derives the bounds with the skill bit.
    **Closed in step 6 (92b6369, 2026-11-10):** the v40 reader admits `actors.options[0]` 2 or 3,
    refuses other values with `Identity`, and derives the bounds and totals with that skill's bit.
11. **The demo reader admits skill 3 already.** `parse_demo_episode1_v2` admits skill bytes 2
    and 3 (decision I4). The wire-24 constructor refuses `Skill::Hard`
    (`PlayerBodyError::Unsupported`, the error older policies give for non-default options),
    and `player_level_run --policy episode1v2` reports a skill-3 demo before any tic. Step 6
    changes only the constructor and the runner.
    **Confirmed in step 6:** the constructor and the runners changed, the reader did not.
12. **Real E1M1 dumps cannot be projected to DSIM40 as they are.** `tools/local/episode1_reference.py`
    runs E1M1 with the frozen M3h9c driver, whose dumps have no `H.secretexit` and no `R fn
    sector` columns. `level_snapshot_m4b.py` refuses any dump outside the M4a driver contract
    (`DumpDeficiency`), so the local sanity run of step 3 used E1M2 (M4a driver). Step 7's
    re-run of the five E1M1 scripts needs those runs on the M4a or M4b driver, as a local-tool
    option, without editing the frozen M3h9c files.

Findings of Phase 2 step 4 (2026-11-10), from porting teleportation under wire 24 (C facts re-read in
linuxdoom-1.10 before each piece):

13. **The ninth-`spechit` witness covers every `P_CheckPosition` caller that wire 24 reaches.** `spechit`
    is computed and its ninth entry refused (`MapScaleError::SpecialCross`) in `movement::Mover::attempt`
    (every `P_TryMove` of `P_XYMovement`: halved steps, the move up to the wall, the projected slide and
    the stairstep tries, p_mobj.c:147–191, p_map.c:750–784), `pursuit::move_actor` (`P_Move`,
    p_enemy.c:293–334), `planes` `P_ThingHeightClip` (p_map.c:536, from `P_ChangeSector`; the result is
    otherwise ignored, as in C) and `projectiles::try_move` (missile flight and `P_CheckMissileSpawn`,
    p_mobj.c:880). `G_CheckSpot` (g_game.c:868) is not reached (no reborn, no deathmatch). `P_TeleportMove`
    appends nothing: it has no line pass and sets `numspechit = 0` (p_map.c:152). This closes the open item
    on the `P_ThingHeightClip` path. SPECH9 shows the refusal on the tic where the genuine engine writes the
    ninth entry (dump tic 75, DSIM tic 76).
14. **`movement::for_things`' window equals `P_TeleportMove`'s.** Both take the cells of
    `(dest ± (radius + MAXRADIUS) − bmaporg) >> MAPBLOCKSHIFT` (p_map.c:155–158; `MAXRADIUS` 32 units,
    p_local.h:51), `bx` outer and `by` inner, each cell in most-recent-link order, and skip cells outside
    the blockmap as `P_BlockThingsIterator` returns true for them (p_maputl.c:512–537). The stomp reuses it.
    This closes the open item of §4.2.3.
15. **The crossing loop runs inside the move under wire 24, for every special.** Teleport ordering forces
    it: a crossing visited before a 97 line in the same `P_TryMove` loop (or in an earlier attempt of the
    same `P_XYMovement`) starts its mover before `EV_Teleport` spawns its fogs, and both are appended to the
    shared thinker order (`P_AddThinker`, p_tick.c:65–71). Wire 24's motion access therefore runs each
    crossing at its place in the loop; the events land in the same order as wire 23's deferred processing,
    and a W1 clear is still staged after the motion (later attempts read it through the motion's `cleared`
    list, as before). Wire 23 keeps the deferred model unchanged. No `EV_*` starter reads a line special,
    so the staging point is not observable.
16. **The voodoo-doll case of §2 T3.9 cannot arise.** The level policy (inherited by wire 24) refuses a
    second type-1 start at spawn (`PlayerBodyError::Binding`), so the only player-linked mobj is the body
    and a telefrag victim is never a player. The stomp's damage plan refuses a player victim explicitly.
17. **`P_SlideMove` after a teleport in the move up to the wall reads the zeroed momentum.**
    `tmxmove = FixedMul (mo->momx, bestslidefrac)` (p_map.c:776–777) reads `momx` after the `P_TryMove` at
    762, so the slide's remainder is 0 and its `P_TryMove` at 784 is the zero-length try at the landing.
    The Rust slide read the momentum captured before the move; it now re-reads it, which changes nothing
    for older wires (no attempt changes momentum there). TSLIDE shows the zero-length try's pickup.

Findings of Phase 2 step 5 (2026-11-10), from porting the Baron of Hell and `A_BossDeath` under wire 24 (each C
fact re-read in linuxdoom-1.10 before the code that relies on it):

18. **The `P_SpawnMissile` shadow pair is observed (closes finding 1's open half).** It is not in any M4b scene, but the
    M4a script INVIS1 reaches it: an Imp fires at the invisible player (`MF_SHADOW`) at dump tics 173 and 254. A local
    re-run of the genuine engine (`m4a_reference.py --only ITEMS --keep-dumps`, strict, overflow and address variants
    byte-identical, `ASAN_OPTIONS=detect_leaks=0`) made six draws in each of those tics: the `A_FaceTarget` pair,
    `lastlook`, the `P_SpawnMissile` pair and the spawn tics (p_enemy.c:794–795; p_mobj.c:506, 909–910, 870). With the
    Imp's own dumped angle giving `R_PointToAngle2` back, the fireball's dumped angle equals `base + ((r1 − r2) << 20)`,
    r1 the first draw of the pair, at both tics (2,471,483,828 and 2,101,157,081); `(r2 − r1)` gives 1,949,292,980 and
    2,143,100,121. GCC 15.2.0 evaluates the left `P_Random()` first here too. `P_SpawnMissile` reads the missile type
    only through its info record (p_mobj.c:889–927), so the Baron's ball takes the same pair; the Rust launcher is one
    function for both types (`projectiles::launch_missile`), and Rust matched INVIS1 in M4a. No witness is needed.
19. **`A_BruisAttack` (p_enemy.c:979–996) confirmed as §3 fact 4**: no `A_FaceTarget`; the melee test is
    `P_CheckMeleeRange` (no draw); the claw is one draw `(P_Random()%8+1)*10` with inflictor and source the Baron; the
    missile is `P_SpawnMissile (actor, actor->target, MT_BRUISERSHOT)`. PIT_CheckThing's species rule (p_map.c:299–313)
    is the one already ported, Knight/Baron clause included; the Rust test against `MT_PLAYER` is type 0.
20. **`A_BossDeath`'s live-player test reads `players[i].health`** (p_enemy.c:1683–1688). `P_DamageMobj` mirrors the
    damage into it and clamps it at 0 (p_inter.c:870–872), and every pickup copies it into the mobj (p_inter.c:239,
    387), so `players[i].health > 0` and the body's `health > 0` agree; the port reads the player record.
21. **The reachable-height fixpoint needs no boss-floor rule (correction to §4.1.4).** `lowerFloorToLowest` moves a
    floor down to `P_FindLowestFloorSurrounding` (p_floor.c:299–305; p_spec.c:270–288), the minimum of current floors
    of the sector and its neighbours. Every such value already lies inside the fixpoint's `[R_min, R_max]`, and a
    lowering floor adds nothing to `Δ` (a fitting thing never sinks into a floor that moves away from it). So the tag-666
    floor changes neither bound, and wire 24 keeps wire 23's derivation.
22. **The ±1,024 sector-height witness (decision I6) covers the boss floor, and cannot fire on it.** The boss floor is an
    ordinary `T_MoveFloor` step (`PlaneHost::set`, the same checked write as every mover). Its destination is a
    current floor, inside the domain by the same witness, and the floor moves monotonically toward it by 1 unit per tic
    (`FLOORSPEED`, p_spec.h:600), stopping on it (p_floor.c:66–79). So no boss-floor step can leave the domain: the
    refusal is not constructible on this path, and no test constructs it.
23. **Per-map bounds under wire 24.** `K` counts doomednum 3003 as a monster that drops nothing (`P_KillMobj`'s drop
    switch names no Baron, p_inter.c:737–754): `Episode1Bounds::from_map_v2`, and the DSIM40 reader derives the same.
    Wire 23 and the DSIM39 reader keep `from_map`, unchanged.
24. **DIE7 timing and the outcomes, as the C dumps show them.** Rust reproduces every C-observed `A_BossDeath`: BOSS1 and
    BOSSM1 run it at dump tics 112 and 223 (outcome 2 then 3 on E1M8; 0 and 0 in the E1M1 slot), BOSS2 at 223 finds
    the tag-666 sector busy (the call is made and creates nothing), BARON1 at 736 on E1M1 (outcome 0) and TFRAG1 at 271
    (0). The floor's first step lands in the same tic (BOSS1 fixture 224).

25. **Two teleports in one tic: observed (owner request).** The second synthetic set's TELE2X (tele2x.wad,
    `tests/reference/m4b_manifest2.json`) makes the second half of a split move (p_mobj.c:147–191), continued from
    the first landing, cross a second 97 line 8 units east of it. Built at the first attempt: in dump tic 36 the
    genuine engine spawned four fogs with four draws and left the player on the second landing (angle 90,
    `reactiontime` 18, momentum 0), identically in the three variants; Rust matches all 66 captures
    ([evidence](validation/M4B.md#second-synthetic-set-two-teleports-in-one-tic)). This closes step 4's open item.
26. **A Demon or Spectre bite on a body `A_Saw` flagged (step 7, the M5WALK3 refusal).** `A_Saw` sets
    `MF_JUSTATTACKED` on a hit (p_pspr.c:542); `P_PlayerThink` runs before the thinkers (p_tick.c:150–152) and clears
    it on the next tic (p_user.c:249–255); nothing on the bite path reads it (p_enemy.c:935–948, 782–796, 174–192;
    p_inter.c:775–918). Rust's `strike_player` refused such a bite with `PlayerBody(Invariant)`. Wire 24 admits it
    (2dd986c), reproduced by SAWBITE ([evidence](validation/M4B.md#the-m5walk3-divergence-and-its-fix)). Wire 23 admits
    the chainsaw and Demons too, so the same tic can arise there; it is a **known wire-23 gap, left unpatched by rule**
    (historical profiles keep their behaviour; it refuses explicitly and never differs silently), listed in
    [COMPATIBILITY](COMPATIBILITY.md#explicitly-not-admitted). Wires below 23 do not admit the chainsaw.
27. **The `barrels.rs:362:19` mutant is equivalent (step 8, 61c8379; closes M4A B7).** The explosion walk is a series of
    `bnext` segments, one per base cell and one per relink; nothing unlinks, only drops are linked, and each monster
    relinks at most once, at its first visit (a wire-24 teleport inside the chase relinks the same monster again and
    the walk follows only its final link). Counting the segments that can reach each thing gives visits ≤ n(1+L) + L +
    D′(1+L) ≤ n(R+2) − 2 (n the linked things, L the relinkers, D′ the other drops, R the visit roster), below the
    mutant's limit 9 + n(R+2), so the mutant refuses nothing and no test can kill it. The argument does not depend on
    the cell a relink lands in, which was M4a's open case. The proof is a comment above `limit`; behaviour is unchanged. The step 14 review found that the bound `n >= K + 2`
    behind it also holds for a rocket under `episode1`, which stands in for the barrel because `LifeHost::spawn` links
    every thing into `spatial.order`, NOBLOCKMAP included; the comment now says so (6543a46, comment only).
    Owner ruling (2026-11-13): no exclusion in the mutation workflow unless the mutant appears in it (decision I12 runs
    `--in-diff`, and line 362 is unchanged on this branch). The step-14 review should check the proof.
28. **The height bound and the boss floor, refined (step 8, 750b307).** Finding 22 stands and is now tested: a BOSS1
    edit whose tag-666 step has its lowest neighbour at −1,024 descends one unit per tic to it unrefused; at −1,025 the
    map is refused at load (`GeometryError::CoordinateOverflow`). Finding 21 is refined: the global `[R_min, R_max]`
    is unchanged, but the per-sector interval of a tag-666 sector misses the lowered floor, which can make `Δ` too
    small through the close30 or turbo terms of sectors whose lowest floor depends on it (mover_rules.rs:820–841).
    `Δ` and `R_min` are read only by V′ cases (c) and (e) (player_map_scale.rs:770–795), so this can only refuse more,
    never admit wrongly. On E1M1–E1M9 a temporary rule left `(reach, Δ)` unchanged (E1M8: one tag-666 sector). Owner
    ruling (2026-11-13): no change to the map-8 rule; recorded here.
29. **The Baron fireball wall tunnel is a declared wire-24 refuse-the-tic witness (step 9 found it, step 10 records
    it; owner ruling 2026-11-13).** A Baron fireball steps up to 15 units in one unsplit move, more than the 10
    of M3h5 Theorem B for radius 6, so `P_CheckPosition`, which tests only the destination box (p_map.c:191–195,
    374–442), can accept a step that carries the ball wholly past a one-sided wall. The genuine engine keeps
    such a ball flying in the void (observed, three variants byte-identical, on a synthetic stream). Rust admits
    the tunnelling tic, then refuses the next tic with `Monster(Arithmetic)` from the vertex-rectangle check on
    the spawned actor (`demon_map_spawn.rs:155–157`) when the ball has left the rectangle, and the refused tic
    keeps the whole canonical state. Wording of the ruling: Rust either matches the genuine engine or refuses;
    this is not a difference from the original. Wire 23 cannot reach it. It is reachable on E1M8 in principle
    (two Barons on both skills); no real-map run hit it. The soak and the fuzz target admit exactly this case
    through `bruiser_tunnel` (8223b49) and nothing else. **The inner-wall case is covered and matched:** the
    second-set scene BTUNNEL (2b2ebe9, a994737) has a ball step wholly past a 1-unit inner wall and explode on
    the far wall, never outside the rectangle, so Rust admits every tic and matches the genuine engine
    ([evidence](validation/M4B.md#the-baron-fireball-wall-tunnel)). An in-tic witness refusing the tunnelling tic
    itself (`MapScaleError::Escape`, as the barrel corner jump) was the rejected alternative: it would change
    behaviour and need new C evidence and goldens, so it is a possible follow-up only.

30. **A walking monster's teleport fog 1 has the monster's pre-move z (step 14 review finding F1; fixed in 324efa8).**
    `P_Move` calls `P_TryMove` (p_enemy.c:293), whose crossing loop runs `EV_Teleport` (p_map.c:500–514); `P_Move`
    clears `MF_INFLOAT` and sets `z = floorz` only afterwards (p_enemy.c:319–328), and `EV_Teleport` spawns fog 1 at
    `oldz` (p_telept.c:100–102, 111). Rust set z before `cross_lines`. The difference shows only when the fog's
    sector floor is at or below the old z while the box's highest floor is above it, because the fog runs
    `P_ZMovement` in the same tic (p_tick.c:65–71, p_mobj.c:428–436, 284–311); a first scene without that condition
    showed z 24 in C as well. The second-set scene TELEZ (047b2e5) reaches it: genuine three variants byte-identical,
    teleport at dump tic 10, fog 1 at z 0. Before the fix `check_player_episode1v2.py --require` passed 51 of 52
    and TELEZ failed at tic 11, first differing field `actor.3.z` (C 0, Rust 0x1800 in the high word, 24 << 16).
    The fix is ungated: no older policy reads the crosser's z or flags inside the crossing loop (Starter holds no
    actor store; `teleport::cross` refuses before reading unless `world.teleport`), so older records are
    unchanged and every historical golden passes. Tests ea3a592 fail on the old order. Evidence:
    [the evidence record](validation/M4B.md#second-synthetic-set-two-teleports-in-one-tic).

### Open items carried from Phase 1

These are places where the drafts of this note disagree, or leave a question open. None was resolved by choosing a side.

- Closed (owner ruling, 2026-11-10): IWAD identity. The local `DOOM.WAD` is the four-episode build (§1.1), and the genuine
  engine runs it as `registered` because `IdentifyVersion` decides by file name (linuxdoom-1.10/d_main.c:690-702; §1.1). That is
  what the C driver does (`gamemode = registered`, tools/reference/m4a_driver.c:573), so no behaviour changes. `A_BossDeath`'s
  game-mode test takes the same non-commercial branch for registered and retail (§3 part B, fact 20). The wording here and in the
  roadmap now says "the four-episode `DOOM.WAD`, which the genuine engine runs as registered because of its file name".
- Closed (owner ruling, 2026-11-10): policy names. Wire 24 is `PlayerMonstersEpisode1V2` (I2 confirmed); §4 now uses that name.
- Open: `numspechit` after a teleport. The inventory (§4.2.1) repeats the M4a wording "−1 after the teleport" (M4a note §3.3.3).
  The C facts (§2 T6.1 and correction 3) show −1 is the normal exit value of every completed crossing loop and not
  teleport-specific.
- Open: provenance of the thrust figures. The inventory (§4.2.4) says −3,979,345 / −994,836 / −397,934 were computed and not
  observed; §2 T8.5 records the same values as observed in a gcc probe. The values agree.
- Closed (step 4, 2026-11-10) for the port: duplicate player-1 starts (voodoo dolls, §2 T3.9) are refused at spawn under wire 24
  (finding 16); E1M5 and E1M9 load under wire 24, so each has exactly one player-1 start. Tag-0 teleport lines (§2 T2.3) were not
  censused; the port models the missing tag-0 guard (`EV_Teleport` searches the tag-0 sectors). Landings spawn on both skills (one each, §1.6).
- Closed (step 4, 2026-11-10): wire 23's `SpecialCross` witness covers the `P_ThingHeightClip` caller (§2 T9.1); finding 13.
- Closed (step 4, 2026-11-10): the inventory's `for_things` margin (`radius + 32`) equals `P_TeleportMove`'s box (§4.2.3, against §2 T3.3); finding 14.
- Closed (owner ruling, 2026-11-10): a SHA-256 of the original, unpatched `DEMO1` lump may be committed as an input-identity
  digest (and that of the patched scratch copy, if useful); nothing else from the lump (decision I5).
- Closed (2026-11-10): the shadow-fuzz draw order of `A_FaceTarget` and Baron-against-Baron infighting through barrel
  splash were observed in Phase 2 step 1 (findings 1 and 2 above); the `P_SpawnMissile` shadow pair was observed in M4a's
  INVIS1 (finding 18, step 5).
- Reminder for the independent review (step 14, owner request 2026-11-10): check the slide momentum re-read of
  finding 17 (`P_SlideMove` reads `momx` after the move up to the wall, p_map.c:776) specifically, including that it
  changes nothing for wires ≤23.
