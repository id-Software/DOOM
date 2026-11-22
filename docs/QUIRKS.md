# Quirks of the original

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
>
> This register was assembled by agents from a machine sweep of the repository,
> revised after an independent agent review and the owner's rulings, and has
> not been read line by line by a person.

This page is the index of the register of quirks of id Software's linuxdoom-1.10
(the behavioural authority of this repository, commit
`a77dfb96cb91780ca334d0d4cfd86957558007e0`) that a careful reader would call a
bug, an accident or a surprise, and the authoritative home of that list. It holds the quirks found
by the 2026-11-21 sweep described below; it is not claimed to be complete. One row is one quirk: the quirk in a plain sentence, the C file and line
that causes it, what a player sees or hears, its status in this project, and
whether a small change to the C would remove it. **Nothing here fixes anything.**
The register records the original; it does not change what any note, record,
comment, test or tool says.

The rows are split by C file into pages under [quirks/](quirks/): this page carries
the totals, the status table and a link per file. It is not the home of support
status. What the Rust engine supports, refuses or declares stays in
[COMPATIBILITY](COMPATIBILITY.md); a row's status column quotes or cites COMPATIBILITY, or
the investigation note or evidence record that COMPATIBILITY rests on, and is a
summary, not a second authority. If a row and COMPATIBILITY disagree about
support, COMPATIBILITY is right and the row is stale. Milestone planning is in
[ROADMAP](ROADMAP.md); the roadmap's
[limits, overflows and quirks checklist](ROADMAP.md#limits-overflows-and-quirks)
links here.

**Keeping it current.** A pull request that finds or reproduces a quirk of the
original updates its row in the register (this index and the pages under
`docs/quirks/`) in the same pull request, and adds the row if there is none
(standing rule in [AGENTS](../AGENTS.md)).

## Totals

Updated on 2026-11-22 (America/Los_Angeles).

- **Rows: 473**, of which 464 are on the 53 C-file pages and 9 are on the [64-bit build only](quirks/64-bit-only.md) page.
- How the number came about: the sweep wrote 890 raw entries; the merge made 443 rows; the first assembly (commit 2c2ee84) joined 23 pairs into 420 rows; the revision after the independent review and the owner's rulings joined 10 more (listed below), split 10 of the 11 mixed-status rows into 23 rows by status (one row per status; the 11th, Q-p_setup-536, became a row of the 64-bit page), moved 3 rows to the 64-bit page and added 6 rows there: 420 - 10 + 13 + 6 = 429.
- The separate lists: [recorded in only one place and in no note](quirks/only-one-place.md) (12 whole or partial rows and a list of readings); [not traced to a C line](quirks/not-traced.md) (none left); [dropped, and why](quirks/dropped.md) (54 entries); [where the documents disagree](quirks/disagreements.md) (75 entries, 7 marked policy claim); [the 64-bit build only](quirks/64-bit-only.md) (9 rows).

The 2026-11-22 follow-up split 24 more rows into 48 and classified 48 rows from the Rust code and existing tests. A separate refusal row records the historical endpoint policy for a stalled trace. A further audit split 19 remaining mixed-status rows into 38, distinguished the R4 flagged-column contract from a refusal, and settled the two 64-bit harness-exposure unknowns by reading the builders. The status counts below describe the current register.

### Status of the 464 rows on the C-file pages

Each row carries one status. The 64-bit page records whether the reference harness is affected.

| Status | Rows |
| --- | ---: |
| reproduced | 268 |
| refused | 90 |
| not yet ported | 49 |
| not reproduced (declared …) | 56 |
| comment only | 1 |
| Total | 464 |

## Rows by C file

One page per C file (a header counts as its own file); each page carries its own table and its own Notes.

| C file | Rows |
| --- | --- |
| [d_main.c](quirks/d_main.c.md) | 11 |
| [d_net.c](quirks/d_net.c.md) | 5 |
| [doomdef.h](quirks/doomdef.h.md) | 1 |
| [f_wipe.c](quirks/f_wipe.c.md) | 3 |
| [g_game.c](quirks/g_game.c.md) | 35 |
| [hu_lib.c](quirks/hu_lib.c.md) | 3 |
| [hu_stuff.c](quirks/hu_stuff.c.md) | 7 |
| [i_sound.c](quirks/i_sound.c.md) | 21 |
| [i_system.c](quirks/i_system.c.md) | 2 |
| [i_video.c](quirks/i_video.c.md) | 13 |
| [info.c](quirks/info.c.md) | 1 |
| [m_bbox.c](quirks/m_bbox.c.md) | 1 |
| [m_fixed.c](quirks/m_fixed.c.md) | 2 |
| [m_menu.c](quirks/m_menu.c.md) | 3 |
| [m_misc.c](quirks/m_misc.c.md) | 3 |
| [p_ceilng.c](quirks/p_ceilng.c.md) | 1 |
| [p_doors.c](quirks/p_doors.c.md) | 6 |
| [p_enemy.c](quirks/p_enemy.c.md) | 28 |
| [p_floor.c](quirks/p_floor.c.md) | 13 |
| [p_inter.c](quirks/p_inter.c.md) | 26 |
| [p_lights.c](quirks/p_lights.c.md) | 2 |
| [p_map.c](quirks/p_map.c.md) | 41 |
| [p_maputl.c](quirks/p_maputl.c.md) | 19 |
| [p_mobj.c](quirks/p_mobj.c.md) | 37 |
| [p_plats.c](quirks/p_plats.c.md) | 4 |
| [p_pspr.c](quirks/p_pspr.c.md) | 9 |
| [p_saveg.c](quirks/p_saveg.c.md) | 4 |
| [p_setup.c](quirks/p_setup.c.md) | 8 |
| [p_sight.c](quirks/p_sight.c.md) | 7 |
| [p_spec.c](quirks/p_spec.c.md) | 23 |
| [p_switch.c](quirks/p_switch.c.md) | 8 |
| [p_telept.c](quirks/p_telept.c.md) | 3 |
| [p_tick.c](quirks/p_tick.c.md) | 1 |
| [p_user.c](quirks/p_user.c.md) | 7 |
| [r_bsp.c](quirks/r_bsp.c.md) | 1 |
| [r_data.c](quirks/r_data.c.md) | 12 |
| [r_defs.h](quirks/r_defs.h.md) | 1 |
| [r_draw.c](quirks/r_draw.c.md) | 9 |
| [r_main.c](quirks/r_main.c.md) | 5 |
| [r_plane.c](quirks/r_plane.c.md) | 5 |
| [r_segs.c](quirks/r_segs.c.md) | 9 |
| [r_sky.c](quirks/r_sky.c.md) | 1 |
| [r_things.c](quirks/r_things.c.md) | 4 |
| [s_sound.c](quirks/s_sound.c.md) | 21 |
| [sndserv/wadread.c](quirks/sndserv-wadread.c.md) | 2 |
| [sounds.h](quirks/sounds.h.md) | 1 |
| [st_lib.c](quirks/st_lib.c.md) | 4 |
| [st_stuff.c](quirks/st_stuff.c.md) | 17 |
| [st_stuff.h](quirks/st_stuff.h.md) | 1 |
| [tables.c](quirks/tables.c.md) | 4 |
| [v_video.c](quirks/v_video.c.md) | 5 |
| [w_wad.c](quirks/w_wad.c.md) | 2 |
| [wi_stuff.c](quirks/wi_stuff.c.md) | 2 |

| **Total** | **464** |

No page: p_local.h (its two rows were joined into rows of p_map.c and p_maputl.c; see the lists below).

Other pages: [recorded in only one place](quirks/only-one-place.md), [not traced to a C line](quirks/not-traced.md), [dropped, and why](quirks/dropped.md), [where the documents disagree](quirks/disagreements.md) and [the 64-bit build only](quirks/64-bit-only.md).

### Finding a row from its id

An id is `Q-<file>-<line>`: the file name with `.` written `_` (`p_map` for p_map.c, `p_local_h` for p_local.h, `wadread` for sndserv/wadread.c) and the first C line it concerns; a second row on the same line takes `-2`, and the rows of a quirk with several statuses take a letter, `a`, `b`, `c`. The row is on the page of that C file, at `quirks/<file>.md#<id>`, for example `quirks/p_map.c.md#Q-p_map-189` (an id anchor, written as text here because the link checker resolves only heading anchors). Every id of the first assembly still resolves as an anchor; the ids that moved are listed here:

- **joined** rows keep the absorbed id as an extra anchor on the surviving row;
- **split** rows keep the original id as an anchor on the first of the new rows (`a`);
- **moved** rows (the 64-bit faults) are on the [64-bit page](quirks/64-bit-only.md) under the same id.

Later pull requests should cite ids as text, not as links to an id anchor.

#### Ids joined into another row (10)

| Absorbed id | Now an anchor on | Page |
| --- | --- | --- |
| Q-i_sound-213 | Q-s_sound-364 | [s_sound.c.md](quirks/s_sound.c.md) |
| Q-m_fixed-62-2 | Q-p_sight-125 | [p_sight.c.md](quirks/p_sight.c.md) |
| Q-m_fixed-62-3 | Q-r_main-412 | [r_main.c.md](quirks/r_main.c.md) |
| Q-p_inter-675 | Q-p_inter-705 | [p_inter.c.md](quirks/p_inter.c.md) |
| Q-p_inter-914 | Q-p_maputl-529 | [p_maputl.c.md](quirks/p_maputl.c.md) |
| Q-p_local_h-154 | Q-p_maputl-544 | [p_maputl.c.md](quirks/p_maputl.c.md) |
| Q-p_local_h-51 | Q-p_map-1221 | [p_map.c.md](quirks/p_map.c.md) |
| Q-p_mobj-152-2 | Q-p_map-191c | [p_map.c.md](quirks/p_map.c.md) |
| Q-p_mobj-152-3 | Q-p_map-191a | [p_map.c.md](quirks/p_map.c.md) |
| Q-p_mobj-819-2 | Q-p_enemy-795 | [p_enemy.c.md](quirks/p_enemy.c.md) |

#### Ids split by status (10 rows into 23)

| Original id | Now the rows |
| --- | --- |
| Q-p_map-191 | Q-p_map-191a, Q-p_map-191b, Q-p_map-191c |
| Q-g_game-1589 | Q-g_game-1589a, Q-g_game-1589b |
| Q-p_floor-242 | Q-p_floor-242a, Q-p_floor-242b |
| Q-p_inter-806 | Q-p_inter-806a, Q-p_inter-806b |
| Q-p_inter-902 | Q-p_inter-902a, Q-p_inter-902b |
| Q-p_switch-113 | Q-p_switch-113a, Q-p_switch-113b, Q-p_switch-113c |
| Q-r_draw-528 | Q-r_draw-528a, Q-r_draw-528b |
| Q-s_sound-330 | Q-s_sound-330a, Q-s_sound-330b, Q-s_sound-330c |
| Q-s_sound-367 | Q-s_sound-367a, Q-s_sound-367b |
| Q-v_video-171 | Q-v_video-171a, Q-v_video-171b |

#### More ids split by status on 2026-11-22 (24 rows into 48)

| Original id | Now the rows |
| --- | --- |
| Q-f_wipe-159 | Q-f_wipe-159a, Q-f_wipe-159b |
| Q-g_game-579 | Q-g_game-579a, Q-g_game-579b |
| Q-g_game-1421 | Q-g_game-1421a, Q-g_game-1421b |
| Q-g_game-1508 | Q-g_game-1508a, Q-g_game-1508b |
| Q-i_video-641 | Q-i_video-641a, Q-i_video-641b |
| Q-m_menu-1131 | Q-m_menu-1131a, Q-m_menu-1131b |
| Q-p_floor-169 | Q-p_floor-169a, Q-p_floor-169b |
| Q-p_inter-476 | Q-p_inter-476a, Q-p_inter-476b |
| Q-p_map-240 | Q-p_map-240a, Q-p_map-240b |
| Q-p_map-1290 | Q-p_map-1290a, Q-p_map-1290b |
| Q-p_maputl-472 | Q-p_maputl-472a, Q-p_maputl-472b |
| Q-p_maputl-846 | Q-p_maputl-846a, Q-p_maputl-846b |
| Q-p_mobj-150-2 | Q-p_mobj-150-2a, Q-p_mobj-150-2b |
| Q-p_mobj-672 | Q-p_mobj-672a, Q-p_mobj-672b |
| Q-p_mobj-794 | Q-p_mobj-794a, Q-p_mobj-794b |
| Q-p_pspr-542 | Q-p_pspr-542a, Q-p_pspr-542b |
| Q-p_spec-503 | Q-p_spec-503a, Q-p_spec-503b |
| Q-p_spec-658 | Q-p_spec-658a, Q-p_spec-658b |
| Q-p_switch-224 | Q-p_switch-224a, Q-p_switch-224b |
| Q-r_data-685 | Q-r_data-685a, Q-r_data-685b |
| Q-r_main-484 | Q-r_main-484a, Q-r_main-484b |
| Q-r_plane-398 | Q-r_plane-398a, Q-r_plane-398b |
| Q-s_sound-364 | Q-s_sound-364a, Q-s_sound-364b |
| Q-tables-59 | Q-tables-59a, Q-tables-59b |

The corner follow-up also split Q-p_maputl-871 into Q-p_maputl-871a and Q-p_maputl-871b.

#### Further status audit on 2026-11-22 (19 rows into 38)

| Original id | Now the rows |
| --- | --- |
| Q-g_game-186 | Q-g_game-186a, Q-g_game-186b |
| Q-g_game-409 | Q-g_game-409a, Q-g_game-409b |
| Q-g_game-1512 | Q-g_game-1512a, Q-g_game-1512b |
| Q-hu_stuff-518 | Q-hu_stuff-518a, Q-hu_stuff-518b |
| Q-m_misc-282-2 | Q-m_misc-282-2a, Q-m_misc-282-2b |
| Q-p_map-81 | Q-p_map-81a, Q-p_map-81b |
| Q-p_map-1298 | Q-p_map-1298a, Q-p_map-1298b |
| Q-p_maputl-544 | Q-p_maputl-544a, Q-p_maputl-544b |
| Q-p_maputl-866 | Q-p_maputl-866a, Q-p_maputl-866b |
| Q-p_enemy-1994 | Q-p_enemy-1994a, Q-p_enemy-1994b |
| Q-p_inter-808 | Q-p_inter-808a, Q-p_inter-808b |
| Q-p_setup-336 | Q-p_setup-336a, Q-p_setup-336b |
| Q-p_spec-988 | Q-p_spec-988a, Q-p_spec-988b |
| Q-p_spec-1040 | Q-p_spec-1040a, Q-p_spec-1040b |
| Q-r_data-351 | Q-r_data-351a, Q-r_data-351b |
| Q-r_data-394 | Q-r_data-394a, Q-r_data-394b |
| Q-r_draw-142 | Q-r_draw-142a, Q-r_draw-142b |
| Q-st_stuff-384 | Q-st_stuff-384a, Q-st_stuff-384b |
| Q-w_wad-370 | Q-w_wad-370a, Q-w_wad-370b |

#### Ids moved to the 64-bit page (3)

| Id | Page |
| --- | --- |
| Q-m_misc-229 | [64-bit-only.md](quirks/64-bit-only.md) |
| Q-p_setup-536 | [64-bit-only.md](quirks/64-bit-only.md) |
| Q-r_data-642 | [64-bit-only.md](quirks/64-bit-only.md) |

## How this register was built

Built on 2026-11-21 (America/Los_Angeles) from a read-only sweep followed by a merge, then revised the same day. This describes that day's build.

1. **The sweep.** Nineteen sweep passes read the investigation notes, the evidence records, COMPATIBILITY, the frozen records under [validation/archive](validation/archive/README.md), the comments and test names of the Rust source and the tools, and the commit messages on master, and wrote 890 raw entries. Each entry names a quirk, the C line the document cites, and where it came from. Quirks are often not labelled "quirk", so the sweepers looked for any behaviour of the original that is described as a bug, an accident, an overflow, an undefined or stale read, a limit, an asymmetry or a comment that contradicts the code.
2. **The merge.** The entries were grouped by the C file they cite and merged by seven passes into 443 rows. The merger opened `linuxdoom-1.10/` for every citation (see "What verified means"). Entries that are not quirks of the original (plain design, faults of the Rust port, harness artefacts, aggregate list entries, documentation slips) were dropped and are listed in [Dropped, and why](quirks/dropped.md).
3. **The assembly.** A script (kept outside the repository) turned the row files into tables with no row or source retyped, and the same quirk found by two passes under two C files was joined into one row that keeps every source: 23 pairs, leaving 420 rows. At assembly, a row whose status had no source used "not determined from the documents". The follow-up checked those statuses against code and tests.
4. **The revision.** An independent review of the assembled register and the owner's three rulings of 2026-11-21 were applied together: (a) faults of building the source on a 64-bit host get their own section, never a fixes-profile candidate, with the reference harness's exposure stated per row; (b) the register is split by C file with this index; (c) each row of mixed status became one row per status. The review's findings were applied: one cause was corrected (the `MAXRADIUS` double shift), duplicate rows were joined, overclaims in the surrounding pages were softened, run counts and test totals were removed from rows in favour of links to the evidence, quoted fragments were checked against their sources, IWAD-derived detail was removed, and the list of disagreements was rebuilt so that each entry carries its documents, the C line and the side that matches the C.

## What "verified" means

For a citation, "verified" means an agent opened the C at the cited lines during the merge, the assembly or this revision and the lines hold what the row says. Every citation was opened this way except the rows whose C column says "not reopened", "not opened", "not re-read", "not re-checked", "not verified" or "not re-counted": 9 rows at this revision (Q-i_video-218, Q-p_inter-681, Q-p_map-776, Q-p_mobj-566, Q-p_saveg-232, Q-r_data-701, Q-r_main-870, Q-wadread-206, Q-wi_stuff-1482), and the 64-bit page's Q-p_saveg-40 (a count not re-counted). It does not mean the row's effect on a player was observed: the "what a player sees or hears" column comes from the documents or is evident from the C, and says "not stated" when neither gives it. The C for the `sndserv/` rows is in the top-level `sndserv/` folder, not in `linuxdoom-1.10/`. The revision re-opened the C for the rows it changed and for a further sample of 30 rows that neither the assembly's sample nor the review named; those results are in the pull request, not here, because they are one-time observations.

## What was not read in full

This register is only as complete as its sources and the sweeps, and the sweeps did not read everything.

- **Documents.** Read in full: COMPATIBILITY, ROADMAP, the M5B, M5C, M4A, M4B and M4P notes and evidence records, the M3H1-M3H9C notes and records (the large timing and per-slot tables and the `*_IWAD_COUNTS` per-slot rows were skimmed, not read row by row), and the M2, M3A-M3G notes and records (the evidence records for M3F2-M3G10 and the ZSTD maintenance record were read in full by a second pass). The M5A note and record, ARCHITECTURE, ACTOR_IDENTITIES and VALIDATION were read in two passes that together cover every line. WEAPON_FORMULAS, ASSETS, EVIDENCE_STORAGE and README were read in full.
- **The archive.** The archived guides and notes under [validation/archive](validation/archive/README.md) were read in full except the manifest (headings and digests only), the `.txt` twins of the `.md` copies (not read separately; they differ in banners and links), and the data files (`.json`, `.csv`, `.txt` evidence under the `m3g*` and `maintenance-zstd` subdirectories), which hold no prose.
- **Rust source and fuzz targets.** Only comments and test names were read, never the code bodies. In `crates/doom-sim` roughly 5,600 comment lines that had no C citation and no keyword were read in a second pass; the generated tables `identity/names.rs` and `fine_sine.rs` were not read. In `doom-game`, `doom-play`, `doom-render`, `doom-sound` and the fuzz targets the first pass read a keyword-filtered extract plus every test name, and a second pass read the comment lines the first had not; "quirks stated in unlabelled prose with none of the filter words may have been missed" is the sweepers' own limit. A quirk mentioned only in code, with no comment or test name, would not appear here.
- **Tools.** The comments and docstrings of 371 files under `tools/` were read (about 9,400 lines kept after dropping empty ones); the code bodies, the two `source_suite*.json` data files and the pure data tables were not read.
- **Commit messages.** Three passes read the log of master's commit messages, newest first. The local clone is shallow: its log holds 898 commits and ends at the merge of PR #39. *Pass 1* read the first 1,600 log lines (the newest commits; the passes count them as 150 and as 127 commits, because they counted differently) in full, and read the remaining older commits of the clone only through filters (a C citation, or one of the words quirk, undefined, overflow, uninitialised, stale, bug, surprise, accident, "does not match", contradict, `I_Error`, crash, "the original" or "genuine engine"), plus the subject lines of some of them. *Pass 2* then read every subject and body in full, with no filter, from log line 1,590 to the end of the clone's log, which covers every commit from about the 128th newest on. *Pass 3* read the 325 commits older than the clone's log, back to the original release, from a full log obtained separately; almost all of them are subject-only. So every commit message was read in full at least once; the newest 127 to 150 were read in full by one pass only.
- **STATE_FORMAT and TESTING.** Searched with a keyword grep only (quirk, undefined, overflow, uninitialised, stale, dangling, freed, accident, out of bounds, `I_Error`, never reset, zero-fill), not read. Every hit is either a Rust policy or already a row (the never-reset wall-step globals, the zero-filled mover allocations, the diagonal-wall reading quirks, the THING angle overflow); the grep found no new quirk.
- **Not yet in the register.** The quirk list of milestone M6a ("U8") is not in yet; it goes in as rows when pull request #65 lands, and #65 is not edited for it.

## How to read a row

- **Id.** `Q-<file>-<line>` is stable; a later pull request points at a row by its id, written in text. Each row carries an HTML anchor of its id, so `quirks/p_map.c.md#Q-p_map-189` works in a browser. Paths of Rust source in the sources column are written without the leading `crates/`.
- **C file and line.** The lines were opened in `linuxdoom-1.10/` (or `sndserv/`). A document that cites a different line is noted under the table of the file, or in [Where the documents disagree](quirks/disagreements.md) when the difference is the point.
- **Status.** One of these words first, then the document's own words; a row has one status, and a quirk with several has one row per status:
  - `reproduced (...)` the Rust engine does what the C does, under the wire or milestone named (the wire numbers are the policy table of [COMPATIBILITY](COMPATIBILITY.md#policies)); its row gives the admitted scope;
  - `refused (...)` the Rust engine does not run the situation: it rejects the map at load, or refuses the tic with a named error or witness and the state preserved, instead of doing what the C does;
  - `not yet ported` the behaviour belongs to a system the project does not have yet, or to a mechanism not admitted under any wire;
  - `not reproduced (declared ...)` a document states that the project deliberately does something else (a declared contract of the playable build, or an adapter of the reference harness, for example); the row cites the contract or adapter;
  - `comment only` the original comment contains the quirk and has no executable effect.
- **Small C change removes it?** `yes`, `no` or `unknown`, with at most a few words of why. This is a judgement from reading the C, not a claim that the change is safe: a "yes" never means the project should make it.
- **Sources.** The documents the row came from. Abbreviations: `COMPAT` is [COMPATIBILITY](COMPATIBILITY.md); `M5A`, `M5B`, `M5C`, `M4A`, `M4B` (a bare one, or "note") are the milestone notes such as [M5B_REFERENCE_INVESTIGATION.md](M5B_REFERENCE_INVESTIGATION.md); "record" or `VAL-M5A` and similar are the evidence records such as [validation/M5B.md](validation/M5B.md); `§` numbers and decision labels such as `S4`, `R3`, `T11` or `H3` are the note's own. "(line from the sweep)" marks a line number the sweep supplied for a document that names only the file. Source files, tools and commits are cited by path and line or SHA. A joined row lists the sources of every row it absorbed, marked "joined from".
