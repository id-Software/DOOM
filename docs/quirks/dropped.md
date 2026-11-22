# Dropped, and why

[Back to the register index](../QUIRKS.md).


What the merge dropped as not a quirk of the original: plain design, faults
of the Rust port, harness artefacts, aggregate or duplicate list entries and
documentation slips. Each says why. The row text is as the merger wrote it
(the sweep's entry names are kept so a reader can search the notes).

`z_zone.c:440` (`Z_ChangeTag`'s owner-pointer test, found in a commit message) was dropped here at first as a portability fault of building the old C on a modern host. The owner ruled on 2026-11-21 that such faults get their own section: it is now row Q-z_zone-440 on the [64-bit build only](64-bit-only.md) page, with the other LP64 faults. `state_t`'s `long` fields (item 3 of chunk 2) are not a fault of that kind: every value fits 32 bits.

**Chunk 1 (d_main.c, d_net.c, doomdef.h, f_wipe.c, g_game.c, hu_lib.c, hu_stuff.c, i_sound.c, i_system.c):**

1. skill-argument-is-one-more-than-the-skill-value — plain design: the command line counts skills from 1 like the menu, and the demo header stores the 0-based skill_t; M5B note 2.7 records it as a naming difference, not a defect.
2. actions-have-no-original-ordinal — a port representation rule (the Rust ActionId never serialises a C address); the `(actionf_v)(-1)` removal sentinel has no effect in play.
3. nochange-sentinel-ten-numweapons-nine — plain design: an enum sentinel placed after the count.
4. nochange-sentinel-is-10-after-numweapons-9 — duplicate of 3, same reason.
5. wipe-colour-crossfade-unreachable — dead code with no effect (the sweeper noted it as likely not a quirk).
6. noblit-parsed-never-read — a variable set and never read; no effect.
7. hu-lastautomapactive-is-dead-state — a static written and never read; no effect.
8. steptable-clear-loop-commented-out — a commented-out clearing loop whose state static zero initialisation already gives; no effect.
9. i-sound-errno-redeclared — a build-portability issue of the source with a modern libc (handled by declared one-line adapters), not behaviour.
10. weapon-key-nine-never-tested — plain design: the loop stops before wp_supershotgun because P_PlayerThink selects it through key 3 (p_user.c:300-303, verified); keys 9 and 0 are simply unbound.

**Chunk 2 (i_video.c, info.c, info.h, m_bbox.c, m_fixed.c, m_menu.c, m_misc.c, m_random.c, p_ceilng.c, p_doors.c, p_enemy.c):**

1. info.c `firepistol-cadence-nineteen-vs-fourteen`: plain design. The pistol's first and held-fire intervals follow from the state durations (info.c:149-152, verified); nothing about them is a bug, accident or surprise.
2. info.c `player-type-has-missile-state-reference`: no behaviour of the original. MT_PLAYER's missilestate S_PLAY_ATK1 (info.c:1120, verified) is never used through A_Chase for a player; the finding concerns a Rust native copy (harness/port), which clears it.
3. info.h `info-fields-are-c-long`: plain design (considered for the [64-bit page](64-bit-only.md), not a fault there either). state_t's frame/tics/misc fields are C long (info.h:1148-1154, verified); every value in the table fits i32, so no outcome depends on the width.
4. m_random.c `rndtable-never-returns-40`: an evidence limit, not a behaviour. The table lacks 40 and 38, so a `< 40` boundary cannot be exercised by real draws; no outcome of the original differs. (The fact is kept as a note on the A_CPosRefire row.)
5. m_random.c `painchance-lut-lacks-179-180`: the same kind of evidence limit (no 179 or 180 in the table).
6. m_random.c `rndtable-has-no-40-and-no-179-180`: duplicate of 4 and 5.
7. p_enemy.c `lookforplayers-rear-accept-within-melee`: plain design; the source comment says "if real close, react anyway" (p_enemy.c:547-549).
8. p_enemy.c `alook-wake-sound-random-groups`: plain design (random choice among the see sounds, p_enemy.c:635-651).
9. p_enemy.c `chase-reactiontime-does-not-delay`: plain design; reactiontime gates only the missile decision (p_enemy.c:212-213, 676-677).
10. p_enemy.c `chase-justattacked-nightmare-skips-newchasedir`: plain design of nightmare/fast (p_enemy.c:719).
11. p_enemy.c `chase-active-sound-draw-always`: plain design; a 3-in-256 chance is a draw by nature (p_enemy.c:771-775).
12. p_enemy.c `sposattack-shared-aim-sequential-pellets`: plain design; one aim per volley, pellets resolved in order (p_enemy.c:835-842), as for the player's shotgun.
13. p_enemy.c `skullattack-sound-no-zero-guard`: no behaviour of the original; the only user (MT_SKULL) has a nonzero attack sound, so the missing guard never acts (p_enemy.c:1431).

Not dropped although its sweeper suggested it: p_enemy.c `reachable-pdiehi-only-commercial` is merged into the hard-death-scream row, which other sweeps recorded as a quirk of the C.

**Chunk 3 (p_floor.c, p_inter.c, p_lights.c, p_local.h, p_user.c):**

- `bonuscount-has-no-cap`: plain design. The counter is decremented every tic and the palette index is clamped (st_stuff.c:1031-1034); no overflow is reachable. The "no cap" remark is a Rust bound comment (crates/doom-render/src/d_display.rs:640-641).
- `3. P_GiveAmmo case am_misl has no break`: no behavioural effect; the fall-through lands in an empty `default: break;`. The entry itself says "Effect: none".
- `medikit-and-armor-names-gotmega-is-blue-armor`: identifier naming only. GOTMEGA's text is "Picked up the MegaArmor!" (d_englsh.h:81), the blue armor's own name; nothing differs in play.
- `health-bonus-collects-at-cap-and-counts`: plain design ("Bonuses are always collected, even at the cap", COMPATIBILITY#original-health-and-armor-bonuses; the sweeper marked it intended).
- `soulsphere-always-picked-up`: plain design (the sweeper marked it design).
- `player-mirrored-health-vs-mobj-health`: plain design, nothing observable; M4B §7 finding 20 records that the clamped player health and the signed body health agree on every `> 0` test.
- `damage-armor-saved-capped-clears-type-before-subtract`: plain design of armor absorption; clearing the type before the subtraction has no effect on the result.
- `xdeath-needs-health-strictly-below-minus-spawnhealth`: a boundary choice, not a defect (the sweeper says so); the same E entry's tics part is merged into p_inter.c:726.
- `player-attacker-null-does-not-clear-target`: plain design; `P_DeathThink` already handles a null or self attacker (p_user.c:200), and a player body's `target` is not used to choose anything here.

**Chunk 4 (p_map.c, p_maputl.c):**

- `checkposition-center-sector-vs-extreme-sector`: while a box straddles sectors, the centre's sector can differ from the sector giving the highest floor. Dropped as plain design: the C comment at p_map.c:402-405 states the intent ("The base floor / ceiling is from the subsector that contains the point. Any contacted lines the step closer together will adjust them"), and the sweep itself rated it inherent to the design.
- Part of `rocket-direct-hit-then-splash-and-post-explosion-half-step`: a rocket that hits a thing damages it in PIT_CheckThing and then again in the explosion's P_RadiusAttack (the sweep entry's wording) (p_map.c:323-324, p_enemy.c:1600). Dropped as plain design (the rocket's impact-plus-splash damage). The entry's other half (the post-explosion half step) is the p_mobj.c:150 row, so the entry is counted as merged, not dropped.

**Chunk 5 (p_mobj.c, p_plats.c, p_pspr.c, p_saveg.c, p_setup.c):**

Nine entries, each with the reason (every C line named was opened):

1. `setmobjstate-null-no-field-update` — plain design: entering S_NULL removes the actor; leaving its tics, sprite and frame unassigned has no consequence (nothing observable).
2. `corpse-friction-exception` — plain design: the comment states the intent ("do not stop sliding if halfway off a step with some momentum").
3. `corpse-friction-exception-at-ledges` — duplicate of 2, same reason.
4. `player-stop-threshold-angle-only` — plain design: a player stops below STOPSPEED only without forward or side input; turning is not movement.
5. `nogravity-momz-survives-explosion` — not a quirk: NOGRAVITY simply skips gravity, and P_ExplodeMissile zeroes momz at p_mobj.c:92, so nothing "survives"; M3H2 itself makes no observation claim (momz is always zero there).
6. `terminal-countkill-respawn-gate` — plain design: the nightmare / -respawn timer (12*35 updates, leveltime&31, P_Random() > 4) as written; no accident is described.
7. `missile-launcher-and-bfg-do-not-autofire` — plain design: the comment says "the missile launcher and bfg do not auto fire". The entry's disagreement is a documentation slip only: M4A 3.5.3 says G_PlayerReborn sets attackdown, docs/validation/M4A.md says usedown; g_game.c:822 sets both (`p->usedown = p->attackdown = true;`).
8. `chaingun-flash-state-offset-arithmetic` — plain design: the flash frame is chosen by offset in the contiguous state table, which is how the table is laid out; nothing observable.
9. `alower-does-not-clamp-height-for-a-live-player` — no behaviour: the unclamped height is overwritten at once by P_BringUpWeapon (`sy = WEAPONBOTTOM`, p_pspr.c:151) or the psprite is set to S_NULL (p_pspr.c:409), so it is never displayed or read. The A_Lower half of batch E's `psprite-lower-no-clamp-and-flash-birth-tic` is dropped for the same reason; its flash half is merged into p_pspr.c:875.

Parts of merged entries not carried as quirks (the entries themselves are merged, not dropped): the extra `1 + P_Random() % tics` start-frame draw (p_mobj.c:787-788, E `spawnmapthing-extra-random-and-order`, intended randomisation); A_Saw's hit test reading the aim trace's `linetarget` (G1b, ordinary design); puff and blood before damage (E `shoot-effects-before-damage-and-jitter`, an order in p_map.c, left to the p_map.c rows); the floor `newspecial` and door `topcountdown` allocations (H2, G1a, left to the p_floor.c and p_doors.c rows).

**Chunk 6 (p_sight.c, p_spec.c, p_switch.c, p_telept.c, p_tick.c, p_user.c, r_bsp.c, r_data.c, r_defs.h):**

- E `crossbspnode-side2-visits-both-children` (p_sight.c:257-290, verified): plain design. Mapping a start side 2 to 0 and crossing the other child when the target side differs is what the C comment at p_sight.c:275 says it intends (the comment at p_sight.c:275 reads `an "on" should cross both sides`); "preserving the actual source order" is a port requirement, not a quirk.
- E `checksight-reject-same-sector` (p_sight.c:313-326, verified): plain design. REJECT is a precomputed table the engine is meant to obey; a set bit hiding a same-sector monster is map data, not engine behaviour (the M3D3A note itself calls asymmetric bits "legitimate test inputs").
- H2 `thinker-function-pointer-sentinel` (p_tick.c:108, verified): plain design idiom with no behavioural effect; the archive entry is a harness observation ("GNU compiler-profile observations"), not a quirk of play.
- E `playermove-run-entry-only-from-s-play` (p_user.c:166-170, verified): plain design (the run animation starts only from the standing state, so pain is not interrupted; the sweeper also judged "no" for a small change).
- H2 `dead-player-gets-no-hint-to-press-use` (p_user.c:227-228, verified as the BT_USE reborn test): plain design of the original (no on-screen hint exists to be missing); the source is a usability note of the port.
- G2 `composite-texture-over-64k-errors` (r_data.c:365-369, verified): plain design, a deliberate checked limit with an error message that keeps 16-bit column offsets valid.

**Chunk 7 (r_draw.c, r_main.c, r_plane.c, r_segs.c, r_sky.c, r_things.c, s_sound.c, sounds.c, sounds.h, st_lib.c, st_stuff.c, st_stuff.h, tables.c, v_video.c, w_wad.c, sndserv/wadread.c, wi_stuff.c, z_zone.c):**

1. renderer-vanilla-limits-and-undefined-reads — an aggregate list entry with no C lines; each item it lists is its own row here (r_plane.c:245, r_plane.c:384, r_bsp.c:88, r_segs.c:385, r_things.c:326, r_things.c:815, r_draw.c:142, r_draw.c:274, r_main.c:870, r_segs.c:181, r_segs.c:81), and COMPAT is cited in each. (The other two aggregate entries, renderer limits (checklist rows) and renderer-limits-refuse-or-drop-silently, give C lines and sources and were merged into those rows rather than dropped.)
2. makefile-flags-line-numbers-differ — a documentation slip, not a quirk of the original: M5C §1.1.0 cites the Makefile CFLAGS line as Makefile:9 (correct) and §1.3.0 item 1 as Makefile:8 with CC :6 and LIBS :10 (actual CC :7, CFLAGS :9, LIBS :11; the sweeper verified). Recorded here for the documentation owner; no document is changed (see the page of disagreements).
3. adjustsoundparams-volume-arithmetic-overflows — a Rust-port fault: doom-sound's public `adjust_sound_params` overflowed and panicked in a debug build for a volume outside 0..127 (COMPAT#what-the-sound-code-refuses: "before, such a value overflowed and panicked in a debug build. The session's own configuration is always checked, so no game reaches it"). In the original every value of snd_SfxVolume reaches S_AdjustSoundParams only after S_SetSfxVolume's 0..127 check (s_sound.c:172, 634-635; the menu also calls it), and 127 x 1040 cannot overflow, so the C has no reachable overflow at s_sound.c:812-815. The genuine config-range behaviour is the row s_sound.c:634.
4. Six named files: a coverage listing of comment lines read, not a quirk entry.
5. Other files of G1a's range: a coverage listing, not a quirk entry.
