# Quirks recorded in only one place

[Back to the register index](../QUIRKS.md).

The owner asked to see anything that is described as a quirk in only one place
(a source comment, a test name, a tool, an archive file or a commit message)
and nowhere in an investigation note, an evidence record or COMPATIBILITY, so
that it is clear what was never properly recorded. For every row the merge
re-checked with broad greps over `docs/*.md` and `docs/validation/*.md` (the
archive excluded); several sweepers' "found only here" claims were wrong on
re-check and are not repeated. What is left is below.

**Rows whose only source is a source comment** (searched in `docs/*.md` and `docs/validation/*.md`, not the archive; the notes are the investigation notes, COMPATIBILITY and the evidence records):

- `Q-p_lights-83`: a fire flicker's lower bound can sit above its upper bound. Only a comment in `fuzz/fuzz_targets/player_specials.rs:168-172`. Searches: `minlight`, `fireflicker`, `fire flicker`, `flicker`, `& 64` / `maxtime`, `exceed`, `minlight + 16`, `p_lights.c:83`. The notes give only the formula, the special-17 listing, the field description and an "ordered" envelope check; none says `minlight` can exceed `maxlight`.
- `Q-r_data-394`: a single-patch column whose patch is lump 0 reads unwritten composite bytes. Only a comment in `crates/doom-render/src/r_data.rs:267-272` (and a test comment). Searches: `lump 0`, `lump zero`, `lump > 0`, `lump number 0`, `lump index 0`, `LumpZero`, `collump[x] >= 0`, `first lump of the directory`; the only hits state the `lump > 0` test, not the consequence.
- `Q-p_sight-106`: the comment of `P_InterceptVector2` names the wrong callers. Only the C source comment itself. Searches: `only called by`, `addthings and addlines`, `addlines traverser`.

**Rows with a part recorded in only one place:**

- `Q-p_enemy-738`: the consequence "a Baron that can never move keeps movecount nonzero and never fires" is stated only in `tools/reference/m4b_wad.py:362-365` (searched `never fires`, `can never move`, `keeps movecount`). The gate itself is in the notes.
- `Q-p_enemy-865`: the contradiction between the comment "keep firing unless target got out of sight" and the code is the reading of an earlier sweep of the archive; no note states it (searched `keep firing`, `got out of sight`).
- `Q-p_map-189`: the checklist row in ROADMAP and the commit that added it (2eb9398) are the only places; no investigation note and no COMPATIBILITY section describes the behaviour. COMPATIBILITY records only a refusal at load of one map on one skill, for a reason "not yet known".
- `Q-p_map-618`: the extra `ANG180` in `P_HitSlideLine` is described as source in M3H5, but no document says it can reverse the slide; the reversal values are the sweep's own computation.
- `Q-g_game-1421`: with `-fast`, every later `G_InitNew` halves the tics again (the condition tests `fastparm` alone); only the single halving is in a note (M3D3F). Searched `halv`, `>>= 1`, `fastparm`.
- `Q-i_sound-334`, `Q-i_sound-143`, `Q-i_sound-364`: the i_sound.c comments that contradict the code ("(unused)" at 339, "(not used)" at 494, the "clamp volume" comment at 363, the "unused" pitch table). The notes record the code, not that the comments are wrong; commits 5700cb5 and cff3ed4 mention the pitch comments.
- `Q-p_mobj-331`: the comment at p_mobj.c:288-290 about the lost-soul momz negation describes an older statement order than the code (the negation at 291-295 comes before the zeroing at 297-310). No document discusses it (searched `kinda useless`, `somebody left`, `p_mobj.c:29`).
- `Q-p_saveg-303`: `tracer` is restored unchanged on load by the same raw copy (p_saveg.c:300), keeping a pointer value from the saving session. No document says so (searched `tracer` with save, unarchive, p_saveg).
- `Q-p_mobj-794`: p_mobj.c:627 is cited by no document (the angle row has other sources).

**Readings that are the register builder's or an earlier sweep's own, in no document** (each is in the row's note or the sentence below; none is presented as an observation):
`Q-g_game-74` (the check follows the writes), `Q-g_game-815` (the consequence for the menu pause), `Q-p_doors-118` (the `open` type is also missing from the down branch's switch), `Q-p_enemy-512` (the single-player case), `Q-p_enemy-693` (`actor->angle &= (7<<29)` has the same signed-shift form), `Q-p_enemy-802` (the chaingunner plays `sfx_shotgn`), `Q-p_mobj-900` (no document states a player-visible effect), `Q-p_mobj-758` (the consequence of an unknown THING type is not stated in words), `Q-p_map-1290` (a C observation of the item case exists only in tools; the wire-20 sentence was not updated).
