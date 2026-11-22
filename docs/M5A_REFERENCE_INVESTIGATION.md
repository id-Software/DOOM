# M5a: the faithful software renderer (the 3D view): reference investigation

Phase 1 note for milestone M5a, which adds a pixel-exact software renderer for the 3D view: the framebuffer that
`R_RenderPlayerView` draws (BSP, walls, planes, sky, lighting, sprites, weapon sprites and fuzz), written in safe
Rust in a new crate, compared byte for byte against the genuine linuxdoom-1.10 renderer on synthetic rooms in CI
and on the real Episode 1 maps locally. This note records the original C facts the renderer must reproduce, the
genuine-engine harness design with the result of a scratch prototype, the Rust-side design and state inventory, the
owner decisions R1–R16 with one recommendation each, and the Phase 2 plan, size estimate and cap.

The behavioural authority is the original id Software Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. Milestone status lives in [ROADMAP](ROADMAP.md), support in
[COMPATIBILITY](COMPATIBILITY.md), schemas in [STATE_FORMAT](STATE_FORMAT.md) and commands in [TESTING](TESTING.md)
(see the [documentation authority map](README.md#documentation-authority)). Observations of this milestone are
in `docs/validation/M5A.md`. This note records only counts, pass/fail facts and digests from
runs against the IWAD: no lump content, no IWAD-derived names, coordinates or geometry.

**Merged since: PR #61, by the owner, on 2026-11-17. The status below is as written before that.** **Status / next step (2026-11-17 05:45 PST).** The full real-map re-check (61 runs, 0 refused) and the synthetic genuine check passed after the review fixes, and two 30-minute AddressSanitizer fuzz campaigns (leak detection on) found nothing ([evidence](validation/M5A.md#full-re-checks-and-fuzz-campaigns-after-the-review-fixes)); the owner accepted the declared refusals on 2026-11-16. Phase 2 steps 1-7a are done, step 7b's documentation is written and the independent review's findings are resolved (§7 items 40-48; [evidence](validation/M5A.md#independent-review); [support](COMPATIBILITY.md#software-renderer-m5a); [ROADMAP](ROADMAP.md)). The renderer matches the genuine engine on every comparable synthetic frame and on every frame of the 61 scripted real-map runs (local). Timings are recorded (R15, [evidence](validation/M5A.md#full-re-checks-and-fuzz-campaigns-after-the-review-fixes)). The `Mutants` run's survivors, including the failed shard 0 re-run locally, are dispositioned and independently reviewed ([evidence](validation/M5A.md#mutation-testing)): 272 survivor mutants (268 distinct descriptions), 199 killed by new tests, 5 by the new LANDING room's genuine goldens, 15 loop timeouts, 53 equivalent; the review's disagreements are resolved by four adopted tests and three new synthetic rooms (LANDING, SEGANGLE, MASKPAD; [evidence](validation/M5A.md#independent-review-of-the-dispositions-2026-11-17)). The final gates from clean passed at `3238ee3` ([evidence](validation/M5A.md#final-gates-from-clean)). Next step: mark PR #61 ready for the owner. Half-done work: none. Open: the patchless column (§7 item 32) and a Medusa room (item 34) still need an owner decision on the graphics PWAD; the THING-loading truncation outside commercial mode is a recorded gap with a proposed follow-up ([COMPATIBILITY](COMPATIBILITY.md#known-gap-thing-loading-stops-at-the-first-doom-ii-type-outside-commercial-mode)). Resume: `git checkout feat/m5a-renderer && git pull`; commands in [TESTING](TESTING.md#m5a-synthetic-renderer-rooms).

## Summary

- **Outcome of Phase 2 (2026-11-16).** The Rust renderer (`crates/doom-render`) implements R1-R16 as confirmed and
  matches the genuine renderer on every synthetic frame and on every frame of the scripted real-map runs, with no
  simulation change; observations are in [the evidence record](validation/M5A.md) and support in
  [COMPATIBILITY](COMPATIBILITY.md#software-renderer-m5a). The bullets below are the Phase 1 findings.
- **Limits.** Vanilla's renderer limits behave in three different ways (§1.8, §2.9): `MAXDRAWSEGS` (256) and
  `MAXVISSPRITES` (128) drop silently and deterministically; `MAXVISPLANES`, `MAXOPENINGS` and the solid-seg array
  are checked late or not at all (writes past the arrays happen before any `I_Error`). R3 reproduces the first two
  and refuses the frame with a named witness for the rest.
- **Floating point.** All floating point in the world renderer files is inside `#if 0`; the only live use reachable
  is the `double` division in `FixedDiv2` (m_fixed.c:69-87) (§1.10, §2.8). Renderer arithmetic otherwise wraps
  (R12), and the genuine renderer overflows on ordinary frames.
- **`validcount` is harmless.** The renderer increments the global `validcount` and writes sector stamps, but
  every game reader tests them only after its own increment, so rendering cannot change a gameplay outcome (§1.9,
  §2.7).
- **`ML_MAPPED` is the one dump field rendering changes.** `R_StoreWallRange` sets bit 256 of `linedef->flags`
  (r_segs.c:398). In the prototype, rendering every tic changed that bit and nothing else on the real maps
  (2,377,888 dump rows; never-render runs equalled the frozen runner exactly); synthetic WADs never showed it (R5).
- **Undefined texels.** Wall columns index `& 127` whatever the texture height and composites are not cleared, so
  some genuine pixels depend on memory layout; the prototype saw a pixel differ between two strict runs of the same
  binary (R4).
- **`viewz` is not stored but is derivable.** The Rust state holds no `viewz`, `viewheight` or
  `deltaviewheight`; a renderer-owned view shadow observing every committed tic re-derives them exactly and is
  validated per tic against the genuine runner (§4.1.2, R6).
- **Reusable-switch parity is not derivable.** After a re-press of an SR/GR line no canonical field or event
  records which texture shows; R7 asks to refuse such frames with a named witness (the brief says to stop and ask).
- **`fuzzpos` persists.** The fuzz table position is never reset and the view rectangle is never cleared, so
  spectre pixels and uncovered pixels depend on every earlier frame; R8 fixes a frame-sequence contract.
- **Prototype result.** In a scratch prototype (not committed) the genuine renderer compiled whole into a forked
  runner reproduced every committed M4b dump digest on the 22 synthetic scripts at three view configurations (66
  strict runs), and the strict, address and overflow frame streams were byte-identical with 14 declared sanitizer
  adapters plus one LP64 adapter (§3.0). On the real maps 60 strict runs (157,256 frames) exited 0.
- **Nothing in the simulation changes** under any recommendation (§5, §4.0).
- **Size.** About 13k–17k lines against a 25k cap on code, tools and docs, with a stated fallback (§6.2, §6.3).

## 1. C facts: view setup, BSP, walls, planes, sky, tables and limits

The facts in this section come from reading `linuxdoom-1.10/` at the authority commit; all citations are
`linuxdoom-1.10/<file>:<line>`. Nothing was built or run for it except a Python token count of the `tables.c`
arrays (§1.7). `RANGECHECK` is defined (doomdef.h:75), so every `#ifdef RANGECHECK` block cited below is live
in the reference build. Scope: `r_main.c`, `r_bsp.c`, `r_segs.c`, `r_plane.c`, `r_sky.c` and `tables.c`; things,
column and span drawers, textures and view inputs are §2.

### 1.1 R_RenderPlayerView path (r_main.c)

Caller: `D_Display` calls `R_ExecuteSetViewSize()` first if `setsizeneeded`
(d_main.c:215-217), then `R_RenderPlayerView(&players[displayplayer])` only when
`gamestate == GS_LEVEL && !automapactive && gametic` (d_main.c:267-268). The status bar
(`ST_Drawer`, d_main.c:245) and `HU_Erase` (d_main.c:231-232) run before it, `HU_Drawer`
after it (d_main.c:270-271), border code after that (d_main.c:278-293).

Order inside `R_RenderPlayerView` (r_main.c:870-898):

1. `R_SetupFrame(player)` (r_main.c:872), body r_main.c:830-863:
   - `viewplayer = player` (834); `viewx = player->mo->x` (835); `viewy = player->mo->y` (836);
     `viewangle = player->mo->angle + viewangleoffset` (837; `viewangleoffset` is a global
     int, r_main.c:52, never written anywhere in the tree; the only other mention is an unused extern at d_net.c:553, so it is 0);
     `extralight = player->extralight` (838); `viewz = player->viewz` (840).
   - `viewsin = finesine[viewangle>>ANGLETOFINESHIFT]` (842); `viewcos = finecosine[...]` (843),
     `finecosine = &finesine[FINEANGLES/4]` (r_main.c:113).
   - `sscount = 0` (845).
   - Fixed colormap (847-859): if `player->fixedcolormap` nonzero,
     `fixedcolormap = colormaps + player->fixedcolormap*256*sizeof(lighttable_t)`
     (849-851; `lighttable_t` is `byte`, r_defs.h:314, so offset = n*256);
     `walllights = scalelightfixed` (853) and all 48 `scalelightfixed[i] = fixedcolormap`
     (855-856). Else `fixedcolormap = 0` (859) (walllights is NOT reset here; it is set per
     seg in R_StoreWallRange). Producers: `player->fixedcolormap = INVERSECOLORMAP` (p_user.c:367),
     `= 1` light amp (p_user.c:377), `= 0` (p_user.c:369,380,383; p_mobj.c:682).
     `extralight` producers: p_pspr.c:763/768/773 (0/1/2), p_mobj.c:681.
   - `framecount++` (861); `validcount++` (862).
2. `R_ClearClipSegs()` (875) r_bsp.c:245-252: `solidsegs[0] = {-0x7fffffff, -1}`,
   `solidsegs[1] = {viewwidth, 0x7fffffff}`, `newend = solidsegs+2`.
3. `R_ClearDrawSegs()` (876) r_bsp.c:68-71: `ds_p = drawsegs`.
4. `R_ClearPlanes()` (877) r_plane.c:185-209 (§1.4).
5. `R_ClearSprites()` (878) r_things.c:317-320: `vissprite_p = vissprites`.
6. `NetUpdate()` (881), `R_RenderBSPNode(numnodes-1)` (884), `NetUpdate()` (887),
   `R_DrawPlanes()` (889), `NetUpdate()` (892), `R_DrawMasked()` (894), `NetUpdate()` (897).
7. `R_DrawMasked` (r_things.c:958-987, §2.1.8): sorts and draws vissprites back to front
   (962-975), then every drawseg with `maskedtexturecol` from `ds_p-1` down to `drawsegs`
   via `R_RenderMaskedSegRange(ds, ds->x1, ds->x2)` (978-980), then psprites if
   `!viewangleoffset` (984-985). R_DrawSprite also calls `R_RenderMaskedSegRange(ds,r1,r2)`
   for partial ranges (r_things.c:892).

NetUpdate (d_net.c:368-...): reads `I_GetTime()`, and for each new tic calls `I_StartTic`,
`D_ProcessEvents`, `G_BuildTiccmd(&localcmds[maketic%BACKUPTICS])`, `maketic++`
(d_net.c:377-410), then (unless `singletics`, 413-414) network sends. It builds input ticcmds
only; it does not run game tics, does not touch renderer globals and is wall-clock driven.
Headless with a no-op/disabled NetUpdate the frame output is unchanged; it matters only for
input timing in the live engine (the events it processes go through responders, which a
headless harness does not feed).

### 1.2 r_bsp.c

Globals (r_bsp.c:47-54): `curline, sidedef, linedef, frontsector, backsector`,
`drawseg_t drawsegs[MAXDRAWSEGS]`, `ds_p`. `MAXSEGS 32` (88); `newend` (91);
`cliprange_t solidsegs[MAXSEGS]` (92), `cliprange_t = {int first; int last;}` (80-85).

**R_RenderBSPNode(bspnum)** (552-578): if `bspnum & NF_SUBSECTOR`: `bspnum == -1` ->
`R_Subsector(0)` (560-561) else `R_Subsector(bspnum & ~NF_SUBSECTOR)` (563); return. Else
`side = R_PointOnSide(viewx, viewy, &nodes[bspnum])` (570), recurse front
`children[side]` (573), then back `children[side^1]` only if `R_CheckBBox(bsp->bbox[side^1])`
(576-577). Recursive (C stack).

**R_PointOnSide** (r_main.c:161-211): `dx==0`: `x <= node->x ? node->dy > 0 : node->dy < 0`
(172-178); `dy==0`: `y <= node->y ? node->dx < 0 : node->dx > 0` (179-185); else
`dx=x-node->x, dy=y-node->y` (187-188); sign fast path
`(node->dy ^ node->dx ^ dx ^ dy) & 0x80000000` -> return `((node->dy ^ dx) & 0x80000000) != 0`
(191-199); else `left = FixedMul(node->dy>>FRACBITS, dx)`,
`right = FixedMul(dy, node->dx>>FRACBITS)`, return `right < left ? 0 : 1` (201-210). Same code
as the game's `R_PointInSubsector` path (r_main.c:800-823), already shared with simulation.

**R_CheckBBox(bspcoord)** (381-487): `checkcoord[12][4]` (365-378) has 11 initializer rows:
`{3,0,2,1},{3,0,2,0},{3,1,2,0},{0},{2,0,2,1},{0,0,0,0},{3,1,3,0},{0},{2,0,3,1},{2,1,3,1},{2,1,3,0}`;
row 11 is implicit zeros. `boxx`: `viewx <= BOXLEFT` 0, `< BOXRIGHT` 1, else 2 (404-409);
`boxy`: `viewy >= BOXTOP` 0, `> BOXBOTTOM` 1, else 2 (411-416); `boxpos = (boxy<<2)+boxx`
(418), `== 5` -> true (419-420); rows 3,7,11 are unreachable. Corners
`x1=bspcoord[cc[0]], y1=[cc[1]], x2=[cc[2]], y2=[cc[3]]` (422-425).
`angle1 = R_PointToAngle(x1,y1) - viewangle`, `angle2 = ... (x2,y2) - viewangle` (428-429);
`span = angle1 - angle2` (431); `span >= ANG180` -> true (434-435). Left clip:
`tspan = angle1 + clipangle; if (tspan > 2*clipangle) { tspan -= 2*clipangle; if (tspan >= span) return false; angle1 = clipangle; }`
(437-448); right clip symmetric with `tspan = clipangle - angle2`, `angle2 = -clipangle`
(449-459). `sx1 = viewangletox[(angle1+ANG90)>>ANGLETOFINESHIFT]`, `sx2 = ...angle2...`
(465-468); `sx1 == sx2` -> false (471-472); `sx2--` (473); find first `start` with
`start->last >= sx2` (475-477); `sx1 >= start->first && sx2 <= start->last` -> false (479-484);
else true (486). All angle arithmetic is `angle_t` (unsigned, tables.h:78) modular.

**R_Subsector(num)** (497-542): RANGECHECK `num >= numsubsectors` ->
`I_Error("R_Subsector: ss %i with numss = %i", ...)` (503-508). `sscount++` (510);
`frontsector = sub->sector` (512); `count = sub->numlines`, `line = &segs[sub->firstline]` (513-514).
`floorplane = frontsector->floorheight < viewz ? R_FindPlane(floorheight, floorpic, lightlevel) : NULL`
(516-523). `ceilingplane = (ceilingheight > viewz || ceilingpic == skyflatnum) ? R_FindPlane(ceilingheight, ceilingpic, lightlevel) : NULL`
(525-533). `R_AddSprites(frontsector)` (535; writes `sec->validcount`, r_things.c:622-626).
Then `R_AddLine(line)` for each seg in order (537-541). The renderer never writes
`sector->validcount` in R_Subsector itself; only R_AddSprites does.

**R_AddLine(line)** (259-356): `curline = line` (268); `angle1 = R_PointToAngle(v1)`,
`angle2 = R_PointToAngle(v2)` (271-272); `span = angle1 - angle2` (276); `span >= ANG180` ->
return (backface, 279-280); `rw_angle1 = angle1` (283, global int in r_segs.c:60);
`angle1 -= viewangle; angle2 -= viewangle` (284-285); same two clip blocks as R_CheckBBox but
returning (287-307); `x1 = viewangletox[(angle1+ANG90)>>ANGLETOFINESHIFT]`, `x2 = ...` (311-314);
`x1 == x2` -> return (317-318). `backsector = line->backsector` (320).
- No backsector -> `clipsolid` (323-324).
- Closed door: `backsector->ceilingheight <= frontsector->floorheight || backsector->floorheight >= frontsector->ceilingheight` -> `clipsolid` (327-329).
- Window: ceiling heights differ or floor heights differ -> `clippass` (332-334).
- Empty-line reject: `ceilingpic ==`, `floorpic ==`, `lightlevel ==` and `curline->sidedef->midtexture == 0` (raw, untranslated) -> return without clipping or drawing (341-347).
- Otherwise falls to `clippass: R_ClipPassWallSegment(x1, x2-1)` (350-352);
  `clipsolid: R_ClipSolidWallSegment(x1, x2-1)` (354-355).

**R_ClipSolidWallSegment(first,last)** (103-185): find first `start` with
`start->last >= first-1` (113-115; terminates on sentinel `solidsegs[1].last = 0x7fffffff`).
If `first < start->first`: if `last < start->first-1` -> `R_StoreWallRange(first,last)`, then
insert a new post by shifting `[start, newend)` up one (`next = newend; newend++;` copy down,
124-133) and return; else store `(first, start->first-1)` and `start->first = first`
(137-141). If `last <= start->last` return (144-145). Loop while
`last >= (next+1)->first-1`: store the gap `(next->last+1, (next+1)->first-1)`, `next++`, and if
`last <= next->last` then `start->last = next->last; goto crunch` (147-161). Store
`(next->last+1, last)`, `start->last = last` (164-166). `crunch`: if `next == start` return;
else compact `while (next++ != newend) *++start = *next;` and `newend = start+1` (170-184).
**No bounds check on `newend++` (124-125).**

**R_ClipPassWallSegment(first,last)** (196-238): same search; stores visible fragments
(214, 219, 229, 237) but never modifies `solidsegs`.

**R_PointToAngle(x,y)** (r_main.c:291-374): `x -= viewx; y -= viewy` (296-297); both 0 ->
0 (299-300). Octants via `tantoangle[SlopeDiv(num,den)]`: x>=0,y>=0: x>y `tantoangle[SD(y,x)]`
(312) else `ANG90-1-tantoangle[SD(x,y)]` (317); x>=0,y<0 (y=-y): x>y `-tantoangle[SD(y,x)]`
(328) else `ANG270+tantoangle[SD(x,y)]` (333); x<0 (x=-x), y>=0: x>y `ANG180-1-tantoangle[SD(y,x)]`
(348) else `ANG90+tantoangle[SD(x,y)]` (353); y<0: x>y `ANG180+tantoangle[SD(y,x)]` (364) else
`ANG270-1-tantoangle[SD(x,y)]` (369). Subtractions `x - viewx` are signed 32-bit (wrap on
overflow for coordinates > 32767 units apart); negation of `INT_MIN` is UB in C.
**R_PointToAngle2** (r_main.c:377-388) WRITES `viewx = x1; viewy = y1` then calls R_PointToAngle.

**SlopeDiv(num,den)** (tables.c:50-62, unsigned args): `den < 512` -> `SLOPERANGE` (56-57);
`ans = (num<<3)/(den>>8)` (59, `num<<3` wraps mod 2^32 when `num >= 2^29`);
`return ans <= SLOPERANGE ? ans : SLOPERANGE` (61). Rust already has `slope_div`
(crates/doom-sim/src/movement.rs:534) and `tan_to_angle.rs`.

**R_PointToDist(x,y)** (r_main.c:391-418): `dx = abs(x - viewx)`, `dy = abs(y - viewy)`
(402-403); swap so `dx >= dy` (405-410);
`angle = (tantoangle[FixedDiv(dy,dx)>>DBITS] + ANG90) >> ANGLETOFINESHIFT` (412; `angle` is
`int`, but the shifted operand is unsigned so the value is 0..8191);
`dist = FixedDiv(dx, finesine[angle])` (415). Only called by R_StoreWallRange (r_segs.c:408)
and r_things (not by game code).

**R_FakeFlat**: confirmed absent: `grep -n "FakeFlat\|fakeflat" *.c *.h` in linuxdoom-1.10
returns nothing. Frontsector/backsector are the raw `sub->sector` (r_bsp.c:512) and
`line->backsector` (r_bsp.c:320); masked segs re-derive from `curline->frontsector/backsector`
(r_segs.c:118-119).

### 1.3 r_segs.c

Globals r_segs.c:46-95 (`segtextured, markfloor, markceiling, maskedtexture, toptexture,
bottomtexture, midtexture, rw_normalangle, rw_angle1 (int), rw_x, rw_stopx, rw_centerangle,
rw_offset, rw_distance, rw_scale, rw_scalestep, rw_midtexturemid, rw_toptexturemid,
rw_bottomtexturemid, worldtop, worldbottom, worldhigh, worldlow, pixhigh, pixlow,
pixhighstep, pixlowstep, topfrac, topstep, bottomfrac, bottomstep, walllights,
maskedtexturecol (short*)`).

**R_StoreWallRange(start, stop)** (374-745), in order:
1. `if (ds_p == &drawsegs[MAXDRAWSEGS]) return;` comment "don't overflow and crash" (385-387). **Yes, linuxdoom-1.10 has this check.**
2. RANGECHECK `start >= viewwidth || start > stop` -> `I_Error("Bad R_RenderWallRange: %i to %i", start, stop)` (389-392).
3. `sidedef = curline->sidedef; linedef = curline->linedef` (394-395); **`linedef->flags |= ML_MAPPED`** (398; ML_MAPPED = 256, doomdata.h:135).
4. `rw_normalangle = curline->angle + ANG90` (401); `offsetangle = abs(rw_normalangle - rw_angle1)` (402: unsigned difference converted to int for `abs`, result stored to angle_t); clamp `> ANG90` -> ANG90 (404-405); `distangle = ANG90 - offsetangle` (407); `hyp = R_PointToDist(v1)` (408); `rw_distance = FixedMul(hyp, finesine[distangle>>ANGLETOFINESHIFT])` (409-410).
5. `ds_p->x1 = rw_x = start; ds_p->x2 = stop; ds_p->curline = curline; rw_stopx = stop+1` (413-416).
6. `ds_p->scale1 = rw_scale = R_ScaleFromGlobalAngle(viewangle + xtoviewangle[start])` (419-420); if `stop > start`: `scale2 = R_ScaleFromGlobalAngle(viewangle + xtoviewangle[stop])`, `scalestep = rw_scalestep = (scale2 - rw_scale) / (stop-start)` (C int division truncating toward zero) (422-427); else `scale2 = scale1` (445) and **`rw_scalestep` / `ds_p->scalestep` are NOT written** (stale from the previous seg; harmless for the loop since it runs one column, but `ds_p->scalestep` is an uninitialized/stale drawseg field read by r_things / R_RenderMaskedSegRange: `spryscale = ds->scale1 + (x1 - ds->x1)*rw_scalestep` with `x1 == ds->x1` gives 0 contribution, so stale value is multiplied by 0 there; the §2 should check R_DrawSprite's use of `ds->scalestep`).
7. `worldtop = frontsector->ceilingheight - viewz; worldbottom = frontsector->floorheight - viewz` (450-451); all textures/flags zeroed, `ds_p->maskedtexturecol = NULL` (453-454).
8. Single-sided (456-481): `midtexture = texturetranslation[sidedef->midtexture]` (459); `markfloor = markceiling = true` (461); ML_DONTPEGBOTTOM: `vtop = frontsector->floorheight + textureheight[sidedef->midtexture]` (untranslated texture's height), `rw_midtexturemid = vtop - viewz` (462-467); else `rw_midtexturemid = worldtop` (472); `+= sidedef->rowoffset` (474); `silhouette = SIL_BOTH; sprtopclip = screenheightarray; sprbottomclip = negonearray; bsilheight = MAXINT; tsilheight = MININT` (476-480).
9. Two-sided (482-614): clips NULL, `silhouette = 0` (485-486). `front floor > back floor` -> SIL_BOTTOM, `bsilheight = front floor` (488-492); else `back floor > viewz` -> SIL_BOTTOM, `bsilheight = MAXINT` (493-498). `front ceil < back ceil` -> |= SIL_TOP, `tsilheight = front ceil` (500-504); else `back ceil < viewz` -> |= SIL_TOP, `tsilheight = MININT` (505-510). `back ceil <= front floor` -> `sprbottomclip = negonearray; bsilheight = MAXINT; |= SIL_BOTTOM` (512-517). `back floor >= front ceil` -> `sprtopclip = screenheightarray; tsilheight = MININT; |= SIL_TOP` (519-524). `worldhigh = back ceil - viewz; worldlow = back floor - viewz` (526-527). **Sky hack**: front and back `ceilingpic == skyflatnum` -> `worldtop = worldhigh` (530-534). `markfloor = worldlow != worldbottom || floorpic differ || lightlevel differ` (537-547). `markceiling = worldhigh != worldtop || ceilingpic differ || lightlevel differ` (550-560; uses the sky-hacked worldtop). Closed door (same test as R_AddLine) -> both true (562-567). Top texture if `worldhigh < worldtop`: `toptexture = texturetranslation[sidedef->toptexture]`; ML_DONTPEGTOP -> `rw_toptexturemid = worldtop`, else `vtop = back ceil + textureheight[sidedef->toptexture]` (untranslated), `rw_toptexturemid = vtop - viewz` (570-588). Bottom texture if `worldlow > worldbottom`: `bottomtexture = texturetranslation[sidedef->bottomtexture]`; ML_DONTPEGBOTTOM -> `rw_bottomtexturemid = worldtop` else `worldlow` (589-602). Both mids `+= sidedef->rowoffset` unconditionally (603-604). Masked: `if (sidedef->midtexture)` (raw) -> `maskedtexture = true; ds_p->maskedtexturecol = maskedtexturecol = lastopening - rw_x; lastopening += rw_stopx - rw_x` (607-613; no MAXOPENINGS check).
10. `segtextured = midtexture | toptexture | bottomtexture | maskedtexture` (617). If set (619-658): `offsetangle = rw_normalangle - rw_angle1`; `> ANG180` -> negate; `> ANG90` -> ANG90 (621-627); `rw_offset = FixedMul(hyp, finesine[offsetangle>>ANGLETOFINESHIFT])` (629-630); `if (rw_normalangle - rw_angle1 < ANG180) rw_offset = -rw_offset` (632-633); `rw_offset += sidedef->textureoffset + curline->offset` (635); `rw_centerangle = ANG90 + viewangle - rw_normalangle` (636). Light only if `!fixedcolormap` (642-657): `lightnum = (frontsector->lightlevel >> LIGHTSEGSHIFT) + extralight` (644); fake contrast: `v1->y == v2->y` -> `lightnum--`, else `v1->x == v2->x` -> `lightnum++` (646-649); `walllights = scalelight[clamp(lightnum, 0, LIGHTLEVELS-1)]` (651-656).
11. `frontsector->floorheight >= viewz` -> `markfloor = false` (665-669); `ceilingheight <= viewz && ceilingpic != skyflatnum` -> `markceiling = false` (671-676). These match exactly the NULL conditions of floorplane/ceilingplane in R_Subsector (r_bsp.c:516, 525-526), so a marked plane is never NULL.
12. `worldtop >>= 4; worldbottom >>= 4` (680-681, arithmetic shift of signed int); `topstep = -FixedMul(rw_scalestep, worldtop)`; `topfrac = (centeryfrac>>4) - FixedMul(worldtop, rw_scale)`; same for bottom (683-687). If backsector: `worldhigh >>= 4; worldlow >>= 4`; `pixhigh`/`pixhighstep` only if `worldhigh < worldtop` (comparison now on shifted values) (694-698); `pixlow`/`pixlowstep` only if `worldlow > worldbottom` (700-704). Note this re-tests the conditions on the >>4 values; the toptexture/bottomtexture decision used unshifted values (570, 589). They can disagree only if the two heights differ by less than 16/65536 unit across a 16-subunit boundary; since `viewz` is subtracted from both, the difference equals the sector-height difference, which is a multiple of FRACUNIT whenever sector heights are integral (true for map data; movers' speeds are FRACUNIT multiples in vanilla). Otherwise pixhigh/pixlow would be stale globals (see hazards).
13. `markceiling` -> `ceilingplane = R_CheckPlane(ceilingplane, rw_x, rw_stopx-1)` (708-709); `markfloor` -> `floorplane = R_CheckPlane(floorplane, ...)` (711-712).
14. `R_RenderSegLoop()` (714).
15. Sprite clip save: if `((silhouette & SIL_TOP) || maskedtexture) && !sprtopclip`: `memcpy(lastopening, ceilingclip+start, 2*(rw_stopx-start)); sprtopclip = lastopening - start; lastopening += rw_stopx - start` (718-724); bottom likewise from `floorclip` (726-732). No MAXOPENINGS check.
16. `maskedtexture && !(silhouette & SIL_TOP)` -> `|= SIL_TOP, tsilheight = MININT` (734-738); bottom -> `|= SIL_BOTTOM, bsilheight = MAXINT` (739-743). `ds_p++` (744).

**R_ScaleFromGlobalAngle(visangle)** (r_main.c:453-502; live code 480-501):
`anglea = ANG90 + (visangle - viewangle)`; `angleb = ANG90 + (visangle - rw_normalangle)`
(480-481, **`anglea`/`angleb` are `int`**, 456-457); `sinea = finesine[anglea>>ANGLETOFINESHIFT]`,
`sineb = finesine[angleb>>ANGLETOFINESHIFT]` (484-485; **signed right shift**: if the
angle sum is >= 0x80000000 the index is negative and reads before `finesine`);
`num = FixedMul(projection, sineb) << detailshift` (486); `den = FixedMul(rw_distance, sinea)` (487);
`if (den > num>>16) { scale = FixedDiv(num, den); clamp to [256, 64*FRACUNIT] }` (489-497) else
`scale = 64*FRACUNIT` (499).

**R_RenderSegLoop()** (206-364): `HEIGHTBITS 12`, `HEIGHTUNIT (1<<12)` (203-204). For
`rw_x < rw_stopx` (219):
- `yl = (topfrac + HEIGHTUNIT-1) >> HEIGHTBITS`; `yl < ceilingclip[rw_x]+1` -> `yl = ceilingclip+1` (222-226).
- markceiling: `top = ceilingclip+1; bottom = yl-1; bottom >= floorclip -> floorclip-1; if top <= bottom: ceilingplane->top[rw_x] = top; ->bottom[rw_x] = bottom` (228-241; stored as `byte`).
- `yh = bottomfrac >> HEIGHTBITS`; `yh >= floorclip -> floorclip-1` (243-246).
- markfloor: `top = yh+1; bottom = floorclip-1; top <= ceilingclip -> ceilingclip+1; if top <= bottom: floorplane->top/bottom` (248-259).
- segtextured: `angle = (rw_centerangle + xtoviewangle[rw_x]) >> ANGLETOFINESHIFT` (unsigned); `texturecolumn = (rw_offset - FixedMul(finetangent[angle], rw_distance)) >> FRACBITS`; `index = rw_scale >> LIGHTSCALESHIFT` (unsigned), clamp `>= MAXLIGHTSCALE` -> 47; `dc_colormap = walllights[index]; dc_x = rw_x; dc_iscale = 0xffffffffu / (unsigned)rw_scale` (262-277). With fixedcolormap, `walllights == scalelightfixed` (all entries fixedcolormap).
- midtexture: `dc_yl = yl; dc_yh = yh; dc_texturemid = rw_midtexturemid; dc_source = R_GetColumn(midtexture, texturecolumn); colfunc(); ceilingclip[rw_x] = viewheight; floorclip[rw_x] = -1` (280-290). The draw is unconditional even if `yl > yh` (column drawer handles count < 0; §2).
- else two-sided (291-358): top: `mid = pixhigh >> HEIGHTBITS; pixhigh += pixhighstep; mid >= floorclip -> floorclip-1; mid >= yl ? draw [yl,mid] with rw_toptexturemid, ceilingclip = mid : ceilingclip = yl-1` (294-314); no top: `if (markceiling) ceilingclip = yl-1` (315-320). bottom: `mid = (pixlow + HEIGHTUNIT-1) >> HEIGHTBITS; pixlow += pixlowstep; mid <= ceilingclip -> ceilingclip+1; mid <= yh ? draw [mid,yh] with rw_bottomtexturemid, floorclip = mid : floorclip = yh+1` (322-344); no bottom: `if (markfloor) floorclip = yh+1` (345-350). maskedtexture: `maskedtexturecol[rw_x] = texturecolumn` (352-357; int truncated to `short`).
- `rw_scale += rw_scalestep; topfrac += topstep; bottomfrac += bottomstep` (360-362).
- `midtexture == 0` on a single-sided line (texture "-" -> number 0, r_data.c:701) takes the two-sided branch with no top/bottom: nothing is drawn, clip arrays still close (318-319, 348-349 with markfloor/markceiling true) and solidsegs is closed by R_ClipSolidWallSegment -> stale framebuffer pixels remain (HOM). Same for missing upper/lower textures (texture 0).

**R_RenderMaskedSegRange(ds, x1, x2)** (102-190): `curline = ds->curline; frontsector = curline->frontsector; backsector = curline->backsector` (117-119); `texnum = texturetranslation[curline->sidedef->midtexture]` (120). Light: same fake-contrast lightnum and `walllights = scalelight[...]` (122-134), computed even when fixedcolormap (it then overrides via dc_colormap). `maskedtexturecol = ds->maskedtexturecol` (136); `rw_scalestep = ds->scalestep; spryscale = ds->scale1 + (x1 - ds->x1)*rw_scalestep` (138-139); `mfloorclip = ds->sprbottomclip; mceilingclip = ds->sprtopclip` (140-141). Pegging: ML_DONTPEGBOTTOM -> `dc_texturemid = max(front floor, back floor) + textureheight[texnum] - viewz` (translated texnum here, unlike r_segs.c:465/583) (144-149); else `min(front ceil, back ceil) - viewz` (150-155); `+= sidedef->rowoffset` (156). `if (fixedcolormap) dc_colormap = fixedcolormap` (158-159). For `dc_x = x1..x2` (162): if `maskedtexturecol[dc_x] != MAXSHORT` (0x7fff, doomtype.h:45): if !fixedcolormap `index = spryscale>>LIGHTSCALESHIFT` clamp 47, `dc_colormap = walllights[index]` (167-175); `sprtopscreen = centeryfrac - FixedMul(dc_texturemid, spryscale)`; `dc_iscale = 0xffffffffu / (unsigned)spryscale` (177-178); `col = (column_t*)((byte*)R_GetColumn(texnum, maskedtexturecol[dc_x]) - 3)` (181-182); `R_DrawMaskedColumn(col)` (184, r_things.c:351, §2); `maskedtexturecol[dc_x] = MAXSHORT` (185). `spryscale += rw_scalestep` every column (187).

### 1.4 r_plane.c

Globals: `MAXVISPLANES 128` (52), `visplanes[128]`, `lastvisplane`, `floorplane`, `ceilingplane`
(53-56); `MAXOPENINGS SCREENWIDTH*64` = 20480 (59), `short openings[MAXOPENINGS]`, `lastopening`
(60-61); `short floorclip[SCREENWIDTH], ceilingclip[SCREENWIDTH]` (69-70);
`int spanstart[SCREENHEIGHT], spanstop[SCREENHEIGHT]` (76-77); `planezlight, planeheight` (82-83);
`yslope[SCREENHEIGHT], distscale[SCREENWIDTH], basexscale, baseyscale` (85-88);
`cachedheight/cacheddistance/cachedxstep/cachedystep[SCREENHEIGHT]` (90-93).
`visplane_t` (r_defs.h:460-480): `height, picnum, lightlevel, minx, maxx`, `byte pad1`,
`byte top[320]`, `byte pad2, pad3`, `byte bottom[320]`, `byte pad4`.

**R_InitPlanes** (101-104): empty.

**R_ClearPlanes** (185-209): `floorclip[i] = viewheight; ceilingclip[i] = -1` for `i < viewwidth`
(191-195); `lastvisplane = visplanes; lastopening = openings` (197-198);
`memset(cachedheight, 0, ...)` (201); `angle = (viewangle - ANG90) >> ANGLETOFINESHIFT` (204);
`basexscale = FixedDiv(finecosine[angle], centerxfrac)`; `baseyscale = -FixedDiv(finesine[angle], centerxfrac)` (207-208).
yslope/distscale are NOT computed here; they are computed in R_ExecuteSetViewSize (§1.6).

**R_FindPlane(height, picnum, lightlevel)** (217-259): sky (`picnum == skyflatnum`) ->
`height = 0; lightlevel = 0` (225-229). Linear search `visplanes..lastvisplane` for equal
`(height, picnum, lightlevel)`, first match returned (231-243; no x-range test).
`if (lastvisplane - visplanes == MAXVISPLANES) I_Error("R_FindPlane: no more visplanes");`
(245-246). New: `lastvisplane++`, set fields, `minx = SCREENWIDTH; maxx = -1`,
`memset(check->top, 0xff, sizeof(check->top))` (248-258; bottom not cleared).

**R_CheckPlane(pl, start, stop)** (265-324): `intrl/unionl` and `intrh/unionh` from
`start` vs `pl->minx`, `stop` vs `pl->maxx` (277-297); scan `x = intrl..intrh` for
`pl->top[x] != 0xff` (299-301); none -> `minx = unionl; maxx = unionh`, return `pl` (303-310).
Else new plane copying `height, picnum, lightlevel` into `*lastvisplane`, `pl = lastvisplane++`,
`minx = start; maxx = stop`, memset top 0xff (313-323). **No MAXVISPLANES check here.**

**R_MakeSpans(x, t1, b1, t2, b2)** (330-359): `while (t1 < t2 && t1 <= b1) R_MapPlane(t1, spanstart[t1], x-1), t1++`;
`while (b1 > b2 && b1 >= t1) R_MapPlane(b1, spanstart[b1], x-1), b1--`;
`while (t2 < t1 && t2 <= b2) spanstart[t2++] = x`; `while (b2 > b1 && b2 >= t2) spanstart[b2--] = x`.
`spanstop` is unused.

**R_MapPlane(y, x1, x2)** (120-178): RANGECHECK `x2 < x1 || x1 < 0 || x2 >= viewwidth || (unsigned)y > viewheight`
-> `I_Error("R_MapPlane: %i, %i at %i", x1, x2, y)` (131-139; note `> viewheight`, off by one
permissive). Cache: if `planeheight != cachedheight[y]`: `cachedheight[y] = planeheight;
distance = cacheddistance[y] = FixedMul(planeheight, yslope[y]); ds_xstep = cachedxstep[y] = FixedMul(distance, basexscale);
ds_ystep = cachedystep[y] = FixedMul(distance, baseyscale)` else reuse (141-153). Caches are
reset to 0 per frame (r_plane.c:201); `planeheight` is never 0 for a drawn flat because floors
need `floorheight < viewz` and non-sky ceilings `ceilingheight > viewz` (r_bsp.c:516,525), so a
stale cache is never hit. `length = FixedMul(distance, distscale[x1])`;
`angle = (viewangle + xtoviewangle[x1]) >> ANGLETOFINESHIFT`;
`ds_xfrac = viewx + FixedMul(finecosine[angle], length)`;
`ds_yfrac = -viewy - FixedMul(finesine[angle], length)` (155-158). Light: fixedcolormap ->
`ds_colormap = fixedcolormap`; else `index = distance >> LIGHTZSHIFT` (unsigned), clamp `>= MAXLIGHTZ` -> 127,
`ds_colormap = planezlight[index]` (160-170). `ds_y = y; ds_x1 = x1; ds_x2 = x2; spanfunc()` (172-177).

**R_DrawPlanes** (367-453): RANGECHECK post-hoc checks (375-387):
`ds_p - drawsegs > MAXDRAWSEGS` -> `I_Error("R_DrawPlanes: drawsegs overflow (%i)", ...)`;
`lastvisplane - visplanes > MAXVISPLANES` -> `I_Error("R_DrawPlanes: visplane overflow (%i)", ...)`;
`lastopening - openings > MAXOPENINGS` -> `I_Error("R_DrawPlanes: opening overflow (%i)", ...)`.
For each plane in creation order (389): skip `minx > maxx` (391-392).
Sky (396-420): `dc_iscale = pspriteiscale >> detailshift` (398); `dc_colormap = colormaps` (404,
always colormap 0: ignores light, extralight and fixedcolormap, so invulnerability does not
invert the sky); `dc_texturemid = skytexturemid` (405); for `x = minx..maxx`:
`dc_yl = top[x]; dc_yh = bottom[x]`; if `dc_yl <= dc_yh`: `angle = (viewangle + xtoviewangle[x]) >> ANGLETOSKYSHIFT`
(22), `dc_x = x; dc_source = R_GetColumn(skytexture, angle); colfunc()` (406-418). Note an
unset column has `top[x] = 0xff` (255) and stale `bottom[x]`; since `dc_yl` is int from a byte,
255 > any bottom (<= 199) so it is skipped.
Flat (423-451): `ds_source = W_CacheLumpNum(firstflat + flattranslation[pl->picnum], PU_STATIC)` (423-425);
`planeheight = abs(pl->height - viewz)` (427); `light = (pl->lightlevel >> LIGHTSEGSHIFT) + extralight`
clamped to `[0, LIGHTLEVELS-1]` (428-434; no fake contrast); `planezlight = zlight[light]` (436);
`pl->top[pl->maxx+1] = 0xff; pl->top[pl->minx-1] = 0xff` (438-439; writes into pad1/pad2 or
bottom[] neighbours... `top[320]` is pad2 and `top[-1]` is pad1); `stop = maxx+1`; for
`x = minx..stop`: `R_MakeSpans(x, top[x-1], bottom[x-1], top[x], bottom[x])` (441-449);
`Z_ChangeTag(ds_source, PU_CACHE)` (451). Bytes are unsigned so 0xff=255 acts as "empty".
openings[] usage: masked texture column arrays (r_segs.c:611-612) and sprite clip copies
(r_segs.c:721-723, 729-731); drawsegs point into it.

### 1.5 r_sky.c / sky selection

- `skyflatnum`, `skytexture`, `skytexturemid` globals (r_sky.c:47-49). `R_InitSkyMap` sets only
  `skytexturemid = 100*FRACUNIT` (r_sky.c:57-61; the `skyflatnum` line is commented out, 59).
  Called once from R_Init (r_main.c:788); not called on view-size change despite the comment.
- `SKYFLATNAME "F_SKY1"` (r_sky.h:32); `ANGLETOSKYSHIFT 22` (r_sky.h:35): 1024 sky columns per 360°.
- `skyflatnum = R_FlatNumForName(SKYFLATNAME)` at every `G_DoLoadLevel` (g_game.c:454).
- `skytexture` in `G_DoLoadLevel` (g_game.c:458-468) only for `commercial`/`pack_tnt`/`pack_plut`:
  `SKY3`, overridden to `SKY1` if `gamemap < 12`, else `SKY2` if `gamemap < 21`.
- `skytexture` in `G_InitNew` (g_game.c:1365-...; sky block 1455-1478): commercial same rule;
  else by `episode`: 1 `SKY1`, 2 `SKY2`, 3 `SKY3`, 4 `SKY4`. For non-commercial games
  G_DoLoadLevel does not change it, so the sky follows the episode chosen at G_InitNew.
- Sky is untranslated (`R_GetColumn(skytexture, ...)`, not via texturetranslation, r_plane.c:415).
  Sky planes are matched by raw `picnum == skyflatnum` (r_plane.c:225, 396; r_bsp.c:526; r_segs.c:530-531, 672).

### 1.6 r_main.c tables and setup

**R_Init** (r_main.c:773-794): `R_InitData` (775: textures, flats, sprite lumps, colormaps;
r_data.c:654-663), `R_InitPointToAngle` (777; body is `#if 0`, 426-443), `R_InitTables` (779;
body `#if 0`, 509-537), `R_SetViewSize(screenblocks, detailLevel)` (783; only sets
`setsizeneeded/setblocks/setdetail`, 657-665), `R_InitPlanes` (784, empty), `R_InitLightTables`
(786), `R_InitSkyMap` (788), `R_InitTranslationTables` (790), `framecount = 0` (793). The view
size is actually applied by `R_ExecuteSetViewSize` from the first `D_Display` (d_main.c:215-217).
Colormaps: `colormaps = Z_Malloc(length+255)` aligned with `((int)colormaps + 255) & ~0xff`
(r_data.c:641-642; pointer-to-int cast, a 64-bit hazard for the reference build).

**Tables are literal data**: R_InitPointToAngle and R_InitTables are compiled out; finesine,
finetangent and tantoangle come from tables.c (§1.7).

**R_InitTextureMapping** (544-603): `FIELDOFVIEW 2048` (48).
`focallength = FixedDiv(centerxfrac, finetangent[FINEANGLES/4 + FIELDOFVIEW/2])` (557-558; index 3072).
For `i < 4096` (560): `finetangent[i] > 2*FRACUNIT` -> `t = -1`; `< -2*FRACUNIT` -> `t = viewwidth+1`;
else `t = FixedMul(finetangent[i], focallength); t = (centerxfrac - t + FRACUNIT-1) >> FRACBITS`,
clamp to `[-1, viewwidth+1]`; `viewangletox[i] = t` (562-576). For `x = 0..viewwidth`:
`i = 0; while (viewangletox[i] > x) i++; xtoviewangle[x] = (i << ANGLETOFINESHIFT) - ANG90` (582-588).
Fencepost: `viewangletox == -1` -> 0, `== viewwidth+1` -> viewwidth (591-600; the
`t = FixedMul(...); t = centerx - t` there is dead). `clipangle = xtoviewangle[0]` (602).
Arrays: `viewangletox[FINEANGLES/2]` (98), `xtoviewangle[SCREENWIDTH+1]` (103). Integer only
(FixedMul/FixedDiv).

**R_InitLightTables** (612-642): `DISTMAP 2` (612). For `i < LIGHTLEVELS`:
`startmap = ((LIGHTLEVELS-1-i)*2)*NUMCOLORMAPS/LIGHTLEVELS` (626); for `j < MAXLIGHTZ`:
`scale = FixedDiv(SCREENWIDTH/2*FRACUNIT, (j+1) << LIGHTZSHIFT) >> LIGHTSCALESHIFT; level = startmap - scale/DISTMAP`,
clamp `[0, NUMCOLORMAPS-1]`, `zlight[i][j] = colormaps + level*256` (627-640). Independent of
view size (uses SCREENWIDTH, not viewwidth). Constants (r_main.h:69-87): `LIGHTLEVELS 16`,
`LIGHTSEGSHIFT 4`, `MAXLIGHTSCALE 48`, `LIGHTSCALESHIFT 12`, `MAXLIGHTZ 128`, `LIGHTZSHIFT 20`,
`NUMCOLORMAPS 32`. Tables: `scalelight[16][48]`, `scalelightfixed[48]`, `zlight[16][128]`
(r_main.c:116-118).

**R_ExecuteSetViewSize** (671-761): `setsizeneeded = false` (680); `setblocks == 11` ->
`scaledviewwidth = 320; viewheight = 200` (682-686) else `scaledviewwidth = setblocks*32;
viewheight = (setblocks*168/10) & ~7` (688-691). `detailshift = setdetail;
viewwidth = scaledviewwidth >> detailshift` (693-694). `centery = viewheight/2; centerx = viewwidth/2;
centerxfrac = centerx<<FRACBITS; centeryfrac = centery<<FRACBITS; projection = centerxfrac` (696-700).
Drawers: detail 0 -> `colfunc = basecolfunc = R_DrawColumn; fuzzcolfunc = R_DrawFuzzColumn;
transcolfunc = R_DrawTranslatedColumn; spanfunc = R_DrawSpan` (702-708); else `R_DrawColumnLow`,
`R_DrawFuzzColumn`, `R_DrawTranslatedColumn`, `R_DrawSpanLow` (709-715).
`R_InitBuffer(scaledviewwidth, viewheight)` (717); `R_InitTextureMapping()` (719);
`pspritescale = FRACUNIT*viewwidth/SCREENWIDTH; pspriteiscale = FRACUNIT*SCREENWIDTH/viewwidth` (722-723);
`screenheightarray[i] = viewheight` for `i < viewwidth` (726-727);
`yslope[i] = FixedDiv((viewwidth<<detailshift)/2*FRACUNIT, abs(((i - viewheight/2)<<FRACBITS) + FRACUNIT/2))`
for `i < viewheight` (730-735); `distscale[i] = FixedDiv(FRACUNIT, abs(finecosine[xtoviewangle[i]>>ANGLETOFINESHIFT]))`
for `i < viewwidth` (737-741); `scalelight[i][j] = colormaps + clamp(startmap - j*SCREENWIDTH/(viewwidth<<detailshift)/DISTMAP, 0, 31)*256`
(745-760, left-to-right integer division).

**R_InitBuffer(width, height)** (r_draw.c:696-722): `viewwindowx = (SCREENWIDTH - width) >> 1`
(706); `columnofs[i] = viewwindowx + i` for `i < width` (709-710); `viewwindowy = width == SCREENWIDTH ? 0 : (SCREENHEIGHT - SBARHEIGHT - height) >> 1`
(713-715; `SBARHEIGHT 32`, r_draw.c:51); `ylookup[i] = screens[0] + (i + viewwindowy)*SCREENWIDTH` (718-719).

Defaults: `{"screenblocks", &screenblocks, 9}`, `{"detaillevel", &detailLevel, 0}` (m_misc.c:279-280;
globals m_menu.c:83-84). Menu sizes: `screenblocks` 3..11 (`screenSize = screenblocks - 3`,
range 0..8, m_menu.c:1854, 1155-1173). `M_ChangeDetail` toggles `detailLevel` then returns
before `R_SetViewSize` with "low detail mode n.a." (m_menu.c:1131-1147): low detail is reachable
only via a config file `detaillevel 1` at startup.

Derived geometry (computed from the formulas above):

| setblocks | scaledviewwidth x viewheight | viewwindowx, viewwindowy | centerx, centery (detail 0) |
| --- | --- | --- | --- |
| 3 | 96 x 48 | 112, 60 | 48, 24 |
| 4 | 128 x 64 | 96, 52 | 64, 32 |
| 5 | 160 x 80 | 80, 44 | 80, 40 |
| 6 | 192 x 96 | 64, 36 | 96, 48 |
| 7 | 224 x 112 | 48, 28 | 112, 56 |
| 8 | 256 x 128 | 32, 20 | 128, 64 |
| 9 (default) | 288 x 144 | 16, 12 | 144, 72 |
| 10 | 320 x 168 | 0, 0 | 160, 84 |
| 11 | 320 x 200 | 0, 0 | 160, 100 |

Border/screen layout: the 3D view writes only `screens[0]` rows `viewwindowy..+viewheight-1`,
columns `viewwindowx..+scaledviewwidth-1` (via ylookup/columnofs). For `scaledviewwidth == 320`
(blocks 10, 11) `R_FillBackScreen` and `R_DrawViewBorder` return immediately (r_draw.c:746-747,
849-850). Otherwise `R_FillBackScreen` tiles `FLOOR7_2` (Doom) / `GRNROCK` (commercial) into
`screens[1]` rows `0..167` and draws brdr_* patches around the window (r_draw.c:753-821);
it runs when the level state is entered (d_main.c:278-282). `R_DrawViewBorder` copies
`screens[1] -> screens[0]` for the top band, bottom band and side strips only, never the view
rectangle (r_draw.c:843-875), for `borderdrawcount` frames (d_main.c:285-293). The status bar
occupies rows 168-199 for blocks <= 10 (ST_Drawer, d_main.c:245). **The renderer never clears
the view rectangle**: any column the BSP/planes/sprites do not cover keeps the previous
frame's bytes (HOM), and with full-width views HU_Erase does not erase message text inside the
view (hu_lib.c:154-165 erases only when `viewwindowx` != 0).

### 1.7 tables.c / tables.h

- `FINEANGLES 8192`, `FINEMASK 8191`, `ANGLETOFINESHIFT 19` (tables.h:50-55).
- `ANG45 0x20000000`, `ANG90 0x40000000`, `ANG180 0x80000000`, `ANG270 0xc0000000` (tables.h:68-71).
- `SLOPERANGE 2048`, `SLOPEBITS 11`, `DBITS (FRACBITS-SLOPEBITS)` = 5 (tables.h:74-76); `typedef unsigned angle_t` (78).
- `int finetangent[4096]` (tables.c:67-581): 4096 literal entries, first `-170910304`, last `170910304`.
- `int finesine[10240]` (tables.c:584-1866): 10240 literal entries, first `25`, last two `65535`. Declared extern as `finesine[5*FINEANGLES/4]` (tables.h:58).
- `angle_t tantoangle[2049]` (tables.c:1870-2129): 2049 literal entries, first `0`, last `536870912` (= ANG45).
- Entry counts verified by tokenizing the initializers with `python3 -I` (regex over tables.c): 4096 / 10240 / 2049.
- `PI 3.141592657` is defined only if `!LINUX` (tables.h:41-45) and used only in `#if 0` code.
- In tables.c finetangent precedes finesine in source order (67, 584); whether they are adjacent in memory is compiler/linker dependent.
- Rust already carries `FINE_SINE: [i32; 10240]` (crates/doom-sim/src/fine_sine.rs:4) and a
  tantoangle table (crates/doom-sim/src/movement.rs:6 `include!("tan_to_angle.rs")`); no
  finetangent table was found in crates (grep for `finetangent|FINETANGENT`): it must be added
  from tables.c:67-581.

### 1.8 Vanilla limits and overflow behaviour (linuxdoom-1.10 exactly)

| Limit | Definition | Overflow behaviour in linuxdoom-1.10 |
| --- | --- | --- |
| MAXSEGS = 32 (solidsegs) | r_bsp.c:88, 92 | **Unchecked.** `newend++` (r_bsp.c:124-125) and the shift loop write past `solidsegs[31]`. Number of posts is bounded only by geometry: two sentinels plus at most ceil(viewwidth/2) disjoint, non-adjacent posts (adjacent posts merge, r_bsp.c:114,148), i.e. up to 162 entries at viewwidth 320, so overflow is possible in principle (needs > 30 separated solid fragments in one frame). Writes corrupt whatever follows `solidsegs` in .bss/common storage; r_bsp.c's other uninitialized globals are `curline, sidedef, linedef, frontsector, backsector, drawsegs, ds_p, newend` (47-54, 91) but actual adjacency is compiler/linker dependent (gcc `-fcommon` common symbols). Effect is layout-dependent; no I_Error. |
| MAXDRAWSEGS = 256 | r_defs.h:55; drawsegs r_bsp.c:53 | **Silent drop**: `if (ds_p == &drawsegs[MAXDRAWSEGS]) return;` (r_segs.c:386-387) before any write (including the ML_MAPPED write). The caller still updates solidsegs (R_ClipSolidWallSegment), so the columns stay undrawn (stale pixels, no plane marks, no clip update). The RANGECHECK `> MAXDRAWSEGS` test in R_DrawPlanes (r_plane.c:376-378) can never fire. |
| MAXVISPLANES = 128 | r_plane.c:52-53 | R_FindPlane: `== MAXVISPLANES` -> `I_Error("R_FindPlane: no more visplanes")` (r_plane.c:245-246). **R_CheckPlane is unchecked** (r_plane.c:313-317): it writes `*lastvisplane` and increments past the array (each visplane_t is 20+1+320+2+320+1 = 664 bytes before padding, r_defs.h:460-480). Once R_CheckPlane pushes the count past 128, R_FindPlane's `==` test is skipped forever. At frame end RANGECHECK `lastvisplane - visplanes > MAXVISPLANES` -> `I_Error("R_DrawPlanes: visplane overflow (%i)", ...)` (r_plane.c:380-382), but only after the out-of-bounds writes (which may have already corrupted `lastvisplane`, `floorplane`, `ceilingplane`, `openings`, ... depending on layout) and only if execution reaches it. Exactly 128 via R_CheckPlane and no further R_FindPlane-new: then no error (count == 128 is not > 128). |
| MAXOPENINGS = 20480 shorts | r_plane.c:59-61 | **Unchecked during the frame**: `lastopening += ...` (r_segs.c:612, 723, 731) and `memcpy` (721, 729) plus `maskedtexturecol[rw_x] = ...` (r_segs.c:356) write past `openings`. Post-hoc RANGECHECK `> MAXOPENINGS` -> `I_Error("R_DrawPlanes: opening overflow (%i)", ...)` (r_plane.c:384-386). Corruption before the check is layout-dependent. Worst case per frame is far below the limit for 320 columns only with few masked/silhouette segs; each such seg uses up to 3x its width. |
| MAXVISSPRITES = 128 | r_things.h:31, r_things.c:289 | Silent: `R_NewVisSprite` returns `&overflowsprite` once full (r_things.c:330-331); details for the §2. |
| Recursion depth | R_RenderBSPNode | C stack recursion, depth = BSP depth; no limit check. |

### 1.9 Renderer/game shared globals and whether rendering can change game outcomes

Globals written by the world renderer that game code also touches:

1. **`validcount`** (r_main.c:55): renderer `validcount++` once per frame (r_main.c:862).
   Game increments: P_NoiseAlert (p_enemy.c:164), P_TeleportMove (p_map.c:151), P_CheckPosition
   (p_map.c:409), P_PathTraverse (p_maputl.c:774), P_CheckSight (p_sight.c:332).
2. **`sector_t.validcount`** (r_defs.h:123-124): renderer writes `sec->validcount = validcount`
   in R_AddSprites (r_things.c:622-626) after testing equality. Game: P_RecursiveSound tests
   `sec->validcount == validcount && sec->soundtraversed <= soundblocks+1` and writes it
   (p_enemy.c:115-121), always after P_NoiseAlert's increment (p_enemy.c:164-165).
3. **`line_t.validcount`** (r_defs.h:210-211): renderer never touches it. Game: P_BlockLinesIterator
   (p_maputl.c:497-500; callers P_CheckPosition p_map.c:438 after p_map.c:409, P_PathTraverse
   p_maputl.c:850 after 774) and P_CrossSubsector (p_sight.c:171-174, after p_sight.c:332).
4. **`mobj_t.validcount`** (p_mobj.h:246-247): no reader or writer anywhere in the tree.

Argument: every reader of a validcount stamp (game: p_enemy.c:115, p_maputl.c:497,
p_sight.c:171; renderer: r_things.c:622) is preceded in the same traversal by its own
`validcount++`, and every writer stores the then-current `validcount`. Because `validcount`
only increases, after any increment no stored stamp can equal the current value, regardless
of how many extra increments (renderer frames) occurred in between. So the extra per-frame
increment and the renderer's sector stamps cannot change any equality test outcome in game
code, and game stamps cannot change the renderer's. The only theoretical dependency is
signed-int wraparound of `validcount` after ~2^31 increments (signed overflow is UB in C),
where the renderer would shift the wrap point; unreachable in practice. `validcount` and
sector/line stamps are not saved in savegames (P_ArchiveWorld writes only specific fields,
p_saveg.c:130-160; the archived `mobj_t` memcpy includes the unused `mobj->validcount`).
**Verdict: rendering cannot change game-logic outcomes via validcount**; but the absolute
value of `validcount` and `sector->validcount` differ between "render" and "no-render" C runs,
so they must not be part of any canonical state compared across such runs.

Other renderer writes to game-visible state:

5. **`line_t.flags |= ML_MAPPED`** (r_segs.c:398) for every stored wall range. Readers: automap
   (am_map.c:1128) and the savegame (`*put++ = li->flags`, p_saveg.c:141; restored 195). No
   `p_*` gameplay reader of bit 256: the only code that masks line flags in a way that clears it,
   `door->line->flags &= ML_BLOCKING^0xff` (p_doors.c:637), is inside `#if 0` (p_doors.c:557-764).
   So rendering changes line flags (and savegame bytes) but not gameplay.
6. **`viewx`/`viewy`**: game code calls `R_PointToAngle2`, which overwrites `viewx/viewy`
   (r_main.c:384-385): p_mobj.c:906, p_user.c:202, p_pspr.c:490/526, p_enemy.c:537/789/1051,
   p_inter.c:812, p_map.c:610/615, st_stuff.c:814, s_sound.c:780. No game code reads
   `viewx/viewy/viewz/viewangle` (grep over p_*, g_*, am_*, st_*, hu_*, s_*, d_*, wi_*, m_*),
   and R_SetupFrame re-derives them each frame, so neither direction leaks. Game code does not
   call `R_PointToAngle` (one-argument-pair, viewx-relative) or `R_PointToDist`.
7. Zone memory: R_DrawPlanes `W_CacheLumpNum(..., PU_STATIC)` / `Z_ChangeTag(PU_CACHE)`
   (r_plane.c:423-425, 451), R_GetColumn lump caching (r_data.c:394-395) and composite texture
   generation (r_data.c, §2.3) change zone heap layout/purge state. No gameplay result
   depends on addresses, but a zone exhaustion `I_Error` could in principle become reachable
   differently; memory-only effect.
8. The renderer reads but does not write: `player->mo->{x,y,angle}`, `player->{viewz,extralight,fixedcolormap}`
   (r_main.c:835-840, 847), sector heights/pics/lightlevel, sidedef offsets/textures,
   `texturetranslation[]`/`flattranslation[]` (animated by game code), seg/vertex/node data.
   No lazy caches in `sector_t` exist in vanilla (no soundorg/blockbox mutation by r_*).

### 1.10 Floating point in these files

All floating point in r_main.c, r_bsp.c, r_segs.c, r_plane.c, r_sky.c and tables.c is
inside `#if 0`:
- r_main.c:432-440 (R_InitPointToAngle): `f = atan((float)i/SLOPERANGE)/(3.141592657*2); t = 0xffffffff*f;` (`#if 0` 429-442).
- r_main.c:514-534 (R_InitTables): `a = (i-FINEANGLES/4+0.5)*PI*2/FINEANGLES; fv = FRACUNIT*tan(a);` and `a = (i+0.5)*PI*2/FINEANGLES; t = FRACUNIT*sin(a);` (`#if 0` 512-535).
- `#include <math.h>` (r_main.c:32) is otherwise unused.
- R_InitTextureMapping (544-603) and R_InitLightTables (614-642) are integer-only (FixedMul/FixedDiv).

Live floating point reachable from the world renderer is **FixedDiv2** (m_fixed.c:69-87):
`c = ((double)a) / ((double)b) * FRACUNIT;` with `c >= 2147483648.0 || c < -2147483648.0`
-> `I_Error("FixedDiv: divide by zero")`, `return (fixed_t)c` (truncation toward zero), reached
when FixedDiv's guard `(abs(a)>>14) >= abs(b)` is false (m_fixed.c:57-65; guard returns
`(a^b)<0 ? MININT : MAXINT`). Renderer FixedDiv call sites: r_main.c:412, 415 (R_PointToDist),
491 (R_ScaleFromGlobalAngle), 557 (focallength), 629 (zlight), 734 (yslope), 740 (distscale);
r_plane.c:207-208 (basexscale/baseyscale). FixedMul is `long long` integer (m_fixed.c:43-49).
The Rust side already ports FixedDiv2 as IEEE binary64 divide-then-multiply
(crates/doom-types/src/fixed.rs:76), which is exact for IEEE-754 double with
round-to-nearest (a/b correctly rounded, *65536 exact as a power of two, then truncation).
No float tables need committing: every table is either literal (tables.c) or computed with
integer ops plus FixedDiv2, so an integer port reproduces them; optionally cross-check
viewangletox/xtoviewangle/yslope/distscale/zlight/scalelight against a C dump per
(setblocks, detail).


### 1.11 Hazards for a byte-exact port

1. `R_ScaleFromGlobalAngle` uses `int anglea/angleb` with signed `>>19` (r_main.c:456-457, 484-485); an angle sum >= 0x80000000 indexes `finesine` negatively (reads memory before the array, layout-dependent). Port must use i32 arithmetic shift and either prove the index stays in range or refuse.
2. `R_PointToDist` with the viewpoint exactly on `v1` gives `FixedDiv(0,0) = MAXINT` (m_fixed.c:62-63) and `tantoangle[MAXINT>>5]`, an out-of-bounds read (r_main.c:412). Reachable when the player stands exactly on a seg's start vertex and the seg survives R_AddLine (angle1 = 0 from r_main.c:299-300).
3. `SlopeDiv` `num<<3` wraps in u32 (tables.c:59); `x - viewx` wraps in i32 (r_main.c:296-297, 402-403); `abs(INT_MIN)` (r_main.c:402-403, r_segs.c:402). Use wrapping ops exactly.
4. `rw_scalestep`/`ds_p->scalestep` not written for single-column ranges (r_segs.c:422-446): stale value from the previous seg is stored into the drawseg only implicitly (field not assigned). Any reader must reproduce staleness or be shown to multiply it by 0.
5. Pegging uses untranslated `textureheight[sidedef->...]` in R_StoreWallRange (r_segs.c:465, 583) but translated in R_RenderMaskedSegRange (r_segs.c:148).
6. `maskedtexturecol` is `short`; `texturecolumn` truncates to 16 bits (r_segs.c:356) and a column of exactly 32767 equals the MAXSHORT sentinel (r_segs.c:165).
7. visplane `top[]/bottom[]` are bytes; 0xff sentinel; `top[minx-1]`/`top[maxx+1]` pads (r_plane.c:438-439); R_FindPlane does not clear `bottom[]`.
8. `toptexture` chosen on unshifted heights (r_segs.c:570, 589) but `pixhigh/pixlow` initialised on `>>4` heights (r_segs.c:694-704); fractional heights can leave pixhigh/pixlow stale (globals persist across segs and frames).
9. Overflows: solidsegs (unchecked), visplanes via R_CheckPlane (unchecked until frame-end I_Error), openings (unchecked until frame-end I_Error) are memory-layout-dependent and cannot be reproduced; they must be refuse-the-frame witnesses. Drawsegs overflow is a deterministic silent drop and can be reproduced exactly.
10. The framebuffer persists: uncovered columns (texture 0 walls, drawseg drop, HOM) show previous-frame bytes, including HUD text drawn by HU_Drawer the previous frame. Exact comparison needs the same prior framebuffer or must refuse frames with uncovered pixels.
11. Sky always uses colormap 0 and `dc_iscale = pspriteiscale>>detailshift` (r_plane.c:398, 404), ignoring fixedcolormap.
12. Order matters: visplanes drawn in creation order (r_plane.c:389); R_FindPlane returns the first match (r_plane.c:231-243); masked segs drawn from the last drawseg backwards (r_things.c:978).
13. ML_MAPPED is set on line flags by rendering (r_segs.c:398): if the Rust canonical state includes line flags, C runs with rendering differ from runs without.
14. Low detail: `viewwidth = scaledviewwidth >> 1`, but yslope uses `(viewwidth<<detailshift)/2` and scale uses `num << detailshift` (r_main.c:734, 486); R_InitBuffer gets scaledviewwidth (r_main.c:717).

## 2. C facts: sprites, column and span drawing, textures, fuzz, weapon sprites and the view height

The facts in this section come from reading `linuxdoom-1.10/` at the authority commit (the prefix is omitted
below: `r_things.c:483` means `linuxdoom-1.10/r_things.c:483`); nothing was built or run for it. Scope: `r_things.c`,
`r_draw.c`, `r_data.c`, `v_video.c` and the view inputs from game state; the world renderer is §1, which is cited
only where a fact here depends on it. `RANGECHECK` is defined unconditionally (doomdef.h:75), so every
`#ifdef RANGECHECK` block below is compiled in and its `I_Error` is fatal. Makefile CFLAGS are
`-g -Wall -DNORMALUNIX -DLINUX` (Makefile:9): no optimisation flag, no `-DUSEASM`. `boolean` is
`enum {false, true}` (doomtype.h:34), int-sized.

### 2.1 r_things.c

#### 2.1.1 Constants and data

- `MINZ = FRACUNIT*4`, `BASEYCENTER = 100` (r_things.c:46-47). `MAXVISSPRITES = 128` (r_things.h:31).
- `LIGHTLEVELS 16`, `LIGHTSEGSHIFT 4`, `MAXLIGHTSCALE 48`, `LIGHTSCALESHIFT 12` (r_main.h:69-73);
  `NUMCOLORMAPS 32` (r_main.h:87).
- `FF_FULLBRIGHT 0x8000`, `FF_FRAMEMASK 0x7fff` (p_pspr.h:50-51). `MF_SHADOW 0x40000`
  (p_mobj.h:167); `MF_TRANSLATION 0xc000000`, `MF_TRANSSHIFT 26` (p_mobj.h:199-201).
- `negonearray[SCREENWIDTH]`, `screenheightarray[SCREENWIDTH]` (r_things.c:81-82):
  `negonearray[i] = -1` for all 320 (r_things.c:303-306); `screenheightarray[i] = viewheight`
  only for `i < viewwidth` (r_main.c:726-727).
- `pspritescale = FRACUNIT*viewwidth/SCREENWIDTH`, `pspriteiscale = FRACUNIT*SCREENWIDTH/viewwidth`
  (r_main.c:722-723): both `FRACUNIT` at viewwidth 320.
- `spriteframe_t { boolean rotate; short lump[8]; byte flip[8]; }` (r_defs.h:427-440);
  `spritedef_t { int numframes; spriteframe_t* spriteframes; }` (r_defs.h:448-453).
- `vissprite_t` fields, order: prev, next, x1, x2, gx, gy, gz, gzt, startfrac, scale, xiscale,
  texturemid, patch, colormap, mobjflags (r_defs.h:375-409).
- `drawseg_t`: curline, x1, x2, scale1, scale2, scalestep, silhouette, bsilheight, tsilheight,
  sprtopclip, sprbottomclip, maskedtexturecol (r_defs.h:322-347); `SIL_NONE 0, SIL_BOTTOM 1,
  SIL_TOP 2, SIL_BOTH 3` (r_defs.h:50-53).

#### 2.1.2 R_InitSprites / R_InitSpriteDefs / R_InstallSpriteLump (startup)

Called as `R_InitSprites (sprnames)` from P_Init (p_setup.c:704).
1. Fill `negonearray` with -1 (r_things.c:303-306), then `R_InitSpriteDefs` (r_things.c:308).
2. `numsprites` = count of the NULL-terminated name list (r_things.c:190-194); return if 0
   (r_things.c:196-197). Lump scan range is `firstspritelump .. lastspritelump` inclusive
   (r_things.c:201-202, 217).
3. Per sprite name i: `memset(sprtemp, -1, sizeof sprtemp)` (r_things.c:210) so `rotate = -1`,
   every `lump[r] = -1`, every `flip[r] = 0xff`; `maxframe = -1` (r_things.c:212).
   `sprtemp` has 29 frames (r_things.c:94).
4. A lump matches when its first 4 name bytes equal the sprite name compared as one `int`
   (r_things.c:213, 219). `frame = name[4]-'A'`, `rotation = name[5]-'0'` (r_things.c:221-222).
   If `modifiedgame`, the lump installed is `W_GetNumForName(name)` (last lump of that name in
   the whole directory, w_wad.c:377-379), else `l` (r_things.c:224-227); installed unflipped
   (r_things.c:229). If `name[6]` is nonzero, a second install uses `frame = name[6]-'A'`,
   `rotation = name[7]-'0'`, lump `l` (not `patched`) and `flipped = true` (r_things.c:231-236).
5. R_InstallSpriteLump (r_things.c:105-157), arguments are `unsigned` frame/rotation:
   - `frame >= 29 || rotation > 8` -> I_Error "Bad frame characters" (r_things.c:114-116); a
     character below 'A'/'0' wraps to a huge unsigned and hits this.
   - `maxframe = max(maxframe, frame)` (r_things.c:118-119).
   - rotation 0: if `rotate == false` -> I_Error "multip rot=0 lump" (r_things.c:124-126); if
     `rotate == true` -> I_Error "has rotations and a rot=0 lump" (r_things.c:128-130); else set
     `rotate = false` and all 8 `lump[r] = lump - firstspritelump`, `flip[r] = flipped`
     (r_things.c:132-138).
   - rotation 1..8: if `rotate == false` -> I_Error (r_things.c:142-144); `rotate = true`
     (r_things.c:146); `rotation--` (r_things.c:149); if `lump[rotation] != -1` -> I_Error "two
     lumps mapped" (r_things.c:150-153); store lump index and flip (r_things.c:155-156).
6. Completeness (r_things.c:241-272): no frames -> `numframes = 0` (r_things.c:241-245); else
   `maxframe++` and for every frame `0..maxframe-1`: rotate == -1 -> I_Error "No patches found"
   (r_things.c:253-257); rotate == 1 needs all 8 lumps else I_Error "missing rotations"
   (r_things.c:263-270). Frames copied to a zone block (r_things.c:275-278).
   Lump numbers stored are relative to `firstspritelump` (r_things.c:135, 155).

#### 2.1.3 Per-frame entry points (execution order)

R_RenderPlayerView (r_main.c:870-898): R_SetupFrame (872), R_ClearClipSegs, R_ClearDrawSegs,
R_ClearPlanes, R_ClearSprites (875-878), NetUpdate (881), R_RenderBSPNode (884), NetUpdate,
R_DrawPlanes (889), NetUpdate, R_DrawMasked (894), NetUpdate (897).

- R_ClearSprites: `vissprite_p = vissprites` (r_things.c:317-320).
- R_SetupFrame inputs relevant here: `viewplayer = player`, `viewx/viewy = mo->x/y`,
  `viewangle = mo->angle + viewangleoffset`, `extralight = player->extralight`,
  `viewz = player->viewz` (r_main.c:834-840); `fixedcolormap = colormaps +
  player->fixedcolormap*256` when nonzero, else 0 (r_main.c:847-859); `validcount++`
  (r_main.c:862). `viewangleoffset` (r_main.c:52) is never assigned anywhere (grep: only
  r_main.c:837, r_things.c:984, extern d_net.c:553), so it is always 0.

#### 2.1.4 R_AddSprites (r_things.c:613-640)

Called once per visited subsector from R_Subsector, after floor/ceiling plane lookup and
before that subsector's segs (r_bsp.c:535).
1. `if (sec->validcount == validcount) return;` then `sec->validcount = validcount`
   (r_things.c:622-626): each sector's things are projected once per frame, on the first
   subsector of that sector reached in BSP front-to-back order.
2. `lightnum = (sec->lightlevel >> LIGHTSEGSHIFT) + extralight` (r_things.c:628); clamp to
   `scalelight[0]` if < 0, `scalelight[LIGHTLEVELS-1]` if >= 16, else `scalelight[lightnum]`
   into global `spritelights` (r_things.c:630-635).
3. `for (thing = sec->thinglist; thing; thing = thing->snext) R_ProjectSprite(thing)`
   (r_things.c:638-639). Thinglist order: P_SetThingPosition inserts at the head
   (p_maputl.c:414-420), unlink at p_maputl.c:356-362. So projection order = BSP order of
   sectors, then most-recently-linked-first within a sector.

#### 2.1.5 R_ProjectSprite (r_things.c:452-604), in order

1. `tr_x = thing->x - viewx; tr_y = thing->y - viewy` (r_things.c:483-484).
2. `gxt = FixedMul(tr_x, viewcos); gyt = -FixedMul(tr_y, viewsin); tz = gxt - gyt`
   (r_things.c:486-489).
3. `if (tz < MINZ) return;` (r_things.c:492-493). The view player's own mobj has tr = 0, tz = 0
   and is always rejected here (viewx/viewy are its x/y, r_main.c:835-836).
4. `xscale = FixedDiv(projection, tz)` (r_things.c:495); `projection = centerxfrac`
   (r_main.c:700). Since tz >= 4*FRACUNIT and centerxfrac <= 160<<16, xscale <= 40<<16: the
   FixedDiv guard never saturates here.
5. `gxt = -FixedMul(tr_x, viewsin); gyt = FixedMul(tr_y, viewcos); tx = -(gyt+gxt)`
   (r_things.c:497-499).
6. `if (abs(tx) > (tz<<2)) return;` (r_things.c:502-503). `tz<<2` is a signed shift; for
   tz >= 0x20000000 it wraps (gcc behaviour; C UB) - see Hazards.
7. RANGECHECK: `(unsigned)thing->sprite >= numsprites` -> I_Error (r_things.c:506-510);
   `(thing->frame & FF_FRAMEMASK) >= sprdef->numframes` -> I_Error (r_things.c:512-516).
   `sprframe = &sprdef->spriteframes[frame & FF_FRAMEMASK]` (r_things.c:517).
8. Rotation (r_things.c:519-532): if `sprframe->rotate`:
   `ang = R_PointToAngle(thing->x, thing->y)` (viewer-relative, uses viewx/viewy) and
   `rot = (ang - thing->angle + (unsigned)(ANG45/2)*9) >> 29` (r_things.c:522-523), all in
   unsigned 32-bit (angle_t) arithmetic; `(unsigned)(ANG45/2)*9 = 0x90000000`. `lump =
   lump[rot]`, `flip = flip[rot]` (r_things.c:524-525). Else `lump[0]`, `flip[0]`
   (r_things.c:530-531).
9. Edges: `tx -= spriteoffset[lump]; x1 = (centerxfrac + FixedMul(tx, xscale)) >> FRACBITS`
   (r_things.c:535-536); `if (x1 > viewwidth) return;` (r_things.c:539-540, note `>` not `>=`);
   `tx += spritewidth[lump]; x2 = ((centerxfrac + FixedMul(tx, xscale)) >> FRACBITS) - 1`
   (r_things.c:542-543); `if (x2 < 0) return;` (r_things.c:546-547).
10. `vis = R_NewVisSprite()` (r_things.c:550). Fields: `mobjflags = thing->flags`,
    `scale = xscale<<detailshift`, `gx, gy, gz = thing->x, y, z`,
    `gzt = thing->z + spritetopoffset[lump]`, `texturemid = gzt - viewz`,
    `x1 = max(x1, 0)`, `x2 = min(x2, viewwidth-1)` (r_things.c:551-559).
11. `iscale = FixedDiv(FRACUNIT, xscale)` (r_things.c:560). Flip: `startfrac =
    spritewidth[lump]-1` (fixed: width*65536 - 1) and `xiscale = -iscale`; else `startfrac = 0`,
    `xiscale = iscale` (r_things.c:562-571). If clipped on the left (`vis->x1 > x1`):
    `startfrac += xiscale*(vis->x1 - x1)` (plain int multiply, r_things.c:573-574).
    `vis->patch = lump` (r_things.c:575).
12. Colormap, first match wins (r_things.c:578-603):
    `thing->flags & MF_SHADOW` -> `NULL` (fuzz); else `fixedcolormap` if nonzero; else
    `frame & FF_FULLBRIGHT` -> `colormaps` (map 0); else
    `index = xscale >> (LIGHTSCALESHIFT - detailshift)`, clamped to `MAXLIGHTSCALE-1`,
    `colormap = spritelights[index]` (r_things.c:597-602). Note it shifts `xscale` (not
    `vis->scale`) by `12-detailshift`, which equals `vis->scale >> 12`.

Case x1 == viewwidth passes step 9, giving `vis->x1 = viewwidth > vis->x2 = viewwidth-1`: an
empty vissprite that still consumes a slot (Hazards).

#### 2.1.6 R_NewVisSprite / overflow (r_things.c:326-335)

`if (vissprite_p == &vissprites[MAXVISSPRITES]) return &overflowsprite;` else post-increment.
At overflow: no message, no error, `vissprite_p` stays at the end, the 129th+ sprite is written
into the static `overflowsprite` (r_things.c:326) and is never sorted or drawn (sort walks only
`vissprites..vissprite_p`, r_things.c:796, 803). Silent drop of every sprite projected after the
128th in projection order.

#### 2.1.7 R_SortVisSprites (r_things.c:784-835), exact algorithm

1. `count = vissprite_p - vissprites`; `unsorted` is a local sentinel; return if 0
   (r_things.c:796-801).
2. Link `vissprites[0..count-1]` in array order into a circular doubly linked list headed by
   `unsorted` (r_things.c:803-812).
3. `vsprsortedhead` (global sentinel) emptied (r_things.c:816). Repeat `count` times
   (r_things.c:817-834): `bestscale = MAXINT`; scan from `unsorted.next` forward; take `ds` when
   `ds->scale < bestscale` (strict, r_things.c:822); unlink `best` and append it at the TAIL of
   the sorted list (r_things.c:828-833).
4. Result: ascending scale (far to near). **Tie order: among equal `scale`, the earliest in the
   remaining unsorted list (= earliest projected, i.e. lowest vissprites index) is chosen first
   and appended first, so equal-scale sprites are drawn in projection order; a later-projected
   sprite overdraws an earlier one at equal scale.** Selection is O(n^2), stable.
5. `best` is uninitialised before the first pass (r_things.c:815 commented out); if every scale
   were >= MAXINT it would be used stale. Unreachable: vis->scale <= 80<<16 (1.5 step 4,
   r_things.c:552).

#### 2.1.8 R_DrawMasked (r_things.c:958-986), order

1. R_SortVisSprites (r_things.c:963).
2. If any vissprites, draw each with R_DrawSprite from `vsprsortedhead.next` to the tail
   (back to front) (r_things.c:965-975).
3. Then every drawseg from `ds_p-1` down to `drawsegs` with `maskedtexturecol` non-NULL:
   `R_RenderMaskedSegRange(ds, ds->x1, ds->x2)` (r_things.c:978-980). R_RenderMaskedSegRange
   marks drawn columns `MAXSHORT` and skips marked ones (r_segs.c:162-185, §1.3), so
   columns already drawn during sprite clipping are not redrawn.
4. `if (!viewangleoffset) R_DrawPlayerSprites()` (r_things.c:984-985): always taken (1.3).

#### 2.1.9 R_DrawSprite (r_things.c:842-950)

1. Local `short clipbot[SCREENWIDTH], cliptop[SCREENWIDTH]`; for `x = spr->x1..spr->x2` set both
   to -2 (r_things.c:845-855).
2. Scan `ds = ds_p-1` down to `drawsegs` (r_things.c:860). Skip if `ds->x1 > spr->x2 ||
   ds->x2 < spr->x1 || (!ds->silhouette && !ds->maskedtexturecol)` (r_things.c:863-870).
3. `r1 = max(ds->x1, spr->x1)`, `r2 = min(ds->x2, spr->x2)` (r_things.c:872-873).
   `lowscale = min(scale1, scale2)`, `scale = max(scale1, scale2)` (if `scale1 > scale2` then
   low = scale2, else low = scale1; r_things.c:875-884).
4. Seg behind sprite: `if (scale < spr->scale || (lowscale < spr->scale &&
   !R_PointOnSegSide(spr->gx, spr->gy, ds->curline)))` then if `ds->maskedtexturecol`,
   `R_RenderMaskedSegRange(ds, r1, r2)`; `continue` (r_things.c:886-895). Masked mids behind a
   sprite are drawn now, before the sprite, limited to r1..r2.
5. Otherwise clip: `silhouette = ds->silhouette`; clear SIL_BOTTOM if `spr->gz >=
   ds->bsilheight`; clear SIL_TOP if `spr->gzt <= ds->tsilheight` (r_things.c:899-905).
   silhouette 1: `clipbot[x] = ds->sprbottomclip[x]` where still -2; 2: `cliptop[x] =
   ds->sprtopclip[x]` where still -2; 3: both (r_things.c:907-931). First writer wins per
   column; scanning runs from the last-emitted drawseg backwards.
6. Unclipped columns: `clipbot = viewheight`, `cliptop = -1` (r_things.c:938-945).
7. `mfloorclip = clipbot; mceilingclip = cliptop; R_DrawVisSprite(spr, spr->x1, spr->x2)`
   (r_things.c:947-949).

#### 2.1.10 R_DrawVisSprite (r_things.c:396-443)

1. `patch = W_CacheLumpNum(vis->patch + firstspritelump, PU_CACHE)` (r_things.c:408).
2. `dc_colormap = vis->colormap` (r_things.c:410). If NULL: `colfunc = fuzzcolfunc`
   (r_things.c:412-416). Else if `mobjflags & MF_TRANSLATION`: `colfunc =
   R_DrawTranslatedColumn` (the function directly, not `transcolfunc`) and `dc_translation =
   translationtables - 256 + ((flags & MF_TRANSLATION) >> (MF_TRANSSHIFT-8))`, i.e. table
   `t-1` for translation bits t = 1..3 (r_things.c:417-422). Otherwise colfunc keeps its value
   (basecolfunc, restored at the end of every call, r_things.c:442).
3. `dc_iscale = abs(vis->xiscale) >> detailshift; dc_texturemid = vis->texturemid;
   frac = vis->startfrac; spryscale = vis->scale; sprtopscreen = centeryfrac -
   FixedMul(dc_texturemid, spryscale)` (r_things.c:424-428).
4. `for (dc_x = vis->x1; dc_x <= vis->x2; dc_x++, frac += vis->xiscale)`: `texturecolumn =
   frac >> FRACBITS`; RANGECHECK `texturecolumn < 0 || >= patch->width` -> I_Error
   "R_DrawSpriteRange: bad texturecolumn" (r_things.c:430-436); `column = patch +
   LONG(columnofs[texturecolumn])`; `R_DrawMaskedColumn(column)` (r_things.c:437-439).
   Note the loop variable is the global `dc_x`.
5. `colfunc = basecolfunc` (r_things.c:442).

#### 2.1.11 R_DrawMaskedColumn (r_things.c:345-388)

Globals `mfloorclip, mceilingclip, spryscale, sprtopscreen` (r_things.c:345-349).
1. `basetexturemid = dc_texturemid` (r_things.c:357).
2. While `column->topdelta != 0xff` (r_things.c:359):
   `topscreen = sprtopscreen + spryscale*column->topdelta; bottomscreen = topscreen +
   spryscale*column->length` (int multiplies, r_things.c:363-364);
   `dc_yl = (topscreen + FRACUNIT-1) >> FRACBITS; dc_yh = (bottomscreen-1) >> FRACBITS`
   (r_things.c:366-367);
   `if (dc_yh >= mfloorclip[dc_x]) dc_yh = mfloorclip[dc_x]-1;`
   `if (dc_yl <= mceilingclip[dc_x]) dc_yl = mceilingclip[dc_x]+1;` (r_things.c:369-372);
   if `dc_yl <= dc_yh`: `dc_source = (byte*)column + 3; dc_texturemid = basetexturemid -
   (topdelta << FRACBITS); colfunc()` (r_things.c:374-383);
   next post at `column + length + 4` (r_things.c:384).
3. `dc_texturemid = basetexturemid` (r_things.c:387).
No other clamp: dc_yl/dc_yh are bounded only by the clip arrays.

#### 2.1.12 R_DrawPlayerSprites / R_DrawPSprite (r_things.c:643-776)

R_DrawPlayerSprites:
1. `lightnum = (viewplayer->mo->subsector->sector->lightlevel >> LIGHTSEGSHIFT) + extralight`,
   clamped exactly as in R_AddSprites into `spritelights` (r_things.c:753-762).
2. `mfloorclip = screenheightarray; mceilingclip = negonearray` (r_things.c:765-766).
3. For `i = 0..NUMPSPRITES-1` (ps_weapon = 0 then ps_flash = 1, p_pspr.h:60-66): if
   `psp->state`, R_DrawPSprite (r_things.c:769-775). Flash is drawn after (over) the weapon.

R_DrawPSprite:
1. RANGECHECK sprite/frame like R_ProjectSprite (r_things.c:659-669). `lump = lump[0]`,
   `flip = flip[0]` (r_things.c:672-673): psprites never use rotations.
2. `tx = psp->sx - 160*FRACUNIT; tx -= spriteoffset[lump]; x1 = (centerxfrac + FixedMul(tx,
   pspritescale)) >> FRACBITS`; `if (x1 > viewwidth) return` (r_things.c:676-683);
   `tx += spritewidth[lump]; x2 = ((centerxfrac + FixedMul(tx, pspritescale)) >> FRACBITS) - 1`;
   `if (x2 < 0) return` (r_things.c:685-690).
3. Local `avis` (not a vissprite slot) (r_things.c:656, 693): `mobjflags = 0`;
   `texturemid = (BASEYCENTER<<FRACBITS) + FRACUNIT/2 - (psp->sy - spritetopoffset[lump])`;
   x1/x2 clamped to [0, viewwidth-1]; `scale = pspritescale << detailshift` (r_things.c:694-698).
4. Flip: `xiscale = -pspriteiscale, startfrac = spritewidth[lump]-1`; else `pspriteiscale`, 0;
   left-clip adjust as for things (r_things.c:700-712). `patch = lump` (r_things.c:714).
5. Colormap, first match wins (r_things.c:716-736):
   `viewplayer->powers[pw_invisibility] > 4*32 || powers[pw_invisibility] & 8` -> NULL (fuzz);
   else `fixedcolormap`; else `state->frame & FF_FULLBRIGHT` -> `colormaps`; else
   `spritelights[MAXLIGHTSCALE-1]` (brightest distance entry of the player's sector light).
   The psprite fuzz test reads the power counter, not `mo->flags & MF_SHADOW`.
6. `R_DrawVisSprite(vis, vis->x1, vis->x2)` (r_things.c:738).

Psprite y at viewheight 200 (centery 100, r_main.c:696): spryscale = FRACUNIT, so
`sprtopscreen = (100<<16) - texturemid = psp->sy - spritetopoffset - FRACUNIT/2`.


### 2.2 r_draw.c

#### 2.2.1 Buffer addressing: R_InitBuffer (r_draw.c:695-720)

`viewwindowx = (SCREENWIDTH - width) >> 1`; `columnofs[i] = viewwindowx + i` for `i < width`;
`viewwindowy = 0` if `width == SCREENWIDTH` else `(SCREENHEIGHT - SBARHEIGHT - height) >> 1`
(SBARHEIGHT 32, r_draw.c:51); `ylookup[i] = screens[0] + (i + viewwindowy)*SCREENWIDTH`.
Called as `R_InitBuffer(scaledviewwidth, viewheight)` (r_main.c:717). At screenblocks 11:
scaledviewwidth 320, viewheight 200 (r_main.c:682-686), so the view is the whole of screens[0].
At screenblocks 10: 320x168 at viewwindowy 0 (r_main.c:689-690, r_draw.c:712-713).

Detail selection (r_main.c:693-714): detailshift 0 -> `R_DrawColumn`, `R_DrawFuzzColumn`,
`R_DrawTranslatedColumn`, `R_DrawSpan`; detailshift 1 -> `R_DrawColumnLow`, `R_DrawFuzzColumn`,
`R_DrawTranslatedColumn`, `R_DrawSpanLow`. Low detail is reachable only from the config default
`detaillevel` (m_misc.c:280, r_main.c:783); the menu toggle returns before resizing
(m_menu.c:1131-1147).

#### 2.2.2 R_DrawColumn (r_draw.c:105-148)

`count = dc_yh - dc_yl`; return if < 0 (r_draw.c:112-116). RANGECHECK: `(unsigned)dc_x >=
SCREENWIDTH || dc_yl < 0 || dc_yh >= SCREENHEIGHT` -> I_Error (r_draw.c:118-123; checks
SCREENHEIGHT, not viewheight). `dest = ylookup[dc_yl] + columnofs[dc_x]`; `fracstep =
dc_iscale; frac = dc_texturemid + (dc_yl - centery)*fracstep` (r_draw.c:128-133). Do-while
`count+1` pixels: `*dest = dc_colormap[dc_source[(frac>>FRACBITS) & 127]]; dest +=
SCREENWIDTH; frac += fracstep` (r_draw.c:138-147).
The `&127` is the only vertical wrap for walls and masked columns: rows wrap with period 128
regardless of texture height; for heights < 128 indices `height..127` read past the column
data; for heights > 128 rows 128+ are never shown. The unrolled variant is `#if 0`
(r_draw.c:154-208).

#### 2.2.3 R_DrawColumnLow (r_draw.c:211-253)

Same count/RANGECHECK (r_draw.c:219-234, check done before doubling). Then `dc_x <<= 1`
mutates the global (r_draw.c:236); writes `dest` and `dest2` at columns `2x, 2x+1` with the same
`&127` lookup (r_draw.c:238-252).

#### 2.2.4 R_DrawFuzzColumn (r_draw.c:256-368)

- `FUZZTABLE 50`, `FUZZOFF (SCREENWIDTH)` (r_draw.c:259-260): offsets are one ROW up/down
  (the comments at r_draw.c:279-280, 355-356 saying left/right are wrong).
- `fuzzoffset[50]` (r_draw.c:263-272), sign sequence (+ = +320, - = -320):
  `+ - + - + + - + + - + + + - + + + - - - - + - - + + + + - + - + + - - + + - - - - + + + + - + + - +`
  (29 plus, 21 minus).
- `int fuzzpos = 0` (r_draw.c:274): static initialiser only; the only writes are the increment
  and wrap at r_draw.c:361-362 (grep: no other reference in the tree). Never reset per frame,
  level, demo or game; persists for the life of the process.
- Body: `if (!dc_yl) dc_yl = 1; if (dc_yh == viewheight-1) dc_yh = viewheight-2`
  (r_draw.c:293-298, exact-equality tests); `count = dc_yh - dc_yl`, return if < 0
  (r_draw.c:300-304) (returns do not advance fuzzpos); RANGECHECK (r_draw.c:307-314);
  `dest = ylookup[dc_yl] + columnofs[dc_x]` (r_draw.c:343, `dc_x` not doubled even in low
  detail); frac/fracstep computed but unused for sampling (r_draw.c:346-347, 366).
  Loop: `*dest = colormaps[6*256 + dest[fuzzoffset[fuzzpos]]]; if (++fuzzpos == FUZZTABLE)
  fuzzpos = 0; dest += SCREENWIDTH` (r_draw.c:352-367). Colormap 6 always, independent of
  light, fixedcolormap and dc_colormap.
- In-place semantics: pixels are processed top to bottom, so `dest[-320]` reads the value this
  same column wrote one row earlier (already fuzzed); `dest[+320]` reads the not-yet-processed
  pixel below. Reads see everything drawn earlier in the frame (walls, planes, earlier sprites)
  and, where nothing was drawn this frame, the previous contents of screens[0].

#### 2.2.5 R_DrawTranslatedColumn, translation tables (r_draw.c:382-483)

- Body (r_draw.c:385-447): same count, RANGECHECK and frac set-up; `*dest =
  dc_colormap[dc_translation[dc_source[frac>>FRACBITS]]]` - NO `&127` wrap (r_draw.c:442);
  not detail-aware.
- R_InitTranslationTables (r_draw.c:459-483): `Z_Malloc(256*3+255)` then aligned with an
  `(int)` pointer cast (r_draw.c:463-464); for `0x70 <= i <= 0x7f`: table0 `0x60 + (i&0xf)`,
  table1 `0x40 + (i&0xf)`, table2 `0x20 + (i&0xf)` (r_draw.c:469-475); identity otherwise
  (r_draw.c:479-480). Called in R_Init (r_main.c:790).
- Use: only via `MF_TRANSLATION` bits (r_things.c:417). The only writer of those bits is
  P_SpawnPlayer: `if (mthing->type > 1) mobj->flags |= (mthing->type-1) << MF_TRANSSHIFT`
  (p_mobj.c:668-669); info.c contains no MF_TRANSLATION (grep count 0). Player 1 (consoleplayer
  in single player) gets none; players 2-4 do (netgame, or multi-player demos). Psprites set
  `mobjflags = 0` (r_things.c:694) and never translate.

#### 2.2.6 R_DrawSpan / R_DrawSpanLow (r_draw.c:500-686)

Globals ds_y, ds_x1, ds_x2, ds_colormap, ds_xfrac, ds_yfrac, ds_xstep, ds_ystep, ds_source
(r_draw.c:500-512).
- R_DrawSpan (r_draw.c:520-563): RANGECHECK `ds_x2 < ds_x1 || ds_x1 < 0 || ds_x2 >= SCREENWIDTH
  || (unsigned)ds_y > SCREENHEIGHT` (note `>`: ds_y == 200 passes) (r_draw.c:528-538).
  `dest = ylookup[ds_y] + columnofs[ds_x1]`; `count = ds_x2 - ds_x1`; do-while count+1 pixels:
  `spot = ((yfrac >> (16-6)) & (63*64)) + ((xfrac >> 16) & 63)`;
  `*dest++ = ds_colormap[ds_source[spot]]`; `xfrac += ds_xstep; yfrac += ds_ystep`
  (r_draw.c:541-562). Flat is 64x64 row-major, row from yfrac, column from xfrac; shifts are
  arithmetic on signed fixed_t.
- R_DrawSpanLow (r_draw.c:643-686): RANGECHECK before doubling; `ds_x1 <<= 1; ds_x2 <<= 1`
  mutate the globals (r_draw.c:667-668); `count = ds_x2 - ds_x1` (doubled) and each of the
  `count+1` iterations writes 2 pixels (r_draw.c:673-685): it writes `4*(x2-x1)+2` pixels from
  column `2*x1`, about twice the span width, running past the span and potentially past the row
  end (unchecked).

#### 2.2.7 R_FillBackScreen / R_VideoErase / R_DrawViewBorder

- R_FillBackScreen (r_draw.c:731-811): returns if `scaledviewwidth == 320` (r_draw.c:747-748);
  otherwise writes only `screens[1]` (r_draw.c:756, 776-810). Never touches screens[0].
- R_VideoErase copies screens[1] to screens[0] (r_draw.c:817-828).
- R_DrawViewBorder (r_draw.c:843-875): returns if `scaledviewwidth == SCREENWIDTH`
  (r_draw.c:850-851); otherwise copies the top band through the first row's left margin, the
  last row's right margin through the bottom band, and the side margins (r_draw.c:853-871): only
  pixels outside the view rectangle. Called from D_Display after R_RenderPlayerView
  (d_main.c:285-295).

#### 2.2.8 RANGECHECK

Compiled in (doomdef.h:75). Fatal checks in my files: r_things.c:433-436, 506-516, 659-669;
r_draw.c:118-123, 225-232, 307-314, 396-405, 528-538, 651-661. v_video.c:221-232 only prints
and returns (not fatal).


### 2.3 r_data.c

#### 2.3.1 On-disk structs (32-bit layout)

- `mappatch_t { short originx, originy, patch, stepdir, colormap; }` = 10 bytes
  (r_data.c:69-76).
- `maptexture_t { char name[8]; boolean masked; short width; short height; void**
  columndirectory; short patchcount; mappatch_t patches[1]; }` (r_data.c:84-93). On ILP32:
  offsets name 0, masked 8 (4 bytes), width 12, height 14, columndirectory 16 (4 bytes),
  patchcount 20, patches 22. `masked`, `columndirectory`, `stepdir` and `colormap` are never
  read (only originx, originy, patch copied, r_data.c:539-541).
- In-memory `texpatch_t {int originx, originy, patch}`, `texture_t {name[8], width, height,
  patchcount, patches[]}` (r_data.c:99-125).

#### 2.3.2 R_InitTextures (r_data.c:411-574)

1. PNAMES: `nummappatches = LONG(*(int*)names)`, names are 8 bytes from offset 4; each resolved
   with `W_CheckNumForName` (last match wins, may be -1) (r_data.c:448-457).
2. TEXTURE1 always, TEXTURE2 if present (r_data.c:463-479): `int32 count`, then `int32
   offset[count]` relative to the lump start (r_data.c:464-466, 517, 522). Only check: `offset >
   maxoff` (lump length) -> I_Error (r_data.c:519-520); no bound on the struct body.
3. Per texture: copy width, height, patchcount, name (r_data.c:529-533); each patch: originx,
   originy, `patch = patchlookup[mpatch->patch]` (unchecked index), I_Error if -1
   (r_data.c:537-547).
4. `texturewidthmask = j-1` with `j` the largest power of two `<= width` (r_data.c:551-555);
   `textureheight = height << FRACBITS` (r_data.c:556).
5. `R_GenerateLookup(i)` for every texture (r_data.c:566-567); `texturetranslation[i] = i`,
   allocated `numtextures+1` (r_data.c:570-573).

#### 2.3.3 R_GenerateLookup (r_data.c:296-374)

1. `texturecomposite = 0`, `texturecompositesize = 0` (r_data.c:312-314). `patchcount` byte
   array (r_data.c:322-323).
2. For each patch in definition order, for `x` in `[max(originx,0), min(originx+patchwidth,
   width))`: `patchcount[x]++`, `collump[x] = patch lump`, `colofs[x] =
   LONG(columnofs[x-originx]) + 3` (r_data.c:326-346). Last covering patch wins collump/colofs;
   colofs points at the first post's pixel bytes (skips topdelta, length, pad).
3. For each `x` (r_data.c:349-373): if `!patchcount[x]`, `printf("R_GenerateLookup: column
   without a patch (%s)\n")` and **return** - the remaining columns are not processed at all
   (r_data.c:351-356). If `patchcount[x] > 1`: `collump[x] = -1`, `colofs[x] = current
   compositesize`, I_Error if `compositesize > 0x10000 - height` (r_data.c:365-369), then
   `compositesize += height` (r_data.c:371). `patchcount` is a byte: 256 covering patches would
   wrap to 0.

#### 2.3.4 R_GenerateComposite / R_DrawColumnInCache (r_data.c:184-289)

- R_GenerateComposite (r_data.c:228-289), lazily from R_GetColumn: `Z_Malloc(compositesize)`
  (not zeroed, r_data.c:244-246); patches in definition order, columns clipped to
  `[max(originx,0), min(originx+width, texture->width))`, only columns with `collump[x] < 0`
  (r_data.c:254-284); each through `R_DrawColumnInCache(patchcol, block + colofs[x],
  patch->originy, texture->height)`; later patches overwrite earlier. Rows covered by no post of
  any patch keep whatever the zone block contained. Block retagged PU_CACHE (r_data.c:288).
- R_DrawColumnInCache (r_data.c:184-218): per post: `source = post+3`, `count = length`,
  `position = originy + topdelta` (r_data.c:200-202); if `position < 0`: `count += position;
  position = 0` WITHOUT advancing `source` (r_data.c:204-208) - a post clipped at the top copies
  its first pixels, not its visible tail; clip `position + count > cacheheight`
  (r_data.c:210-211); `memcpy` if `count > 0` (r_data.c:213-214). The `dest` local is unused
  (r_data.c:196). Composite columns have no post headers.

#### 2.3.5 R_GetColumn (r_data.c:382-401)

`col &= texturewidthmask[tex]`; `lump = texturecolumnlump[tex][col]`; `ofs =
texturecolumnofs[tex][col]` (r_data.c:390-392). `if (lump > 0)` return `W_CacheLumpNum(lump) +
ofs` (raw patch bytes from the first post's data) (r_data.c:394-395); else generate the
composite if absent and return `texturecomposite[tex] + ofs` (r_data.c:397-400).

#### 2.3.6 R_InitFlats, R_InitSpriteLumps, R_InitColormaps, name lookups

- R_InitFlats: `firstflat = F_START+1`, `lastflat = F_END-1`, `numflats`; `flattranslation[i] =
  i`, allocated `numflats+1` (r_data.c:581-594).
- R_InitSpriteLumps (r_data.c:603-626): `firstspritelump = S_START+1`, `lastspritelump =
  S_END-1`; per lump `spritewidth = width<<16`, `spriteoffset = leftoffset<<16`,
  `spritetopoffset = topoffset<<16` (r_data.c:622-624).
- R_InitColormaps (r_data.c:633-644): COLORMAP read into a 256-aligned block (pointer `(int)`
  cast, r_data.c:642). Map 0 = full bright (used as `colormaps`, r_things.c:591, 730); map 6 =
  fuzz (r_draw.c:358); map 1 = infrared (p_user.c:377 via r_main.c:849-851); map 32 =
  `INVERSECOLORMAP` (p_user.c:41, 367). Distance tables `scalelight`/`zlight` index maps
  `0..NUMCOLORMAPS-1` (r_main.h:77-87; built by §1.6's r_main.c).
- R_InitData order: textures, flats, sprite lumps, colormaps (r_data.c:654-664).
- R_FlatNumForName: `W_CheckNumForName` (last match in the directory), I_Error if -1, returns
  `i - firstflat` without range check (r_data.c:672-686).
- R_CheckTextureNumForName: `name[0] == '-'` -> 0 (texture 0 doubles as "no texture");
  else first `strncasecmp(..., 8)` match in index order, else -1 (r_data.c:696-709).
  R_TextureNumForName: I_Error on -1 (r_data.c:718-730).

#### 2.3.7 R_PrecacheLevel (r_data.c:743-845)

Returns if `demoplayback` (r_data.c:758-759); otherwise only `W_CacheLumpNum(..., PU_CACHE)` and
memory counters. No pixel or game effect (zone residency only). Called from P_SetupLevel
(p_setup.c:689).

#### 2.3.8 texturetranslation / flattranslation (game writes, renderer reads)

- P_InitPicAnims builds `anims` (texture pairs via R_CheckTextureNumForName/R_TextureNumForName,
  flats via R_FlatNumForName; numpics = picnum - basepic + 1, I_Error if < 2)
  (p_spec.c:148-185).
- P_UpdateSpecials, every tic from P_Ticker (p_tick.c:153), before `leveltime++`
  (p_tick.c:157): for each anim, for `i = basepic .. basepic+numpics-1`:
  `pic = basepic + ((leveltime/speed + i) % numpics)`; `texturetranslation[i] = pic` or
  `flattranslation[i] = pic` (p_spec.c:1100-1110). Also scrolls side `textureoffset` for line
  special 48 (p_spec.c:1114-1124).
- Renderer reads: `texturetranslation[...]` at r_segs.c:120, 459, 573, 592;
  `flattranslation[pl->picnum]` at r_plane.c:424. No renderer file writes them after init
  (r_data.c:573, 593). They are never reset at level load, so they hold the last tic's values
  (of the previous level until the first P_UpdateSpecials of a new level).
- At render time after a tic, the arrays reflect `leveltime_now - 1` (computed before the
  increment).
- Game reads `textureheight` (p_floor.c:386-395), written only at init (r_data.c:556).


### 2.4 v_video.c and the target buffer

- `byte* screens[5]` (v_video.c:44). V_Init allocates 4 screens contiguously
  (v_video.c:482-493, called at d_main.c:1012); I_InitGraphics later re-points `screens[0]` to
  the X image data or a fresh `malloc(SCREENWIDTH*SCREENHEIGHT)` (i_video.c:910-913, called at
  d_main.c:367). ylookup is computed from screens[0] when R_ExecuteSetViewSize runs from
  D_Display (d_main.c:215-217, r_draw.c:719).
- R_RenderPlayerView writes only screens[0], through `ylookup[]/columnofs[]` (r_draw.c:128, 238,
  343, 428, 544, 670). It never clears the view: pixels not written this frame keep their prior
  contents.
- V_DrawPatch (v_video.c:203-262) is not called inside R_RenderPlayerView. Callers: r_draw.c
  (R_FillBackScreen, screens[1] only), am_map, d_main, f_finale, m_misc, st_lib, st_stuff,
  wi_stuff (grep). Inside D_Display, `HU_Drawer()` runs right after R_RenderPlayerView
  (d_main.c:267-271) and draws into screens[0] over the view; menus come later still.
  The pure R_RenderPlayerView framebuffer must be captured between d_main.c:268 and 271.
- `R_RenderPlayerView` is called only if `gamestate == GS_LEVEL && !automapactive && gametic`
  (d_main.c:267-268), with `&players[displayplayer]`.
- `gammatable[5][256]` and `usegamma` (v_video.c:51, 137) are applied only in I_SetPalette when
  building X colours (i_video.c:563-570). Palette-index output is unaffected.
- `devparm` tic dots write row 199 of screens[0] in I_FinishUpdate (i_video.c:361-374), after
  rendering. They affect later frames' undrawn pixels only.


### 2.5 View inputs from game state

#### 2.5.1 Tic/frame ordering

- G_Ticker: reborns, then the `gameaction` loop (ga_loadlevel -> G_DoLoadLevel), then
  `P_Ticker` for GS_LEVEL in the same call (g_game.c:612-622, 727-730).
- P_Ticker: return if `paused`; return if `!netgame && menuactive && !demoplayback &&
  players[consoleplayer].viewz != 1`; P_PlayerThink for each player in index order; then
  P_RunThinkers, P_UpdateSpecials, P_RespawnSpecials, `leveltime++` (p_tick.c:130-158).
- D_DoomLoop: TryRunTics (one or more G_Ticker, d_net.c:745), S_UpdateSounds, D_Display
  (d_main.c:389-395). One frame can follow several tics; the renderer sees the state after the
  last tic.
- P_SetupLevel sets `players[consoleplayer].viewz = 1` (p_setup.c:604); P_SpawnPlayer does not
  set viewz (p_mobj.c:657-686). The first P_PlayerThink of the level overwrites it.

#### 2.5.2 P_CalcHeight (p_user.c:77-141), in full

Called from P_PlayerThink after movement (p_user.c:272) and from P_DeathThink (p_user.c:198).
1. `player->bob = FixedMul(momx, momx) + FixedMul(momy, momy)` (p_user.c:88-90);
   `bob >>= 2` (p_user.c:92); `if (bob > MAXBOB) bob = MAXBOB` with `MAXBOB 0x100000` (16
   units) (p_user.c:49, 94-95). Momentum is `player->mo->momx/momy` at think time: after this
   tic's P_Thrust (p_user.c:160-164), before this tic's P_XYMovement/friction (P_RunThinkers
   runs later, p_tick.c:152).
2. `if ((player->cheats & CF_NOMOMENTUM) || !onground)` (p_user.c:97; CF_NOMOMENTUM = 4,
   d_player.h:75): `viewz = mo->z + VIEWHEIGHT`, clamp to `ceilingz - 4*FRACUNIT`, then
   unconditionally overwrite `viewz = mo->z + player->viewheight` and return
   (p_user.c:99-105). Net effect: `viewz = mo->z + viewheight`, no clamp, no bob, viewheight and
   deltaviewheight untouched.
3. Else `angle = (FINEANGLES/20*leveltime) & FINEMASK` (= `(409*leveltime) & 8191`, int
   arithmetic) and `bob = FixedMul(player->bob/2, finesine[angle])` (p_user.c:108-109).
4. If `playerstate == PST_LIVE` (p_user.c:113-136): `viewheight += deltaviewheight`; if
   `viewheight > VIEWHEIGHT` then `viewheight = VIEWHEIGHT, deltaviewheight = 0`; if `viewheight
   < VIEWHEIGHT/2` then `viewheight = VIEWHEIGHT/2` and `if (deltaviewheight <= 0)
   deltaviewheight = 1`; if `deltaviewheight` nonzero: `+= FRACUNIT/4`, and if that makes it 0,
   set 1. `VIEWHEIGHT = 41*FRACUNIT` (p_local.h:34).
5. `viewz = mo->z + viewheight + bob`; clamp `viewz > ceilingz - 4*FRACUNIT` to that
   (p_user.c:137-140).
- `onground` is a file-scope global (p_user.c:51), set in P_MovePlayer
  (`mo->z <= mo->floorz`, p_user.c:158) and P_DeathThink (p_user.c:197). When
  `mo->reactiontime` is nonzero P_MovePlayer is skipped (p_user.c:267-270) and P_CalcHeight
  reads the stale value left by the previous setter (previous tic, or the previous player in
  the P_Ticker loop).
- Other viewheight/viewz writers: P_ZMovement smooth step-up (`viewheight -= floorz - z;
  deltaviewheight = (VIEWHEIGHT - viewheight) >> 3`, p_mobj.c:252-257) and hard landing
  (`deltaviewheight = momz >> 3`, p_mobj.c:299-306), both during P_RunThinkers, after this
  tic's P_CalcHeight; P_DeathThink (p_user.c:190-196); P_SpawnPlayer `viewheight = VIEWHEIGHT`
  (p_mobj.c:683); teleport `viewz = z + viewheight` (p_telept.c:108), during the thinker pass.
- Consequence: the rendered `viewz` is computed from `mo->z`, `ceilingz` and momentum as they
  were at the start of the player's think, while `viewx/viewy/viewangle` come from the mobj after
  the whole tic (r_main.c:835-837). The renderer must read the stored `player->viewz`.
  Needed state if recomputed: momx, momy, z, ceilingz at think time, viewheight,
  deltaviewheight, playerstate, cheats, onground (global), leveltime. Reading the stored value
  needs only `player->viewz`.

#### 2.5.3 extralight, fixedcolormap, psprites

- `player->extralight`: A_Light0/1/2 set 0/1/2 (p_pspr.c:761-774); reset by P_SpawnPlayer
  (p_mobj.c:681) and G_PlayerFinishLevel (g_game.c:788). Renderer copies it in R_SetupFrame
  (r_main.c:838) and adds it to sector light for things (r_things.c:628), psprites
  (r_things.c:753-755) and walls/planes (§1).
- `player->fixedcolormap`: set at the end of P_PlayerThink after the power counters are
  decremented (p_user.c:339-353): invulnerability nonzero -> 32 if `> 4*32 || &8` else 0;
  else infrared nonzero -> 1 if `> 4*32 || &8` else 0; else 0 (p_user.c:363-383). Reset at
  p_mobj.c:682, g_game.c:789. P_DeathThink returns before this block (p_user.c:258-261), so a
  dead player keeps the last value.
- Invisibility power: decremented at p_user.c:345-347 (clears MF_SHADOW at 0); the psprite
  shadow test reads it directly (r_things.c:716-717).
- Psprites: `pspdef_t { state_t* state; int tics; fixed_t sx, sy; }` (p_pspr.h:68-75).
  P_SetPsprite: NULL state for S_NULL; sets state and tics; if `state->misc1`, `sx =
  misc1<<16, sy = misc2<<16`; runs the action; loops while tics == 0 (p_pspr.c:58-102). No
  vanilla state has nonzero misc1/misc2 (grep of info.c `{SPR_` rows not ending in `0,0}`:
  none), so that path is dead with the stock state table.
- P_MovePsprites (p_pspr.c:851-877): for each psprite with a state and `tics != -1`: `tics--`,
  at 0 `P_SetPsprite(nextstate)`; afterwards `flash.sx/sy = weapon.sx/sy` (p_pspr.c:875-876).
  Called from P_PlayerThink after P_CalcHeight (p_user.c:334) and from P_DeathThink BEFORE its
  P_CalcHeight (p_user.c:187, 198), so a dead player's weapon uses the previous tic's bob.
- A_WeaponReady bob (p_pspr.c:329-333): `angle = (128*leveltime) & FINEMASK; sx = FRACUNIT +
  FixedMul(player->bob, finecosine[angle]); angle &= FINEANGLES/2-1; sy = WEAPONTOP +
  FixedMul(player->bob, finesine[angle])`, `WEAPONTOP = 32*FRACUNIT` (p_pspr.c:48). Only runs
  when not switching or firing (returns at p_pspr.c:304-311, 315-325).
- Other sx/sy writers: P_BringUpWeapon `weapon.sy = WEAPONBOTTOM (128*FRACUNIT)`
  (p_pspr.c:47, 151); A_Lower `sy += 6*FRACUNIT` (p_pspr.c:389), `sy = WEAPONBOTTOM` when dead
  (p_pspr.c:396-398); A_Raise (p_pspr.c:420 ff.). Actions run outside P_MovePsprites (e.g.
  P_DropWeapon from damage during the thinker pass) can leave flash and weapon sx/sy unequal at
  render time; the renderer reads each psprite's own sx/sy (r_things.c:676, 695).
- P_CalcSwing (p_pspr.c:109-128) writes swingx/swingy; nothing in the renderer reads them.

Renderer reads of player state: `mo->x, y, angle, subsector->sector->lightlevel`,
`extralight`, `viewz`, `fixedcolormap`, `powers[pw_invisibility]`, `psprites[0..1].state
(->sprite, ->frame), sx, sy` (r_main.c:834-851, r_things.c:664-676, 695, 716-717, 727,
753-755, 769-775). Per thing: `x, y, z, angle, sprite, frame, flags, snext` and its sector's
`lightlevel, validcount, thinglist` (r_things.c:483-603, 622-639).


### 2.6 Patch/sprite lump format (from the C)

- `patch_t { short width; short height; short leftoffset; short topoffset; int columnofs[8]; }`
  (r_defs.h:356-364): 8-byte header, then `int32 columnofs[width]` from offset 8 (only `[width]`
  used, r_defs.h:362-363). Offsets are from the start of the lump (r_things.c:437-438,
  v_video.c:245, r_data.c:276-277). Values are little-endian through `SHORT`/`LONG`
  (r_things.c:434, 438; r_data.c:622-624).
- `post_t { byte topdelta; byte length; }`, `column_t = post_t` (r_defs.h:285-292). A column is
  a list of posts terminated by `topdelta == 0xff` (r_things.c:359, r_data.c:198,
  v_video.c:248). Pixel data starts at `post + 3` (one unused byte after length;
  r_things.c:376, r_data.c:200, v_video.c:250); the next post is at `post + length + 4` (one
  trailing unused byte; r_things.c:384, r_data.c:216, v_video.c:259-260).
- `topdelta` is an absolute row (no tall-patch relative encoding): `dc_texturemid =
  basetexturemid - (topdelta << FRACBITS)` (r_things.c:377), `position = originy + topdelta`
  (r_data.c:202).
- `leftoffset` (spriteoffset) shifts the sprite left of the thing origin (r_things.c:535,
  678); `topoffset` (spritetopoffset) is the height of the sprite top above `z` (r_things.c:556)
  or above the psprite y origin (r_things.c:695).
- Flats: 64x64 bytes row-major, `spot = row*64 + col` (r_draw.c:552).


### 2.7 Shared globals and renderer side effects on game state

| Item | Writer / reader | Can it change a game-state dump? |
| --- | --- | --- |
| `sector->validcount` | R_AddSprites writes `= validcount` (r_things.c:626); game reads it in P_RecursiveSound (p_enemy.c:115-122) but always after `validcount++` (p_enemy.c:164) | No outcome effect: every game traversal takes a fresh counter value above all render stamps. Field differs after a render; docs/STATE_FORMAT.md:426 lists validcount as not authoritative. |
| global `validcount` | R_SetupFrame `++` (r_main.c:862, §1.1); game increments before each use (p_maputl.c:774, p_map.c:151, 409, p_sight.c:332, p_enemy.c:164) | No (equality-only use; absolute value unused). |
| `fuzzpos` | renderer only (r_draw.c:274, 358-362) | No game effect; pixel output depends on all fuzz pixels drawn since process start. |
| `viewx/viewy` | game writes via R_PointToAngle2 (r_main.c:384-385); no p_/g_/s_ file reads them; R_SetupFrame overwrites (r_main.c:835-836) | No in either direction. |
| `texturetranslation/flattranslation` | game writes (p_spec.c:1106-1108); renderer reads only | Derived from leveltime; renderer does not write. |
| `textureheight` | init only (r_data.c:556); game reads (p_floor.c:386-395) | No. |
| `sprites` | read by f_finale.c:593 (cast) | No. |
| texture composites | lazy `R_GenerateComposite` from R_GetColumn (r_data.c:397-398); `W_CacheLumpNum(PU_CACHE)` everywhere | Zone residency only, no game field. |
| `extralight`, `fixedcolormap`, `spritelights`, `mfloorclip`, `dc_*`, `colfunc`, `vissprites` | renderer-local | No. |
| NetUpdate inside R_RenderPlayerView (r_main.c:881-897) | gathers input events / ticcmds | Not game state until a tic runs; a harness should stub or ignore it. |

Verdict: nothing in r_things.c, r_draw.c, r_data.c or v_video.c writes a mobj, player, sector
gameplay field, RNG or thinker. The only game-structure write is `sector->validcount`
(non-authoritative). Renderer-persistent state across frames: `fuzzpos` and the contents of
screens[0] outside the pixels drawn this frame.


### 2.8 Floating point

- No `float`/`double` token in r_things.c, r_draw.c, r_data.c, v_video.c, p_user.c, p_pspr.c
  (grep).
- Indirect: FixedDiv (m_fixed.c:57-65) guard `(abs(a)>>14) >= abs(b)` -> MININT/MAXINT by sign,
  else FixedDiv2: `c = ((double)a) / ((double)b) * FRACUNIT; if (c >= 2147483648.0 || c <
  -2147483648.0) I_Error; return (fixed_t)c` (m_fixed.c:69-87). Uses here: `xscale =
  FixedDiv(projection, tz)` (r_things.c:495), `iscale = FixedDiv(FRACUNIT, xscale)`
  (r_things.c:560).
- Rust reproduction: one IEEE binary64 division (correctly rounded), multiplication by 65536
  (exact power-of-two scale), truncation toward zero. The existing `Fixed::fixed_div` /
  `div2_checked` already do exactly this (crates/doom-types/src/fixed.rs:56-87). The repo's C
  harness compiles LP64 with `-O0 -fwrapv -ffp-contract=off` (tools/reference/m3h9c_build.py:53),
  i.e. SSE2 binary64 on x86-64. A 1997 i386 x87 build could differ by double rounding in rare
  cases (open question; prior milestones already chose binary64).


### 2.9 Limits and unchecked accesses

- MAXVISSPRITES 128: silent drop past the 128th projected sprite (1.6). Empty vissprites
  (`x1 == viewwidth`, 1.5) count toward the limit.
- MAXOPENINGS: not used by r_things.c / r_draw.c. Sprite clipping uses stack arrays
  `clipbot/cliptop[SCREENWIDTH]` (r_things.c:845-846) and drawseg `sprtopclip/sprbottomclip`
  pointers (into openings, built by r_segs.c, §1.3).
- MAXDRAWSEGS 256 (r_defs.h:55): the drawseg scan in R_DrawSprite is bounded by `ds_p`.
- Unchecked: R_DrawMaskedColumn has no screen clamp besides `mfloorclip/mceilingclip`
  (r_things.c:369-372); safe only while clip values lie in [-1, viewheight]. `mfloorclip[dc_x]`
  is indexed with the global `dc_x`, which R_DrawColumnLow doubles in place (r_draw.c:236): in
  low detail the next post of the same column and the R_DrawVisSprite / R_RenderMaskedSegRange
  `dc_x` loops (r_things.c:430, r_segs.c:162) continue from the doubled value (columns skipped,
  out-of-range clip reads and writes, possible RANGECHECK abort).
- R_DrawSpanLow writes ~2x the span length past `ds_x2` (2.6), unchecked.
- R_DrawFuzzColumn reads `dest +/- 320`; safe for `1 <= dc_yl`, `dc_yh <= viewheight-2`
  after its clamps.
- Column texel reads `dc_source[(frac>>16) & 127]` can index up to 127 bytes past the post start
  (r_draw.c:142), past the end of a single-patch texture column, its lump, or a composite block.
  `R_DrawTranslatedColumn` indexes `dc_source[frac>>16]` without any mask (r_draw.c:442).
- Sprite `texturecolumn` outside `[0, width)` -> fatal RANGECHECK (r_things.c:433-436).
- `startfrac += xiscale*(vis->x1 - x1)`, `spryscale*topdelta`, `409*leveltime`,
  `128*leveltime`, `FixedMul(momx,momx)+FixedMul(momy,momy)`, `tz<<2`: plain 32-bit int
  arithmetic with gcc wrap behaviour (C UB on overflow).


### 2.10 Hazards for a byte-exact port

1. **fuzzpos is process-global and never reset** (r_draw.c:274, 361-362). Shadow pixels
   depend on every fuzz pixel drawn since start-up, so C and Rust must render the same frame
   sequence from the same initial fuzzpos (0). A run that renders only some tics, or a C engine
   that runs several tics per D_Display, produces a different pattern. Fuzz is applied in place
   top to bottom (row above already fuzzed) and samples prior framebuffer contents, including
   undrawn (HOM) pixels from earlier frames.
2. Undrawn pixels keep previous screens[0] contents, including HU_Drawer text drawn after the
   previous frame's render (d_main.c:270-271), menu and wipe output. Equality needs a defined
   initial buffer and capture point (after R_RenderPlayerView, before HU_Drawer).
3. Sort tie order: equal-scale vissprites are drawn in projection order (r_things.c:822 `<`).
   Projection order depends on BSP subsector order, the per-sector validcount gate and thinglist
   head insertion (p_maputl.c:414-420), so Rust must preserve sector thinglist order exactly.
4. Silent MAXVISSPRITES drop, counted including empty `x1 == viewwidth` vissprites
   (r_things.c:330-331, 539).
5. `x1 > viewwidth` and `x2 < 0` rejects (r_things.c:539, 546; 682, 689) are off-by-one-shaped;
   reproduce them literally.
6. Raw-byte texel reads: `& 127` wrap for every column drawer except translated; single-patch
   wall columns point into the raw patch lump at the first post's data (r_data.c:345, 394-395),
   so gaps, `topdelta != 0` and heights < 128 show post headers and following bytes, and can read
   past the lump end. Composite blocks are uninitialised where no post covers a row
   (r_data.c:244, 213). These bytes are not defined by WAD data alone; a port must either prove
   a frame never reaches them or refuse that frame with a witness.
7. R_DrawColumnInCache top clip does not advance `source` (r_data.c:204-208): negative-originy
   patches in multi-patch columns paint shifted pixels. Reproduce.
8. R_GenerateLookup returns at the first patchless column (r_data.c:351-356): later columns
   keep last-patch collump/colofs even if multiply covered (never composited), and uncovered
   columns keep uninitialised zone values.
9. Fatal RANGECHECK paths (sprite texturecolumn, column y range, invalid sprite/frame) must map
   to refuse-the-tic witnesses, not clamps. In particular the flipped `startfrac = width*65536-1`
   stepping by `-iscale` can approach column -1 at the right edge; reproduce the exact frac
   sequence.
10. Low detail (detailshift 1) is broken in linuxdoom-1.10 (global `dc_x`/`ds_x1`/`ds_x2`
    doubling, R_DrawSpanLow overrun, fuzz/translated not doubled). Recommend M5a admit only
    detailshift 0 and refuse low detail explicitly.
11. viewz lags `mo->z` by the think/move ordering (5.2); use stored `player->viewz`. Bob,
    viewheight and A_WeaponReady use pre-increment leveltime. `onground` is a stale-able global.
12. texturetranslation/flattranslation correspond to `leveltime - 1` after a tic and are stale
    across a level change until the first P_UpdateSpecials (p_spec.c:1100-1110, p_tick.c:153-157).
13. `tz<<2` wraps for `tz >= 0x20000000` (r_things.c:502); use `wrapping_shl(2)`. `abs()` of
    INT_MIN stays INT_MIN.
14. Fuzz wins over fixedcolormap and fullbright for things (MF_SHADOW, r_things.c:578) and for
    psprites (invisibility counter, r_things.c:716); translation applies only when the colormap
    is non-NULL (r_things.c:412-422).
15. LP64 hosts: r_draw.c:464 (`(int)translationtables`) and r_data.c:642 / r_data.c:84-93 need
    declared adapters; the existing harness adapts r_data.c only
    (tools/reference/m3h9c_build.py:82-97), so building r_draw.c for C traces needs a new
    declared adapter for line 464.

## 3. The genuine-engine harness, the synthetic graphics set and the rooms

This section reports a scratch prototype of the genuine-engine harness that was built and run during Phase 1 to
size the design. The prototype lives in `~/scratch/m5a/proto` (ext4) and is **not committed**; no repository
file was edited by it. The prototype used a scratch copy of the registered IWAD (`~/scratch/m5a/iwad`); only counts
and pass/fail facts from those runs are recorded here. Source authority is `linuxdoom-1.10` at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`; every `file:line` below is in `linuxdoom-1.10/` unless it names a
`tools/` or `tests/` path. Observed on 2026-11-15 (America/Los_Angeles).

### 3.0 What the prototype showed (observed)

A forked runner (scratch only: `proto/build.py`, `proto/m5a_driver.c`, `proto/m5a_stubs.c`,
`proto/gfx.py`) compiles the 34 genuine files of the M4b runner plus the genuine renderer whole
(`r_main.c r_bsp.c r_segs.c r_plane.c r_things.c r_draw.c v_video.c`), calls the vanilla init
sequence (`V_Init`, `R_Init`, `P_InitSwitchList`, `P_InitPicAnims`, `R_InitSprites`) and
`R_RenderPlayerView (&players[displayplayer])` after every dumped tic. gcc 15.2.0, x86-64.

Synthetic (the 22 scripts of both M4b manifests, each WAD plus a prototype graphics PWAD):

- 66 strict runs (22 scripts x 3 view configurations: screenblocks 11; screenblocks 9;
  screenblocks 10 low detail), rendering every tic: every run exits 0 and its game-dump set
  has exactly the committed `dumps_sha256` of `tests/reference/m4b_synthetic_results.json` /
  `m4b_synthetic_results2.json`. 5,235 frames per configuration (the tics whose `G_Ticker`
  completed; the marker tic has no frame).
- Sanitizer builds: with 14 adapted renderer lines for the sanitizer builds (§3.4) and
  one new LP64 ABI adapter all 132 runs (22 x 6 configurations: strict x3, address x2,
  overflow x1) exit 0, dumps match the committed digests, and for each configuration the
  frame streams of strict, address and overflow are byte-identical (SHA-256 of all frames).
  Without them the address build stops in the first frame of BINF1 (`r_main.c:485`) and the
  overflow build in TELE1, TSPLIT, BVB1, BOSS2, SAWBITE, BTUNNEL, TELEZ (§3.4).
- strict32 (`-m32`, genuine text, no adapter at all): for all 22 scripts at two
  configurations (88 runs with strict) the frame stream equals the strict LP64 stream and
  the dumps match.
- BINF1 visually checked (PNG through the synthetic palette): walls, flats, distance
  lighting, the pistol weapon sprite and a thing sprite are drawn.

Real maps (local only, a scratch copy of the registered IWAD; nothing of it is recorded
here): the E1M1..E1M9 scripts (60 runs, 157,256 frames, DEMO1 not run), screenblocks 10,
rendering every tic: every strict run exits 0; dumps differ from the frozen runner only in
`L.flags` bit 256 (2,377,888 rows); 37 of 60 frame streams are equal between strict and
address, and the address build stops in 3 runs on r_segs.c:603-604 (table in §3.7). Two genuine findings that change the design:

1. **Rendering does perturb the game dump, in exactly one field.** `R_StoreWallRange` sets
   `linedef->flags |= ML_MAPPED` (r_segs.c:398) on every wall it stores. The M4b dump prints
   `L.flags` for lines with a special (`tools/reference/m4a_driver.c` `linesel`, `dump`).
   On real maps the render-every dumps differ from the frozen runner's in that bit only
   (counts in §3.7); nothing else differs. On the synthetic WADs the special lines are
   trigger lines between identical sectors, which `R_AddLine` rejects before storing
   (r_bsp.c:336-344), so the bit never appeared there.
2. **The genuine renderer reads memory outside the source column**, so some C pixels are
   not defined by game state and WAD bytes. Observed: E1M2 M2CHAOS, frame of tic 34, one
   pixel whose value differs between two strict runs of the same binary (ASLR): a wall
   column from `R_RenderSegLoop` whose first row has source row -1, masked by `&127`
   (r_draw.c:142) to row 127 of a short texture column, beyond the patch lump. Several real
   scripts show strict and address frame streams that differ in such pixels (§3.7).

### 3.1 Current runner anatomy

**Builders.** `tools/reference/m3h9c_build.py` is the library; `m4a_build.py` and
`m4b_build.py` are thin forks through its `Runner` class (driver, extra `--wrap` symbols,
behaviour entries, schema, contract path, build directory). M4b swaps in one more sanitizer
adapter for the duration of a call (`m4b_build.m4b_adapters`) so the M3h9c and M4a contracts
stay byte-identical.

**Genuine files compiled whole (34, `m3h9c_build.WHOLE`).** `g_game.c`, the 19 `p_*.c`
gameplay files without `p_saveg.c`, `m_random.c m_fixed.c m_bbox.c m_argv.c tables.c info.c
d_items.c sounds.c dstrings.c doomstat.c doomdef.c w_wad.c z_zone.c r_sky.c r_data.c`.

**r_main.c extract.** Only a declared verbatim extract is compiled (`r_main_extract`): the
include block, the global block `viewangleoffset .. spanfunc`, and six bodies
(`R_PointOnSide`, `R_PointOnSegSide`, `R_PointToAngle`, `R_PointToAngle2`, `R_PointToDist`,
`R_PointInSubsector`) with per-span SHA-256 in the contract.

**R_InitData only in part.** The driver calls `R_InitTextures` and `R_InitFlats` (r_data.c:656,
658) and not `R_InitSpriteLumps` or `R_InitColormaps`; `P_InitSwitchList` and `P_InitPicAnims`
(p_setup.c:702-703) without `R_InitSprites` (p_setup.c:704).

**Stubs (`tools/reference/m3h9c_stubs.c`, 43 functions + 17 variables).** Each preceded by a
machine-read `STUB symbol [io|reads|writes|var] file:lines: reason` line; the build checks with
`nm` that the object defines exactly the declared symbols. Renderer-relevant ones that M5a must
drop because it compiles their genuine definitions: `R_ExecuteSetViewSize` (r_main.c:671),
`setsizeneeded` (r_main.c:652), `R_FillBackScreen` (r_draw.c:731), `R_InitSprites`
(r_things.c:299), `sprites`, `numsprites` (r_things.c:91-92), `screens` (v_video.c:44).

**Driver (`m3h9c_driver.c`, frozen; `m4a_driver.c`, used unchanged by M4b).** Mirrors
`-playdemo`: `Z_Init`, `W_InitMultipleFiles`, `gamemode = registered`, the partial R_InitData,
`P_InitSwitchList`, `P_InitPicAnims`, single-player `D_CheckNetGame` values, `singledemo`,
`G_DeferedPlayDemo`, `G_DoPlayDemo`, dump `load.txt`, then per tic
`G_Ticker; gametic++; Z_CheckHeap; dump tic_NNNNN.txt`, stopping on `I_Quit` (longjmp),
`ga_completed/ga_victory/ga_worlddone` (end=exit) or `PST_REBORN` (end=reborn). Linker
observers (`-Wl,--wrap`): `P_AddThinker`, `Z_Free`, `P_SetThingPosition`, `P_InitThinkers`,
`Z_Malloc` (ZeroFill: every block zero-filled after allocation), and in M4a
`P_RemoveThinker`. Row kinds and columns are the `COLUMNS` string, parsed into the contract.

**Variants (`m3h9c_build.VARIANTS`).** `strict` (`-std=gnu89 -O0 -fwrapv -fno-strict-aliasing
-ffp-contract=off -fexcess-precision=standard -fsigned-char`); `overflow` (`-fno-wrapv`,
`-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow`,
`-fno-sanitize-recover=all`); `address` (`-fsanitize=address,undefined`,
`-fno-sanitize=alignment`, run with `ASAN_OPTIONS=detect_leaks=0`: leak detection disabled,
not LeakSanitizer coverage); opt-in `strict32` (`-m32`, genuine text, no adapter; never in the
contract). Per-file flags: `-include stdint.h` for `r_data.c` and `z_zone.c`.

**Adapters.** Exact one-line text replacements, failing unless the original occurs exactly once
in its file and stands at the declared line (`apply_once`): 8 LP64 ABI adapters on every
contract build (`p_setup.c:536`, `r_data.c:90, 482-485, 642`, `z_zone.c:440`), 6 M3h9c
sanitizer adapters plus the M4b telefrag one (`p_inter.c:817`) on the two sanitizer builds.
`r_data.c:642` (`R_InitColormaps`, intptr_t) is already declared although that runner never
calls it.

**Contracts and hashing.** `--contract`/`--check` regenerate the host-independent contract
(sources' SHA-256, header SHA-256, adapted-source SHA-256 per build class, flags, adapters,
extract, stubs, driver behaviour, dump row kinds). Result files record dump-set SHA-256
(concatenated snapshots in name order), endings, event tics; `m4b_reference.py --check`
requires the three variants' dump sets to be byte-identical.

**Frozen, must not change.** `tools/reference/m3h9c_driver.c`, `m3h9c_stubs.c`,
`m3h9c_build.py`, `m4a_driver.c`, `m4a_build.py`, `m4b_build.py`, every `*_reference.py`,
`*_wad.py`, `*_scripts.py` used by earlier evidence, and everything under `tests/reference/`
(contracts, manifests, results, WADs, scripts, goldens, fixtures). M5a adds files only.

### 3.2 M5a runner design

**New files only.** `tools/reference/m5a_build.py` (imports `m3h9c_build` and `m4b_build` as
libraries, like `m4b_build.py` does), `tools/reference/m5a_driver.c` (fork of
`m4a_driver.c`), `tools/reference/m5a_stubs.c` (fork of `m3h9c_stubs.c`; the frozen file
defines renderer symbols that would now clash), contract
`tests/reference/m5a_runner_contract.json` (schema `DOOM-M5A-RUNNER-v1`). The existing
`Runner` class cannot express a different stub file, an extra source list, or the removal of
the r_main extract, so `m5a_build.py` reuses the library functions (`adapted_sources`,
`layout_probe`, `apply_once`, `stubs` parsing logic, `VARIANTS`, `OPTIONAL_VARIANTS`,
`probe_m32`) and owns its own `build()`/`contract()`; the prototype `proto/build.py` is this
shape and worked.

**Sources.** `WHOLE` (34) + `RENDER = r_main.c r_bsp.c r_segs.c r_plane.c r_things.c
r_draw.c v_video.c` whole; the r_main extract is dropped (it is a verbatim subset of r_main.c,
so the six bodies and the globals are the same text). `tables.c`, `r_sky.c`, `r_data.c` are
already whole. Not compiled: `st_*.c`, `hu_*.c`, `am_map.c`, `wi_stuff.c`, `f_*.c`, `m_menu.c`,
`d_net.c`, `i_*.c`. Per-file `-include stdint.h` also for `r_draw.c`.

**Adapters.** The 8 ABI adapters (now all reached: `R_InitColormaps` runs), plus one new LP64
ABI adapter: `r_draw.c:464` `R_InitTranslationTables` truncates a pointer to `int`
(`(int)translationtables + 255`) exactly like `r_data.c:642`; adapted to `intptr_t`. Plus the
renderer sanitizer adapters of §3.4. A line-keyed adapter form is needed: `r_draw.c:559`
and `r_draw.c:682` are the same text (`\txfrac += ds_xstep; `), so `apply_once`'s
"occurs exactly once" rule cannot take them; `m5a_build.py` adds `apply_at_line` (the original
must stand at the declared line; the contract records the file's occurrence count of the
text) and keeps `apply_once` for every inherited adapter.

**New stubs (m5a_stubs.c).** Everything of `m3h9c_stubs.c` except the 7 renderer symbols above,
plus: `NetUpdate` [io] d_net.c:368-470 (called 4 times from `R_RenderPlayerView`,
r_main.c:881-897; with no events and single-player playback the console player's ticcmd is
overwritten from the demo, g_game.c:664-667); `I_AllocLow` [io] i_system.c:147-154 (malloc +
memset 0, as vanilla; used by `V_Init`, v_video.c:489); `screenblocks` [var] (m_misc.c:279,
default 9) and `detailLevel` [var] (m_misc.c:280, default 0), read by `R_Init`
(r_main.c:783) and set from driver options. The prototype linked with exactly these. Nothing
needs `I_SetPalette`, `I_FinishUpdate` or `I_InitGraphics`: they are called from `D_Display`
and `D_DoomLoop`, which the runner does not run.

**Zone.** Renderer caching goes through `W_CacheLumpNum(…, PU_CACHE)` (r_data.c:R_GetColumn,
r_plane.c:423 flats as `PU_STATIC`, then `Z_ChangeTag (…, PU_CACHE)` at 451), and composites through
`R_GenerateComposite` (`PU_STATIC`, `&texturecomposite[texnum]`, then `PU_CACHE`). These are
purgeable, so the 6 MiB zone can only change addresses, not the game state: observed, the
real-map runs never failed a `Z_Malloc` and `Z_CheckHeap` passed every tic. The ZeroFill
wrapper also fills composite blocks, which only matters for undefined reads (§3.3).

**Init sequence** (the prototype): `V_Init` (d_main.c:1012) ; `Z_Init` ; `W_InitMultipleFiles` ;
`gamemode = registered` ; `R_Init` (d_main.c:1095: `R_InitData` = textures, flats, sprite lumps,
colormaps; `R_InitPointToAngle` and `R_InitTables` are `#if 0` bodies, r_main.c:426-443,
509-536; `R_SetViewSize (screenblocks, detailLevel)`; `R_InitPlanes`; `R_InitLightTables`;
`R_InitSkyMap`; `R_InitTranslationTables`) ; `P_InitSwitchList` ; `P_InitPicAnims` ;
`R_InitSprites (names)` (p_setup.c:704) where `names` is a driver copy of the 138 `sprnames`
pointers with an explicit NULL terminator (below) ; the existing D_CheckNetGame values and demo
start ; after the load, `R_ExecuteSetViewSize ()` (what the first `D_Display` does when
`setsizeneeded` is set, d_main.c:215-217). `R_Init` replaces the two separate calls; the
dumps proved this changes nothing (§3.0).

**sprnames is not NULL-terminated.** `char *sprnames[NUMSPRITES]` (info.c:40-55) has exactly
138 entries, but `R_InitSpriteDefs` counts names with `while (*check != NULL)`
(r_things.c:191), reading past the array. In the prototype's binaries the next object
after `sprnames` is `states[]` (`nm`: strict `sprnames` 0x4baa0, `states` 0x4bf00), and the
padding happens to be zero, so vanilla `P_Init` would find 138; under ASan the read would hit
a redzone. Recommendation: the driver does not call `P_Init` (it already calls its first two
statements separately) and passes `R_InitSprites` a 139-entry copy; declare it in
`driver_behaviour` as equivalent to vanilla when the word after `sprnames` is NULL.

**When to render.** Vanilla: `D_DoomLoop` runs `TryRunTics` (one or more `G_Ticker; gametic++`,
d_net.c:742-746), `S_UpdateSounds`, then `D_Display` (d_main.c:389-395), which renders
`R_RenderPlayerView (&players[displayplayer])` only `if (gamestate == GS_LEVEL &&
!automapactive && gametic)` (d_main.c:267-268), after `ST_Drawer` and before `HU_Drawer`
(d_main.c:246, 270-271). With one tic per frame (`-timedemo`/`singletics` pacing) the frame
after tic N shows exactly the state of dump `tic_N`. So the M5a driver renders after
`G_Ticker; gametic++; Z_CheckHeap; dump` of every dumped tic and before the stop checks:

- the load (tic -1) is never rendered (gametic 0);
- the end=exit tic and the end=reborn tic are rendered (vanilla still draws that frame;
  `gamestate` is still `GS_LEVEL`);
- the end=marker tic has no frame (`I_Quit` exits inside `G_Ticker`);
- the frame is `screens[0]` immediately after `R_RenderPlayerView`, before any HUD text, status
  bar, border, menu or palette: the status bar area (screenblocks <= 10) and the border
  (screenblocks <= 9) keep whatever `I_AllocLow` gave them (zeros), and so must the Rust side.
- `screens[0]` is never cleared between frames (vanilla does not): a view-window pixel no
  drawer writes (hall-of-mirrors) keeps the previous frame's value; the Rust renderer must keep
  a persistent framebuffer.
- Rendering after the dump means a hypothetical perturbation would first show in dump N+1; the
  never/every comparison below catches either order.

**Options.** `--render none|every|TICLIST` (default none; `none` must be byte-identical to the
M4b runner's dumps), `--screenblocks 3..11`, `--detail 0|1`, `--frames-raw DIR` (raw frames for
a tic list or all), `--frame-digests FILE`. The prototype printed an FNV-1a 64 per frame; the
real design writes one line per frame: tic, SHA-256 of the 64,000 bytes (a small self-written
SHA-256 in the driver, or the driver streams raw frames to a pipe that the Python runner hashes:
the pipe costs no disk). Raw `frame_NNNNN.raw` (64,000 bytes, row-major 320x200 palette
indices) only on request.

**Game dumps stay byte-identical.** The dump code is the M4a text unchanged, with one declared
exception forced by r_segs.c:398: in the M5a driver's dump the `L.flags` column is printed with
`ML_MAPPED` (256, doomdata.h:135) cleared, and the per-frame mapped-line set is reported in the
frame stream instead (count and a digest of the sorted line indices), because `ML_MAPPED` is
presentation state: its only reader is the automap (am_map.c:1128), never `p_*.c` or
`g_game.c` (grep: no other use). The Rust renderer then keeps its own mapped set outside the
simulation (AGENTS: presentation must not change simulation state). The proof obligations:

1. `--render none` of the M5a binary: every dump set equal to the committed M4b digests
   (synthetic) and to the frozen M4b runner's dumps (real maps, local). Observed: all equal.
2. `--render every`, unmasked diagnostic mode: dumps differ from `--render none` only in
   `L.flags` bit 256. Observed: true for every synthetic and real run (counts §3.7).
3. `--render every`, masked (the contract mode): byte-identical to `--render none`.
4. Three variants (and strict32 locally) identical in dumps and in frame digests, except pixels
   classified undefined (§3.3).

### 3.3 Renderer perturbation analysis

**validcount.** The renderer increments it once per frame (`R_SetupFrame`, r_main.c:862) and
marks sectors with it (`R_AddSprites`, r_things.c:622-626). Gameplay compares marks only after
its own increment: `P_CheckPosition` (p_map.c:409) before `P_BlockLinesIterator`
(p_map.c:438, p_maputl.c:497-500), `P_TeleportMove` (p_map.c:151), `P_PathTraverse`
(p_maputl.c:774, then 850), `P_CheckSight` (p_sight.c:332, then 171-174), `P_NoiseAlert`
(p_enemy.c:164, then `P_RecursiveSound` 115-121). A renderer mark V is always older than the
gameplay value (strictly increasing, no wrap within any run), so no comparison changes; the
absolute value differs (prototype log: BINF1 ends at validcount 592 with rendering) but is not
dumped. Observed equal dumps confirm it.

**ML_MAPPED.** The one real perturbation (§3.0, finding 2): a gameplay structure written by the
renderer; outcome-neutral, but visible in the frozen dump format.

**fuzzpos** (r_draw.c:274, advanced at 361-362) is renderer state that persists across frames
and is never reset; frame N's fuzz depends on every fuzz column drawn before it. The Rust
renderer must carry it, and both sides must render the same frame set (every tic, or the same
list). The prototype printed it per frame; the frame stream should include it.

**Other persistent renderer state.** `framecount`, `sscount` (counters), `cachedheight[]`
(cleared every frame, r_plane.c:201), the composite cache (content-determined), `screens[0]`
(persistent, hall-of-mirrors), the visplane pads `pad1/pad2` (written 0xff via `top[-1]`,
`top[320]`, r_plane.c:438-439; `pad3/pad4` stay 0 from static storage and are read as
`bottom[-1]`, `bottom[320]`, r_plane.c:446-448). None is read by gameplay.

**RNG.** No renderer file calls `P_Random` or `M_Random` (grep r_*.c, v_video.c). `prndindex`
and `rndindex` in every dump are unchanged (observed).

**Zone.** Rendering allocates purgeable blocks, so later gameplay blocks land at other
addresses. No dump column holds an address (thinker indices only). Observed equal.

**Undefined reads (pixels, not dumps).** These make individual C pixels depend on memory
layout, so the frame is not a function of game state + WAD:

- Wall columns: `R_DrawColumn` indexes `dc_source[(frac>>FRACBITS)&127]` (r_draw.c:142). A
  texture shorter than 128 rows, or a first row whose source row rounds to -1 (observed), reads
  up to 127 bytes past the column's post data: inside the patch lump (defined by WAD bytes,
  Rust can mirror by reading the lump) or past its end into the next zone block or its
  `memblock_t` header (pointers: varies with ASLR; observed on E1M2).
- Composite columns (multi-patch textures, `texturecomposite`): past the block end into the
  next block's header; gaps never drawn by `R_DrawColumnInCache` are ZeroFill zeros in the
  runner but garbage in vanilla.
- `R_GenerateLookup` returning early on a column without a patch (r_data.c:351-355) leaves the
  rest of that texture's lookup as ZeroFill zeros (vanilla: garbage).
- Masked columns built from a composite (multi-patch masked midtextures, "Medusa"):
  `R_RenderMaskedSegRange` treats composite bytes as posts.
- `finesine[negative]` and `finetangent[>=4096]` (§3.4): defined only by the layout
  `finetangent` immediately followed by `finesine` (tables.c:67, 584). The prototype's strict,
  overflow and strict32 binaries have that layout (`nm`: strict 0x38940 + 0x4000 = 0x3c940;
  strict32 0x33920 + 0x4000 = 0x37920); the address build does not (32-byte redzone), so its
  adapters read the intended element explicitly. The driver should check the layout at start
  in strict/overflow/strict32 and refuse to run otherwise.

Plan: a read-provenance observer. `--wrap=R_DrawColumn,R_DrawColumnLow` (they are referenced
cross-object from r_main.c:704, 711 through `colfunc`/`basecolfunc`, so the linker redirects
them) records, before calling the genuine drawer, every source index the loop will read and
whether `dc_source + index` lies inside the cached lump or composite block that `dc_source`
points into (a `--wrap=W_CacheLumpNum` table and the `texturecomposite` sizes give the
bounds). A frame with an out-of-block read is reported with its pixel list; the comparison
treats those pixels as C-undefined, and the Rust renderer must refuse that frame (a
refuse-the-frame witness with complete renderer state), or the harness masks those pixels only
by an explicit owner decision. In-lump overreads are defined and must match. The synthetic
graphics avoid all of these by construction except where a room tests them on purpose.

### 3.4 Overflow and UB in the renderer under the sanitizer variants

Observed (prototype, synthetic scripts; the reports are from gcc 15.2.0 UBSan/ASan, each fixed
by the listed adapter and re-run until all 22 scripts passed in both sanitizer builds with
frame streams identical to strict):

| # | Site | Report | Adapter (sanitizer builds; strict/strict32 keep the text) |
| --- | --- | --- | --- |
| 1 | r_main.c:485 `sineb = finesine[angleb>>ANGLETOFINESHIFT]` (`R_ScaleFromGlobalAngle`) | index -218 out of bounds for `fixed_t [10240]` (BINF1, frame 1): `int angleb` negative after the unsigned sum | negative index reads `finetangent[FINEANGLES/2 + i]` (the contiguous-layout value; i is in [-4096,-1] so it always lands in `finetangent`) |
| 2 | r_main.c:484 `sinea = …` | same form, not observed | same, declared for robustness |
| 3 | r_segs.c:266 `finetangent[angle]` (`R_RenderSegLoop`) | index 8184 out of bounds for `fixed_t [4096]` (`angle` is 0..8191) | index >= 4096 reads `finesine[angle-4096]` |
| 4 | r_segs.c:266 `rw_offset-FixedMul(…)` | signed overflow `-70518787 - 2082184609` (TSPLIT, BVB1, BARON1) | unsigned subtraction, converted back (combined with #3 on the same line) |
| 5-10 | r_plane.c:438, 439, 445-448 `pl->top[maxx+1]`, `top[minx-1]`, `top[x-1]`, `bottom[x-1]`, `top[x]`, `bottom[x]` | index 320 out of bounds for `byte [320]` (`-fsanitize=bounds`); the pads absorb it | subscript through `(byte*)pl->top` / `(byte*)pl->bottom` (same bytes; bounds instrumentation sees a pointer) |
| 11-14 | r_draw.c:559-560 `R_DrawSpan`, 682-683 `R_DrawSpanLow` `xfrac += ds_xstep; yfrac += ds_ystep` | signed overflow `-2145453427 + -11386560` (TELE1, SAWBITE, TELEZ, BOSS2, BTUNNEL) | unsigned sums; line-keyed (559 and 682 are the same text) |
| 15 | r_segs.c:402 `offsetangle = abs(rw_normalangle-rw_angle1)` | negation of -2147483648 (BTUNNEL) | `d < 0 ? 0u - (unsigned)d : (unsigned)d` (gives 0x80000000 for INT_MIN, as `-fwrapv` abs does) |

Plus the ABI adapter `r_draw.c:464`. So 14 replaced lines in the prototype (#3 and #4 are one
line). Observed on real maps only (§3.7), not yet adapted: r_segs.c:603
`rw_toptexturemid += sidedef->rowoffset` and r_segs.c:604 `rw_bottomtexturemid += …`
(signed overflow in E1M8 and E1M9 scripts): 16 sanitizer adapter lines at least.

Not observed but likely on other content (from reading; to be confirmed by the synthetic limit
rooms and the real maps in the sanitizer builds): `r_plane.c:157-158` `ds_xfrac = viewx +
FixedMul(…)` (large plane distances: `FixedMul (planeheight, yslope[y])` near the horizon
exceeds 32 bits, then the sum overflows); `r_draw.c:133, 145` `frac` in `R_DrawColumn`
(far walls: `dc_iscale` up to 2^24 times 100 rows); `r_things.c` `R_DrawMaskedColumn`
`topscreen + spryscale*length` (very close sprites); `r_segs.c:361-362` `topfrac += topstep`
(tall sectors); `m_fixed.c` `FixedDiv` `abs(MININT)`; `r_main.c:453-500` low-detail
`<<detailshift` of a large `num` (`-fsanitize=shift`). The real-map runs (§3.7) were made
with the prototype adapters; any report there is listed in the table.

ASan cannot see overreads inside the zone (one malloc block, i_system stub), which is why the
read-provenance observer of §3.3 is needed rather than relying on ASan.

### 3.5 Synthetic graphics set (no id art)

**Tool and layout.** `tools/reference/m5a_gfx.py` writes `tests/reference/m5a_gfx.wad`, a
**PWAD loaded after the map WAD and before the demo** (`--wad MAP.wad --wad m5a_gfx.wad --wad
DEMO.lmp`); an independent verifier `tools/reference/m5a_check_gfx.py` re-parses it without
importing the generator. Lookups scan backwards (w_wad.c:351-392), and `R_InitFlats`,
`R_InitSpriteLumps`, `R_InitTextures` use the last `F_START/F_END`, `S_START/S_END`, `PNAMES`,
`TEXTURE1` (r_data.c:448, 463, 585-586, 608-609), so the PWAD fully replaces the map WAD's
graphics without touching the frozen WADs. Prototype: 733 lumps, 452,376 bytes.

Hard rule (gameplay identity): the PWAD's `TEXTURE1` starts with `m3h9c_wad.TEXTURES` in the
same order with the same width 64 and height 128 (texture numbers are dumped in `L.s0top…`,
switch indices come from them, and `textureheight` is gameplay-visible through
`P_FindShortestTextureAround`), and its flats start with `m3h9c_wad.FLATS` in the same order
(`S.floorpic` is `lump - firstflat`). New textures and flats are appended after them, and no
`animdefs` start name (p_spec.c:98-118) may be added unless the room wants the animation
(an added one activates `P_InitPicAnims`, p_spec.c:160-172). The prototype did this and every
dump matched.

**Lumps and the errors that require them.**

- `PLAYPAL`: 14 x 768. Not read by the renderer (only `I_SetPalette`); used for PNG export.
  Construction: 16 hue ramps x 16 levels (index = hue*16 + level, level 0 brightest);
  palettes 1-13 blend toward red/yellow/green by our own rule.
- `COLORMAP`: 34 x 256, read by `R_InitColormaps` (`W_GetNumForName`, I_Error
  "W_GetNumForName: COLORMAP not found!", w_wad.c:406). Construction (our own, exact integers):
  map m in 0..31: `hue*16 + min(15, level + m/2)`; map 32 (`INVERSECOLORMAP`, p_user.c:41, 367)
  `15 - level` on the gray ramp; map 33 constant. Fuzz uses map 6 (r_draw.c:358), light
  amplification map 1 (p_user.c:377), sky map 0 (r_plane.c:404).
- `PNAMES` + patches + `TEXTURE1` (no `TEXTURE2`; r_data.c:468 makes it optional): a texture
  naming a missing patch I_Errors "R_InitTextures: Missing patch in texture %s"
  (r_data.c:544); a bad offset "bad texture directory" (r_data.c:520); composites over 64 KB
  "texture %i is >64k" (r_data.c:367). Every switch texture of `alphSwitchList` with episode
  <= 2 must exist (p_switch.c:135-137 I_Error). `SKY1` (g_game.c:464 for episode 1) must be
  256x128 (or at least 128 tall) and single-patch per column: the sky is drawn with `&127`.
  Textures used on walls should be 128 tall (or a power of two >= 128) unless a room tests the
  short-texture read; every column must have a patch (r_data.c:351-355).
- `F_START`/`F_END` flats (4,096 bytes each) including `F_SKY1` (g_game.c:454
  `R_FlatNumForName` I_Error "R_FlatNumForName: %s not found", r_data.c:683) and
  `NUKAGE1..3` if any map uses special 7 or the anim range.
- `S_START`/`S_END` sprites. `R_InitSpriteDefs` iterates all 138 names; a name with **no**
  lump is tolerated (`numframes = 0`, r_things.c:241-245), but a name with some frames must
  have every frame up to its highest letter (r_things.c:255 "No patches found for %s frame
  %c") and, for a rotated frame, all 8 rotations (r_things.c:267 "missing rotations"); mixing
  rot 0 and rotations (125-129, 143) or duplicate rotations (151) also I_Error; frame letter
  >= 29 or rotation > 8 (115). At draw time a thing whose frame is beyond `numframes` I_Errors
  (`R_ProjectSprite`, r_things.c:508-514, after the behind-view and side culls 491-503) and so
  does a weapon sprite (`R_DrawPSprite`, r_things.c:660-668). So "only the sprites the rooms
  use" is legal, but "uses" means every state the actors and weapons can reach. The prototype
  generated, for every sprite referenced by `states[]` (info.c:135ff), every frame from A to
  the highest frame any state uses (FF_FULLBRIGHT masked off), as one 16x24 rot-0 patch with
  holes (so masked drawing and post splitting are exercised), and full 8-rotation sets with
  mirrored pairs (`POSSA2A8` etc.) for one actor: 733 lumps total, small. For the real tool:
  rot 0 for items and effects, 8 rotations with mirrored pairs for the player and every monster
  of the rooms (PLAY, POSS, SPOS, TROO, SARG — the Spectre uses SARG with `MF_SHADOW`, BOSS),
  weapon sprites PUNG, PISG, PISF, SHTG, SHTF, CHGG, CHGF, MISG, MISF, SAWG and fireballs/puffs
  (BAL1, BAL7, PUFF, BLUD, TFOG, IFOG, MISL), `FF_FULLBRIGHT` frames included. Each patch should
  encode (sprite, frame, rotation, column) visibly so a wrong pick shows in the image.
- Not needed because the runner never calls them: `ST_Init` (STBAR, STTNUM…), `HU_Init`
  (STCFN…), `R_FillBackScreen` (border flats and `brdr_*` patches, r_draw.c:731ff), `M_Init`,
  `WI_*`, `F_*`.

**Goldens and images.** Commit only the generator, the verifier, the WAD (synthetic, GPL), the
manifest and digest results; for review, `tools/reference/m5a_png.py` renders any raw frame
through palette 0 to PNG locally (stdlib zlib, as the prototype's `topng.py`), and a small
committed set of PNGs for the synthetic rooms only (a few key frames per room, through the
synthetic palette) is reasonable since everything in them is ours. Never PNGs of real maps.

### 3.6 Synthetic rooms (tool `tools/reference/m5a_wad.py`, extending the m4b_wad `Plan` pattern)

Each room is one `Plan` (grid painter, sectors, sides, things) in the E1M1 slot, with
`m5a_gfx.wad` loaded after it, and one or more demo scripts in the `m4b_scripts.py` format
(`idle`, `cmd`, turns), with a manifest listing per script the frames that must show the
feature (frame tic and a pixel predicate computed by an independent checker, e.g. "column 160
rows 40-60 are texture X row r" from the geometry, not from Rust).

| Room | Geometry and textures | Script |
| --- | --- | --- |
| PEG | upper/lower textures with `ML_DONTPEGTOP`/`ML_DONTPEGBOTTOM` on and off, x/y offsets, a door (DR) so the upper texture moves | stand, turn slowly across the four walls; open the door and watch |
| MASK | two-sided lines with masked midtextures (single-patch with holes), one taller than the opening, one with row offset; a multi-patch masked texture only in a separate "Medusa" room flagged as C-undefined | walk past at grazing angles |
| PLANES | many small sectors with different heights/flats/lights so visplanes split and merge (`R_FindPlane`/`R_CheckPlane`), a sector with equal planes to test merging | sweep turns |
| SKY | F_SKY1 ceilings in two adjacent sectors with different heights (sky upper-texture hack, r_segs.c), a sky floor | look around 360 degrees |
| LIGHT | sectors at light 0..255 in steps of 16, walls of every orientation (fake contrast: a seg with equal y (east-west) is one light level darker, equal x (north-south) one brighter, r_segs.c:124-127 masked, 646-649 walls) | stand in the middle, turn |
| DIST | a long corridor (distance light: `scalelight`, `zlight`, `MAXLIGHTZ` clamp) | walk forward slowly |
| SPRITES | monsters around the player in a ring at the 8 rotation angles, items, a fullbright frame, a corpse | turn in place; one monster walks around |
| CLIP | sprites behind two-sided lines, partly behind masked midtextures, sprites crossing a floor edge (drawseg clipping, `R_DrawSprite` silhouette logic) | strafe |
| WEAPON | player picks up each weapon, fires (flash frames, `extralight`), walks (bob, `psp->sx/sy`) | fire, switch, walk |
| FUZZ | a Spectre (SARG + MF_SHADOW) in front of varied walls, at the top and bottom view rows (fuzz clamps r_draw.c:293-298), and the player invisibility power (psprite fuzz, r_things.c:716-717) | approach the Spectre; pick up a blur sphere |
| LIGHTAMP | light amplification visor (`fixedcolormap` 1, p_user.c:377) and invulnerability (32, p_user.c:367) with their flicker at the end | pick up, wait out the power |
| LIMITS | one room per vanilla limit, each aimed just at and just past the bound: visplanes (128, I_Error r_plane.c:246), drawsegs (256, overflow check r_plane.c:376), vissprites (128, silent `overflowsprite`, r_things.c:326-331), openings (`MAXOPENINGS`, r_plane.c:384), solidsegs (`MAXSEGS` 32, r_bsp.c:88-92, no bound check in `R_ClipSolidWallSegment`: a likely ASan finding) | the frame past the bound must be a refused frame in Rust and an I_Error/abort or report in C |
| SHORT | a 64-tall and a 72-tall wall texture (the `&127` read and the -1 first row), as a deliberate C-undefined test of the provenance observer | stand still |
| HOM | a missing upper texture (hall-of-mirrors: previous frame persists) | turn so the hole moves |
| VIEW | the same room at screenblocks 3..11 and detail 0/1 | idle |

Scripts drive the player with forward/side/turn commands; turning in place is the cheapest
way to sweep a feature through every column. Screenblocks/detail come from the manifest per
script (driver options), not from the demo.

### 3.7 Real maps

The committed real-map scripts (`tests/reference/e1m1_scripts` .. `e1m9_scripts`, 60 runs) plus
DEMO1 (extracted from the IWAD copy and patched as M4b recorded) are the 61 runs of
`tests/reference/m4b_real_results.json`: 158,966 dumped tics in total (E1M1 10,029; E1M2 9,094;
E1M3 13,750; E1M4 9,386; E1M5 38,493 incl. DEMO1; E1M6 12,742; E1M7 12,939; E1M8 30,840;
E1M9 21,693), so 158,966 frames per view configuration (exit and reborn tics have frames,
marker tics do not, which equals the dumped tic count). Raw: 64,000 bytes per frame =
10.17 GB per configuration, so raw frames stay in scratch (and only on request); commit per run
a SHA-256 over the concatenated frame digests plus the frame count, and keep per-frame
SHA-256 lists (32 bytes x 158,966 = 5.1 MB) in scratch or as a local evidence file. The
first-differing-frame search then needs only the per-frame lists.

Observed (prototype, strict and address builds with the prototype adapters, screenblocks 10,
detail 0, rendering every dumped tic; "ML_MAPPED rows" = dump rows of the render-every run
that differ from the frozen M4b runner's, every one of them in `L.flags` bit 256 only; "other
rows" = any other difference; never-render runs of the M5a binary equalled the frozen runner
byte for byte in all 60 runs):

| Map | Runs | Frames (strict) | Other rows | ML_MAPPED rows | strict = address frame stream | address stopped |
| --- | --- | --- | --- | --- | --- | --- |
| E1M1 | 5 | 10,029 | 0 | 61,869 | 5 of 5 | 0 |
| E1M2 | 5 | 9,094 | 0 | 43,783 | 3 of 5 | 0 |
| E1M3 | 5 | 13,750 | 0 | 129,252 | 3 of 5 | 0 |
| E1M4 | 5 | 9,386 | 0 | 89,040 | 4 of 5 | 0 |
| E1M5 | 10 | 36,783 | 0 | 960,803 | 7 of 10 | 0 |
| E1M6 | 5 | 12,742 | 0 | 153,299 | 4 of 5 | 0 |
| E1M7 | 5 | 12,939 | 0 | 286,017 | 3 of 5 | 0 |
| E1M8 | 10 | 30,840 | 0 | 251,484 | 4 of 10 | 2 |
| E1M9 | 10 | 21,693 | 0 | 402,341 | 4 of 10 | 1 |
| total | 60 | 157,256 | 0 | 2,377,888 | 37 of 60 | 3 |

Every strict run exited 0 and its frame count equals its `m4b_real_results.json` tic count
(157,256 = 158,966 minus DEMO1's 1,710, not run). The address build stopped in M8IDLE, M8IDLE3
(r_segs.c:603 `rw_toptexturemid += sidedef->rowoffset`, a sum that overflows 32 bits) and M9WALK
(r_segs.c:604 `rw_bottomtexturemid += …`, a sum that overflows 32 bits): two more sanitizer adapters
needed (unsigned sums). Frame-stream differences, counted per frame by digest for five scripts:
M2CHAOS 1 of 1,550 frames differs between two strict runs and 1 between strict and address;
M3WALK 0 and 1 of 5,793; M5WALK 1 and 1 of 7,238; M7FIGHT 1 and 1 of 1,221; E1M1 CHAOS 0 and 0
of 5,000. So the undefined reads of §3.3 touch a handful of frames per run, and the strict
build is not even reproducible from run to run on them (ASLR). Wall time, strict render-every:
0.5-10.9 s per run; the address build about 2.2x strict (not an idle machine; informational).

### 3.8 Risks

- **Undefined pixels on real maps.** Short wall textures and the first-row -1 index make C
  pixels depend on zone layout (observed). Without the provenance observer the strict, address
  and strict32 frame streams cannot be required equal on real maps, and Rust cannot be compared
  on those pixels. The frequency is unknown until the observer exists (observed: a single pixel
  in a single frame of M2CHAOS; several scripts differ between strict and address, §3.7).
- **Layout dependence of the table overreads** (`finesine[-i]`, `finetangent[4096+i]`): the
  strict frames rely on gcc placing `finetangent` directly before `finesine`; needs a startup
  check and Rust must model the combined table.
- **ML_MAPPED masking** changes what the M5a dump prints compared with the genuine state; it
  must be declared and proven bit-exact (only bit 256, only `L.flags`), with the mapped set
  carried in the frame stream.
- **Line-keyed adapters** are a new mechanism (duplicate source lines); review must check the
  declared line and count.
- **Zone purging under load** (real maps, 6 MiB) could free a cached patch while a drawer still
  points into it (vanilla behaviour); not observed in the prototype runs.
- **sprnames terminator**: the driver copy is a declared deviation from calling `P_Init`.
- More sanitizer findings are likely on real content and limit rooms (§3.4 candidates).

## 4. Rust design and state inventory

This section covers what the renderer reads, where the Rust simulation holds it, what must be derived, how the new
crate is laid out and what cannot be derived (the stop-and-ask items, whose decisions are in §5). The repository head
inspected was `979dde8`; nothing was built or run for it except `which`, `rustc --version` and `df`. Rust citations
are `crates/<crate>/src/<file>:<line>`; C citations are `linuxdoom-1.10/<file>:<line>`. C-side quirk detail is in §1
and §2; this section cites the C only where the Rust design depends on it.

### 4.0 Summary of classification

| Renderer input | Class | Source in Rust |
| --- | --- | --- |
| player mo x, y, angle | (a) | `Actor::position/angle` |
| player `viewz` | **(b-history)**: re-derive with a renderer-owned view shadow observing every tic; see §4.1.2 and stop-and-ask S1 | not stored |
| `extralight` | (a) | `PlayerWeaponState::extra_light` |
| `fixedcolormap` | (b-history, one tic): from previous tic's powers | `PlayerPowers` |
| powers (invisibility for psprite fuzz) | (a) | `Simulation::original_player_powers` |
| psprites (state, sx, sy) | (a) | `PlayerWeaponState::psprites` |
| mobj x, y, z, angle, sprite, frame, flags | (a) | `Actor`, `ActorLife` |
| sector thinglist order | (a), and genuinely compared against C | `Simulation::sector_actor_order` / `spatial_order` |
| sector floor/ceiling height | (a) | `Simulation::sectors` |
| sector lightlevel | (a) | `Simulation::sector_lights_and_specials` |
| sector floorpic | (a via Snapshot) | field `sector.{i}.floorpic` |
| sector ceilingpic | (b): static (no admitted writer) | map lumps |
| flattranslation / texturetranslation | (b): pure function of `tic` and the WAD | derive |
| side textureoffset (special 48) | (b): map offset + `tic` x FRACUNIT, wrapping | derive |
| side rowoffset | (b): static | map lumps |
| side top/mid/bottom texture, non-reusable switches | (b): runtime line special 0 on an S1/W-switch line | field `line.{i}.special` |
| side textures, reusable switches (SR/GR) | **(b) except re-press while the button runs: stop-and-ask S2** | fields `button.{k}.*` + events |
| line flags, segs, subsectors, nodes, vertices, sidedefs, sectors | (a) | `doom_map::Map` |
| leveltime | (b): `Simulation::tic()` | `crates/doom-sim/src/lib.rs:959` |
| sky texture | (b): SKY1 for episode 1 | constant |
| validcount | (b): renderer-local | none needed |
| fuzzpos (r_draw.c:274) | renderer-owned persistent state: stop-and-ask S3 (frame-sequence contract) | renderer |
| undefined texel reads (reads past the lump, uninitialised composite bytes, column-without-patch) | **(c): stop-and-ask S4** | renderer |


### 4.1 State inventory

#### 4.1.1 Public read API that exists today

All of these take `&self` on `Simulation`, so an immutable borrow cannot mutate the
simulation (`Simulation: Send + Sync` is asserted at `crates/doom-sim/src/lib.rs:663-667`).

| API | Location | Gives |
| --- | --- | --- |
| `Simulation::tic()` | `crates/doom-sim/src/lib.rs:959` | completed-tic count (source `leveltime` after `P_Ticker`) |
| `Simulation::sectors()` | `crates/doom-sim/src/lib.rs:865` | `SectorState { floor, ceiling }` (`crates/doom-sim/src/multisector.rs:16-21`) |
| `Simulation::sector_actor_order(sector)` | `crates/doom-sim/src/lib.rs:869-893` | the sector's thing list, head first |
| `Simulation::spatial_order()` | `crates/doom-sim/src/lib.rs:839` | the single newest-first link order |
| `Simulation::actor(id)` | `crates/doom-sim/src/lib.rs:971` | `Actor`; accessors at `crates/doom-sim/src/actor.rs:60-118` |
| `Simulation::events()` | `crates/doom-sim/src/lib.rs:983` | the last completed tic's events |
| `Simulation::sector_lights_and_specials()` | `crates/doom-sim/src/sector_thinkers.rs:460-465` | runtime `(lightlevel, special)` per sector |
| `Simulation::original_player(slot)` | `crates/doom-sim/src/original_player.rs:256` | `OriginalPlayer { mo, health, state, attacker, damage_count, frags }` (`:76-90`) |
| `Simulation::original_player_powers(slot)` | `crates/doom-sim/src/episode1.rs:254-262` | `PlayerPowers { powers[6], cards[6] }` (`crates/doom-sim/src/original_player.rs:108-114`) |
| `Simulation::original_player_weapons(slot)` | `crates/doom-sim/src/player_weapons.rs:321-331` | `PlayerWeaponState { …, bob, extra_light, psprites[2] }` (`:11-39`) |
| `Simulation::actor_registry()` | `crates/doom-sim/src/spawning.rs:62` | `ActorRegistry::state(id)` → `ActorStateDef { sprite, frame, … }` (`crates/doom-sim/src/lifecycle.rs:46-60, 355`) |
| `ActorRegistry::original_state(id)` | `crates/doom-sim/src/identity.rs:337` | validated `OriginalState`, `descriptor()` → sprite, frame (`:145-158, 287`) |
| `Simulation::snapshot()` | `crates/doom-sim/src/lib.rs:1445` | canonical fields by stable path (`StateField::path/value`, `crates/doom-sim/src/snapshot.rs:7-15, 60`) |

`ActorLife` (`crates/doom-sim/src/lifecycle.rs:1196-1227`) is public and carries `sprite: u16`
(the `spritenum_t` ordinal, the order of `SOURCE_SPRITES`, `crates/doom-sim/src/source_actors.rs:82`),
`frame: u32` including `FF_FULLBRIGHT` (0x8000), `flags: u32` (full source flags: `MF_SHADOW`
0x40000, `MF_NOSECTOR` 8, `MF_TRANSLATION` bits), `state`, `reaction_time`. `Actor::location()`
gives `Location { subsector, sector }` (`crates/doom-sim/src/multisector.rs:8-13`).

Not public (only in the canonical Snapshot): runtime `floorpic`
(`crates/doom-sim/src/episode1.rs:270-279` is `pub(crate)`; field `sector.{i}.floorpic` at
`crates/doom-sim/src/snapshot.rs:1116`), runtime line specials (`line.{i}.special`,
`crates/doom-sim/src/snapshot.rs:1372`) and the button slots (`button.{k}.btimer/line/soundorg`,
`crates/doom-sim/src/snapshot.rs:1380-1390`). The renderer can read them from
`Simulation::snapshot()` without touching the simulation (cost: one full capture per rendered
frame; acceptable for the debugger, measurable later). Adding three `pub fn` getters to doom-sim
would be cheaper but touches the simulation crate: see open question Q1.

#### 4.1.2 Player view: `viewx`, `viewy`, `viewangle`, `viewz`

C: `R_SetupFrame` (`linuxdoom-1.10/r_main.c:830-863`) reads `player->mo->x/y`,
`mo->angle + viewangleoffset`, `player->extralight`, `player->viewz`, `player->fixedcolormap`.

- **x, y, angle — (a).** `Actor::position()`, `Actor::angle()` of `OriginalPlayer::mo`, read
  after the tic (post-`P_Ticker`, as `D_Display` reads them). `viewangleoffset` is 0 in a
  single-screen game (b).
- **viewz — not stored.** `docs/COMPATIBILITY.md` (wire 23 "Explicitly not admitted") says the
  view height is not modelled; `crates/doom-sim/src/level.rs:12-14` and
  `crates/doom-sim/src/weapons.rs:609-613` confirm Rust's `P_CalcHeight` computes only `bob`.
  `viewheight`, `deltaviewheight`, `viewz` and the `onground` global exist nowhere in Rust.

  C facts that make `viewz` history-dependent:
  - `P_CalcHeight` (`linuxdoom-1.10/p_user.c:77-141`): `viewz = mo->z + viewheight + bob` with
    `bob = FixedMul(player->bob/2, finesine[(FINEANGLES/20*leveltime)&FINEMASK])`, clamp to
    `mo->ceilingz - 4*FRACUNIT`; live-only `viewheight += deltaviewheight` with the
    VIEWHEIGHT and VIEWHEIGHT/2 limits; if `CF_NOMOMENTUM || !onground` it sets
    `viewz = mo->z + viewheight` **after** a dead clamp (`:97-106`): no bob, no clamp.
  - It runs inside `P_PlayerThink` **before** `P_RunThinkers` (`linuxdoom-1.10/p_tick.c:145-157`,
    `linuxdoom-1.10/p_user.c:267-272`), so the rendered `viewz` uses `mo->z`, `mo->ceilingz`
    and `leveltime` from **before** the tic's own mobj thinker moved the player, while
    `viewx/viewy` are after it. This is genuine vanilla behaviour, not a bug to fix.
  - `onground` is a file-static global set only by `P_MovePlayer` (`p_user.c:157`) and
    `P_DeathThink` (`p_user.c:197`); `P_MovePlayer` is skipped while `mo->reactiontime` is
    nonzero (`p_user.c:267-270`, teleport sets 18, `p_telept.c:120-121`), so `onground` is
    then stale from an earlier tic.
  - `P_ZMovement` (`p_mobj.c:251-258`): smooth step-up `viewheight -= floorz - z;
    deltaviewheight = (VIEWHEIGHT - viewheight) >> 3` when `z < floorz` at entry; hard landing
    (`p_mobj.c:299-307`): `deltaviewheight = momz >> 3` when `momz < -GRAVITY*8`.
  - `P_DeathThink` (`p_user.c:185-198`): `viewheight -= FRACUNIT` down to 6 units,
    `deltaviewheight = 0`.
  - `P_Teleport` (`p_telept.c:106-108`) overwrites `viewz = z + viewheight` after `P_CalcHeight`
    already ran this tic, so the teleport tic renders that value (no bob, no clamp).
  - Level start: `P_SpawnPlayer` sets `viewheight = VIEWHEIGHT` (`p_mobj.c:683`),
    `P_SetupLevel` sets `viewz = 1` (`p_setup.c:604`); `deltaviewheight` is 0 from
    `G_PlayerReborn`'s clear.

  **Derivation (b-history).** Every input the shadow needs is in the canonical state or the
  canonical event stream of each tic:
  - pre-tic `z`, `ceilingz`, `floorz`, `reaction_time`, `playerstate`: the renderer keeps a
    copy from the previous observed tic (`Actor::z/ceiling/floor`, `ActorLife::reaction_time`,
    `OriginalPlayer::state`);
  - `player->bob` as computed in this tic's `P_CalcHeight`: `PlayerWeaponState::bob` (only
    `P_CalcHeight` writes it; genuinely compared, `tools/reference/level_snapshot.py:55-57`);
  - the player's `P_ZMovement` inputs: `Event::Vertical { actor, update }` with
    `update.before = [z, momz]` at entry and `update.constraints = [floorz, ceilingz]` at that
    moment (`crates/doom-sim/src/multisector.rs:121-134, 492-529`; emitted for every actor
    including the player at `crates/doom-sim/src/thinkers.rs:422-439`, under the same
    `z != floorz || momz` gate as `P_MobjThinker`). Step-up iff `before.z < constraints.floor`;
    landing iff `floor_contact && before.momz < -GRAVITY*8`, `deltaviewheight = before.momz >> 3`
    (arithmetic shift), applied after the step-up exactly as in C;
  - the teleport: `MoverEvent::Teleported { actor, line, landing }`
    (`crates/doom-sim/src/teleport.rs:231-235`), last one of the tic wins (a thing can teleport
    twice per tic, `docs/COMPATIBILITY.md` wire 24). The landed `z` is the landing sector's
    floor at that moment; every sector thinker is appended after the player's mobj thinker, so
    that floor is the pre-tic floor (renderer-held copy of the previous tic's `sectors()`), and
    it equals the `z` of the second `MT_TFOG` spawned at `thing->z` (`p_telept.c:113-114`),
    which can serve as a cross-check.
  - `leveltime` for the bob = `tic() - 1` after tic N (P_Ticker increments last,
    `p_tick.c:157`); `CF_NOMOMENTUM`: cheats have no input path in the admitted command domain
    (ticcmds carry no cheats; `docs/COMPATIBILITY.md` "Presentation and cheats"), so 0.

  Preconditions and caveats (hence S1): (i) the shadow must observe every committed tic from
  level start in order (enforce with a tic-number check; a refused tic is not observed); it
  cannot start from a decoded DSIM40 capture alone; (ii) the `events.*` group is **not**
  compared against the genuine engine (`docs/COMPATIBILITY.md` wire 23 "Comparison contract"),
  so `Event::Vertical` / `Teleported` content is Rust-only evidence until the pixel harness (or
  a C-dump of `viewz`) validates it; (iii) it duplicates the presentation half of
  `P_CalcHeight`, `P_ZMovement` and `P_DeathThink` in the renderer.

#### 4.1.3 `extralight`, `fixedcolormap`, powers, psprites

- **extralight — (a).** `PlayerWeaponState::extra_light` (`crates/doom-sim/src/player_weapons.rs:35-36`),
  genuinely compared (`tools/reference/level_snapshot.py:55-57`). Post-tic value is what
  `R_SetupFrame` reads.
- **fixedcolormap — (b-history, one tic).** C sets it at the end of `P_PlayerThink`'s live
  branch (`linuxdoom-1.10/p_user.c:362-383`) from the powers *after* that tic's countdowns
  (`:339-353`) and *before* any pickup in the tic's thinkers; a dead player returns early
  (`:258-262`), keeping the old value. Derivation:
  `fixedcolormap_N = live_at_start_N ? f(countdown(powers_{N-1})) : fixedcolormap_{N-1}`,
  `f` = INVERSECOLORMAP (32) if invulnerability `> 4*32 || &8`, else 1 if infrared
  `> 4*32 || &8`, else 0. Powers: `PlayerPowers` (`crates/doom-sim/src/original_player.rs:108-114`;
  order invulnerability, strength, invisibility, ironfeet, allmap, infrared). Rust's countdown
  is `crates/doom-sim/src/episode1.rs:300-340`. Wire 24 admits the visor (2045) but not the
  invulnerability sphere, so only values 0 and 1 are reachable today; implement the full `f`.
  Level start: 0.
- **Invisibility for psprite fuzz — (a).** `R_DrawPSprite` reads the post-tic
  `powers[pw_invisibility]` (`linuxdoom-1.10/r_things.c`, §2.1.12).
- **psprites — (a).** `PlayerWeaponState::psprites` = `[Psprite { state: Option<StateId>, tics,
  position: [sx, sy] }; 2]` (`crates/doom-sim/src/weapons.rs:204-213`), weapon then flash,
  genuinely compared. State → sprite/frame (with FF_FULLBRIGHT) through
  `actor_registry().state(id)` or `original_state(id)?.descriptor()`. A `None` state is not
  drawn (C `if (psp->state)`).
- Player light for psprites: `mo->subsector->sector->lightlevel` → `Actor::location().sector`
  plus `sector_lights_and_specials()` (a).

#### 4.1.4 Mobjs and the sector thing lists

- x, y, z, angle (a); sprite, frame (incl. FF_FULLBRIGHT), flags incl. `MF_SHADOW` and
  `MF_TRANSLATION` (0 in single player) (a) from `ActorLife`.
- **Order — (a), and validated.** C prepends in `P_SetThingPosition`
  (`linuxdoom-1.10/p_maputl.c:415-420`) and unlinks in `P_UnsetThingPosition` (`:347-365`);
  `R_AddSprites` walks `sec->thinglist` head first (`linuxdoom-1.10/r_things.c:638`), and
  `R_SortVisSprites`' strict `<` (`r_things.c:819-824`) keeps insertion order for equal scales,
  so this order is observable in pixels. Rust keeps one newest-first link order
  (`SpatialIndex::link` = unlink + insert at 0, `crates/doom-sim/src/movement.rs:49-53`) and a
  sector's list is that order filtered by `in_sector_list` (not `MF_NOSECTOR`,
  `crates/doom-sim/src/lifecycle.rs:1782-1784`) and by `location.sector`
  (`crates/doom-sim/src/lib.rs:869-893`). Filtering one global head-insertion order by sector is
  equivalent to per-sector head insertion because every C relink unlinks and relinks the
  sector list and the block list together. This order is canonical (`sector.{i}.actors`,
  `crates/doom-sim/src/snapshot.rs:1075-1094`) and the wire-22..24 comparison checks it equal
  to the genuine dump's sector thing list (`tools/reference/level_snapshot.py:45-48`).
  Performance: `sector_actor_order` allocates a `Vec` per call; the renderer should instead
  bucket `spatial_order()` once per frame (same result).
- The teleport landing (`MT_TELEPORTMAN`) is in no sector list (`docs/COMPATIBILITY.md` wire
  24), as in C (`MF_NOSECTOR`). Puffs, blood and teleport fog are ordinary actors
  (`crates/doom-sim/src/combat.rs:1347`, `crates/doom-sim/src/original_registry.rs:833-839`).
- The viewer's own body is culled by `tz < MINZ` (`linuxdoom-1.10/r_things.c:492`); nothing to do.

#### 4.1.5 Sectors

- floor/ceiling heights (a) `sectors()`.
- lightlevel (a) `sector_lights_and_specials()` under every specials policy; the map light
  under older policies (b).
- **floorpic (a via Snapshot).** Written by plats 20/22 (`linuxdoom-1.10/p_plats.c:187, 199`)
  and donut/change floors (`p_floor.c:232, 243, 368`). Rust stores it as the normalized flat
  name (decision D2, `crates/doom-sim/src/sector_thinkers.rs:23-36, 160-171`), equal to
  `R_FlatNumForName` identity. Read `sector.{i}.floorpic`; map to a flat number with the
  renderer's own `W_CheckNumForName - firstflat`.
- **ceilingpic (b).** No admitted special writes it (only `p_setup.c:250` and `p_saveg.c:184`
  in the source): map value.
- **flattranslation (b).** See §4.1.7.
- specials: irrelevant to the renderer.

#### 4.1.6 Sides and lines

- **rowoffset (b)** static (`p_setup.c` load only; `p_saveg.c` only otherwise).
- **textureoffset (b)** for special 48: `P_UpdateSpecials` adds FRACUNIT to
  `sides[line->sidenum[0]].textureoffset` for every entry of `linespeciallist`
  (`linuxdoom-1.10/p_spec.c:1113-1124`), built once by `P_SpawnSpecials` in line order, at most
  64 (`p_spec.c:1336-1345`; Rust `MAX_LINE_ANIMS`, `crates/doom-sim/src/sector_thinkers.rs:20-22`).
  Every committed tic runs it once, so after tic N:
  `offset = (map_offset << 16).wrapping_add(N * k * FRACUNIT)` where k is the number of list
  entries naming that sidedef (shared sidedefs add twice). DSIM36 itself relies on this
  (`crates/doom-sim/src/diagnostic.rs:246-252`). Wrap after about 32,768 tics is the `-fwrapv`
  behaviour of the genuine strict build.
- **toptexture/midtexture/bottomtexture.** Only `P_ChangeSwitchTexture`
  (`linuxdoom-1.10/p_switch.c:201-263`) and the button restore (`p_spec.c:1127-1153`) write
  them (the sliding-door writes in `p_doors.c:635-701` are inside `#if 0`, `p_doors.c:557, 764`).
  Rust does not simulate switch textures (`crates/doom-sim/src/use_lines.rs:5-13`).
  - Search rule to reproduce: for each `switchlist` index i in order, test top, then mid, then
    bottom (`p_switch.c:224-262`); the first i that matches any of the three wins and is
    replaced by `switchlist[i^1]`. `switchlist` holds `alphSwitchList` entries with
    `episode <= gamemode-episode` (`p_switch.c:107-140`). Because the simulation refuses a map
    whose switch line matches only a non-episode-1 entry (decision D3,
    `crates/doom-sim/src/use_lines.rs:84-92`; `docs/COMPATIBILITY.md` wire 23) and the episode-1
    entries precede the others in `alphSwitchList` (`p_switch.c:50-71`), the episode-1 subset
    gives the same answer for every admitted map; no game-mode input is needed.
  - **Non-reusable (S1, and every `useAgain = 0` call): (b).** `P_ChangeSwitchTexture(line, 0)`
    zeroes `line->special` first (`p_switch.c:212-213`) and nothing else clears a switch
    line's special, so: the front side is flipped iff the map special is one of the
    `P_ChangeSwitchTexture(line,0)` cases in `P_UseSpecialLine`/`P_ShootSpecialLine`
    (`p_switch.c:350-510`, `p_spec.c:985, 997`) **and** `line.{i}.special` reads 0. (D1 manual
    doors and W1 lines also clear specials but never call it: key the derivation on the
    switch-special set, not on "special is 0".)
  - **Reusable (SR 62/63/70, GR 46): (b) while each press starts a button; ambiguous on a
    re-press.** A running `button.{k}` with `line == L` means L's front side shows
    `switchlist[i^1]`; on expiry it is restored to `btexture` (the pre-press texture). But a
    second `P_ChangeSwitchTexture(L, 1)` while L's button runs flips the texture back and
    `P_StartButton` returns without a new slot (`p_switch.c:162-169`). GR 46 calls it on every
    hit whatever `EV_DoDoor` returned (`p_spec.c:988-991`; Rust
    `crates/doom-sim/src/use_lines.rs:283-297`), so one shotgun blast can flip it several times
    in one tic; a fast SR 70 floor can finish and be re-pressed inside 35 tics. The texture is
    then the parity of presses since the button started, which no canonical field holds. The
    switch-sound events (`crates/doom-sim/src/use_lines.rs:59-70`) do not name the line (they
    name `buttonlist[0]`'s sector), so the renderer can *detect* an unexplained switch sound
    (more `sfx_swtchn` events than new buttons plus newly cleared S1 lines) but cannot always
    attribute it. → **S2**.
- **texturetranslation (b).** §4.1.7.
- **line flags (a)** static map flags; the renderer's `ML_MAPPED` write
  (`linuxdoom-1.10/r_segs.c:398`) is automap state: the Rust renderer must not write it and
  nothing in the simulation reads it.

#### 4.1.7 Animated flats and textures (b)

`P_InitPicAnims` (`linuxdoom-1.10/p_spec.c:148-183`) builds `anims` from `animdefs`
(`p_spec.c:101-…`), skipping a texture entry whose start name `R_CheckTextureNumForName` cannot
find and a flat entry whose start lump `W_CheckNumForName` cannot find; `basepic/picnum` are
texture numbers (TEXTURE1 then TEXTURE2 order) or flat numbers (lump index − firstflat);
`numpics < 2` is `I_Error`. `P_UpdateSpecials` (`p_spec.c:1100-1110`) sets, for every anim and
every `i` in `[basepic, basepic+numpics)`,
`translation[i] = basepic + ((leveltime/speed + i) % numpics)` with the `leveltime` of that
tic, i.e. N−1 for tic N (`p_tick.c:153-157`). Rust does not keep the frame, but it is a pure
function of `tic()` and the WAD: identity before the first tic, `f(tic()-1)` after.
Precondition: the C translation tables are identity only if no level or demo ran before the
measured level in that process (`r_data.c:573, 593` set identity at startup; nothing resets
them per level), so the genuine harness must start the level straight from startup (Q4).
The sky is drawn with `R_GetColumn(skytexture, …)` without translation (`r_plane.c:415`).

#### 4.1.8 Map statics, leveltime, sky, validcount

- leveltime (b): `tic()`; for the bob and the anim it is `tic() - 1` (see above).
- sky (b): `G_InitNew` picks SKY1/SKY2/SKY3 by episode for non-commercial games
  (`linuxdoom-1.10/g_game.c:1453-1475`); `G_DoLoadLevel` only overrides for commercial
  (`g_game.c:454-467`). Every admitted profile is Episode 1 → `R_TextureNumForName("SKY1")`;
  `skyflatnum = R_FlatNumForName("F_SKY1")` (`g_game.c:454`); `skytexturemid = 100*FRACUNIT`
  (`r_sky.c:60`). The renderer refuses any other episode until one is admitted.
- validcount (b): only equality with `sector->validcount` matters in the renderer, so a
  renderer-local per-sector frame stamp is equivalent; the gameplay `validcount` is never read.
- Tic 0 (before the first `advance`): `viewz = 1` (`p_setup.c:604`), identity translations,
  `extralight 0`, `fixedcolormap 0`.


### 4.2 Map geometry (doom-map)

`doom_map::Map` (`crates/doom-map/src/parse.rs:80`, accessors `:340-376`) keeps every lump in
source order, raw:

- `Vertex { x: i16, y: i16 }` (`crates/doom-map/src/types.rs:6-11`) → `<<16` as `P_LoadVertexes`.
- `Seg { vertices, angle: u16 (raw BAM bits), linedef, side, offset: i16 }`
  (`types.rs:64-75`): stored as in the lump, not recomputed; C uses `angle<<16`, `offset<<16`.
- `Subsector { seg_count, first_seg }` (`types.rs:78-83`); sector = first seg's side's sector
  (validated consistent, `parse.rs:282-292`), as `P_GroupLines`.
- `Node { origin, delta, bounds: [[i16;4];2] (top,bottom,left,right), children }`
  (`types.rs:94-103`), root last; zero nodes allowed (C `R_RenderBSPNode(-1)` → subsector 0).
- `LineDef { vertices, flags, special, tag, sides: [Option<u16>;2] }` (`types.rs:26-37`);
  `SideDef { offsets: [i16;2], textures: [LumpName;3] (upper, lower, middle), sector }`
  (`types.rs:40-47`; note the order upper/lower/middle is the lump order
  top/bottom/mid); `Sector { heights, flats, light, special, tag }` (`types.rs:50-61`).

Points the renderer must handle itself:
- **Seg backsector** is keyed on `ML_TWOSIDED`, not on the presence of a back sidedef
  (`linuxdoom-1.10/p_setup.c:189-192`). doom-map keeps both, so it is derivable; a map with
  `ML_TWOSIDED` and no back side is C undefined behaviour (`sides[-1]`) → renderer load
  refusal. A back side without the flag renders one-sided.
- Texture names are kept as `LumpName` (exact 8 bytes); resolution to numbers is renderer work
  (`R_TextureNumForName`: `'-'` first byte → 0, otherwise first match by `strncasecmp` over
  TEXTURE1+TEXTURE2, `I_Error` if absent; `linuxdoom-1.10/r_data.c:695-730`). Texture 0 is
  "no texture" in the renderer (the no-texture slot).
- `side()` maps any high-bit sidenum to `None` (`parse.rs:406-412`); C only defines −1. No
  admitted map is affected.
- Nothing is normalized away: no seg angle recomputation, no vertex rounding. The BLOCKMAP is
  not needed by the renderer.


### 4.3 WAD access

doom-wad (`crates/doom-wad/src/archive.rs`) gives a validated directory and borrowed lump
bytes: `Wad::parse` (`:174-203`), `lumps()` in on-disk order (`:209`), `find(name)` = last
match (`:213-215`, `rposition`, like `W_CheckNumForName`'s backward scan), `data(index)`
(`:217-219`), and `WadSet` for load order (`:233-258`). It parses no resource formats.

Name-matching difference: `Lump::is_named` upper-cases **both** the directory name and the
query and stops at NUL (`archive.rs:92-106`); C `W_CheckNumForName` upper-cases only the query
and compares the raw 8 bytes. They differ only for lower-case or NUL-then-garbage directory
names; IWAD directories are upper-case. The renderer should do its own exact C comparison for
resource lookup (or refuse at load on any lower-case/garbage directory name) rather than rely on
`find`.

To add in doom-render (`resources` module), all with checked offsets/sizes and explicit errors,
following the C's own failure points as load refusals:
- PLAYPAL (viewer only: index → RGB), COLORMAP (need ≥ 33×256 bytes for maps 0-31 and 32; C
  aligns to 256 and reads the lump, `r_data.c` `R_InitColormaps`).
- PNAMES (count + 8-byte names), `patchlookup = W_CheckNumForName` per name.
- TEXTURE1 and optional TEXTURE2: `maptexture_t` 32-bit layout (name, masked, width, height,
  columndirectory, patchcount, `mappatch_t[]`); `bad texture directory` and `Missing patch`
  checks (`r_data.c:520, 544`); width mask (`r_data.c:552`).
- `R_GenerateLookup` (`r_data.c:296-374`) exactly: single-patch columns point into the patch
  lump at `columnofs+3` (and ignore `originy`), multi-patch columns get composite offsets;
  the `column without a patch` early `return` (`r_data.c:353-358`) leaves later columns
  unset; `>64k` `I_Error` (`:367`).
- Composites (`R_GenerateComposite`, `r_data.c:228-289`) with a parallel defined-byte mask
  (the buffer is `Z_Malloc`ed and never cleared).
- Patches (`patch_t`: width, height, leftoffset, topoffset, `columnofs[width]`, posts with
  pad bytes) as bounds-checked views into lump bytes.
- Flats: `firstflat = F_START+1`, `lastflat = F_END-1` (lump indices; F1_START etc. inside the
  range are counted), flat number = `W_CheckNumForName(name) - firstflat`; a name resolving
  outside the range is out-of-bounds in C → load refusal.
- Sprites: `S_START..S_END`, `R_InitSpriteDefs` over the `sprnames` list (Rust has it:
  `OriginalSprite::resource_name`, `crates/doom-sim/src/identity.rs:321-327`), with every
  `I_Error` of `R_InstallSpriteLump`/`R_InitSprites` (`linuxdoom-1.10/r_things.c:115-151,
  255-267`) as load refusals; `R_InitSpriteLumps` widths/offsets.
- `modifiedgame` (PWAD) changes sprite lump selection (`patched = W_GetNumForName`); M5a
  should admit IWAD-only (a refusal for PWADs) unless a synthetic-WAD path needs it (see §4.5.5).


### 4.4 Tables

Exists in doom-sim (private):
- `FINE_SINE: [i32; 10240]` (`crates/doom-sim/src/fine_sine.rs:4`, included at
  `crates/doom-sim/src/lib.rs:166`): finesine, and finecosine as `+2048`.
- `TAN_TO_ANGLE: [u32; 2049]` (`crates/doom-sim/src/tan_to_angle.rs:4`).
- `slope_div` and `point_angle` (= `R_PointToAngle` on a delta) at
  `crates/doom-sim/src/movement.rs:534-560`, private.
- Both tables are generated from `linuxdoom-1.10/tables.c` by `tools/reference/generate.py`
  (`generate.py:129-140`) with `--check`.

Missing: `finetangent[4096]` (`linuxdoom-1.10/tables.c:67`), needed by
`R_InitTextureMapping` (`r_main.c:544-612`) and `R_ExecuteSetViewSize`.

Exists in doom-types (public): `Fixed` with `fixed_mul` (64-bit product, `>>16`, truncate;
`crates/doom-types/src/fixed.rs:51-53`) and `fixed_div` = `FixedDiv` guard plus `FixedDiv2` in
IEEE binary64 (`fixed.rs:56-89`), and wrapping `+ - neg` (`fixed.rs:90-108`). This matches
linuxdoom's `m_fixed.c` (the double `FixedDiv2`) and is exactly what `R_InitTextureMapping`,
`yslope`, `distscale`, `R_ScaleFromGlobalAngle` and `R_PointToDist` need. `R_InitTables` and
`R_InitPointToAngle` are `#if 0` float code (`r_main.c:426-442, 509-535`); the shipped tables
are the `tables.c` literals. (§1.10 and §2.8 enumerate the remaining float/double uses.)

Recommendation: **copy, generated, into doom-render** (`tables.rs`: finesine 10240,
finetangent 4096, tantoangle 2049), produced by a new output of `tools/reference/generate.py`
with `--check`, existing outputs byte-unchanged; add a renderer test that recomputes nothing
but asserts the three lengths and spot values against `tables.c` parsing in the generator.
Reason: making doom-sim's tables `pub` or moving them to doom-types edits simulation code,
which the brief forbids; a public doom-types table module would be the clean long-term home
(Q2). Reuse doom-types `Fixed` directly (it is already a shared dependency, not simulation).
`R_PointToAngle`/`R_PointToAngle2`/`R_PointToDist` are reimplemented in doom-render from
`r_main.c` (they use `viewx/viewy` globals and `SlopeDiv` from `tables.c`).


### 4.5 Crate design

#### 4.5.1 Dependency direction

```
doom-types  <-  doom-wad  <-  doom-map  <-  doom-sim
     ^             ^             ^             ^
     +-------------+------ doom-render --------+   (reads doom-sim's public API only)
```

doom-render depends on doom-types, doom-wad, doom-map, doom-sim; nothing depends on
doom-render. Workspace `Cargo.toml` adds the member (`Cargo.toml:2`); `[lints] workspace = true`
inherits `unsafe_code = "forbid"` and `missing_docs = "warn"` (`Cargo.toml:16-18`), and CI
denies Clippy warnings. No new third-party dependency.

#### 4.5.2 Modules (mirroring the C files)

| Module | C | Contents |
| --- | --- | --- |
| `lib.rs` | — | `Renderer`, `RenderConfig`, `Frame`, `RenderRefusal`, `ResourceError`, public docs |
| `tables.rs` | tables.c | generated finesine/finetangent/tantoangle |
| `math.rs` | r_main.c, tables.c | `SlopeDiv`, `R_PointToAngle(2)`, `R_PointToDist`, `R_ScaleFromGlobalAngle`, BAM helpers |
| `resources.rs` (`r_data`) | r_data.c, r_things.c init, w_wad.c lookups | WAD parsing of §4.3, texture/flat/sprite tables, composites with defined-byte masks, `animdefs`, `alphSwitchList` (episode 1) |
| `view.rs` | r_main.c `R_SetupFrame`, p_user.c, p_mobj.c, p_spec.c | `ViewInput` built from `&Simulation`; `ViewShadow` (viewheight, deltaviewheight, onground, viewz, fixedcolormap, previous powers/pre-tic player and sector copies); anim/scroll/switch derivations |
| `r_main.rs` | r_main.c | `R_ExecuteSetViewSize`, `R_InitTextureMapping`, `R_InitLightTables`, `render_player_view` driver |
| `r_bsp.rs` | r_bsp.c | solidsegs, `R_AddLine`, `R_CheckBBox`, `R_Subsector`, `R_RenderBSPNode` |
| `r_segs.rs` | r_segs.c | `R_StoreWallRange`, `R_RenderSegLoop`, `R_RenderMaskedSegRange`, drawsegs, openings |
| `r_plane.rs` | r_plane.c | visplanes, `R_FindPlane/CheckPlane/MakeSpans/MapPlane/DrawPlanes`, sky columns |
| `r_things.rs` | r_things.c | `R_AddSprites`, `R_ProjectSprite`, vissprites (with `overflowsprite`), `R_SortVisSprites`, `R_DrawSprite`, `R_DrawMasked`, psprites |
| `r_draw.rs` | r_draw.c | column, low-detail column, fuzz (with persistent `fuzzpos`), translated column, span, low span; destination = `Frame` with `ylookup/columnofs` offsets |
| `r_sky.rs` | r_sky.c | sky constants |
| `png.rs` | — | tiny palette PNG writer (viewer/debug only) |
| `gif.rs` (optional) | — | animated GIF writer for the viewer |

Per-frame work state (`FrameScratch`: solidsegs, drawsegs, visplanes, openings, vissprites,
clip arrays, cachedheight…) is preallocated once at the C's fixed sizes and reused, so a frame
allocates nothing.

#### 4.5.3 Types

```rust
pub struct Frame(pub Box<[u8; 320 * 200]>);         // palette indices, row-major
pub enum Detail { High, Low }
pub struct RenderConfig { pub screenblocks: u8 /* 3..=11 */, pub detail: Detail }
pub struct Resources { /* validated r_data tables, owned copies of the needed lump bytes */ }
pub struct Renderer { res: Resources, map: MapTables, shadow: ViewShadow, fuzzpos: usize, scratch: FrameScratch, cfg: RenderConfig }
impl Renderer {
    pub fn new(wads: &WadSet, map: &doom_map::Map, cfg: RenderConfig) -> Result<Self, ResourceError>;
    pub fn begin_level(&mut self, sim: &Simulation) -> Result<(), RenderRefusal>; // tic 0
    pub fn observe(&mut self, sim: &Simulation) -> Result<(), RenderRefusal>;     // after each committed advance
    pub fn render(&mut self, sim: &Simulation, out: &mut Frame) -> Result<(), RenderRefusal>;
}
```

- `observe` must be called exactly once per committed tic; it checks
  `sim.tic() == last_observed + 1` (`RenderRefusal::ShadowGap`). It reads `events()`,
  the player, powers, `sectors()` and the Snapshot fields, and updates the shadow; it never
  needs `&mut Simulation`.
- `render` borrows `&Simulation` only. All outputs are written to `out` and to renderer-owned
  scratch. On refusal, `out` and `fuzzpos` are left as before (render into scratch indices,
  commit `fuzzpos` only on success; the frame buffer can be double-buffered or the refusal
  can be documented as leaving `out` unspecified: Q5).
- Default config: screenblocks 11 (full 320×200 view, no status bar), high detail. The view
  window outside `R_RenderPlayerView`'s rectangle is not written by the C function (border and
  status bar are `D_Display`'s), so the comparison should be over the view window with the rest
  of `Frame` pre-filled identically on both sides.

#### 4.5.4 Errors at vanilla limits

Principle (AGENTS.md "A fidelity gap is a refuse-the-tic witness"; here a refuse-the-frame
witness): reproduce every **defined** source behaviour, including silent drops; refuse where
C has `I_Error` or undefined behaviour; never silently diverge.

| Situation | C | Rust renderer |
| --- | --- | --- |
| 257th drawseg | `R_StoreWallRange` returns (`r_segs.c:386-387`) | reproduce (silent drop) |
| 129th vissprite | written to `overflowsprite` and never drawn (`r_things.c:326-335`) | reproduce |
| 129th visplane | `I_Error` (`r_plane.c:245-246`) | `RenderRefusal::Visplanes` |
| openings past 20,480 | no check at write (`r_segs.c` 607-613, 718-732), post-hoc RANGECHECK `I_Error` after memory is overrun (`r_plane.c:384-385`; RANGECHECK on, `doomdef.h:75`) | `RenderRefusal::Openings` before the write |
| solidsegs past 32 | unchecked write (`r_bsp.c:88-92`) | `RenderRefusal::SolidSegs` |
| sprite/frame out of range | `I_Error` in `R_ProjectSprite`/`R_DrawPSprite` | `RenderRefusal::SpriteFrame` |
| RANGECHECK `I_Error`s in drawers and `R_MapPlane` (`r_draw.c:122, 231, 311, 401, 534, 657`; `r_plane.c:137`) | fatal | `RenderRefusal::Range` |
| texel fetch past lump end, uninitialised composite byte, column after a "column without a patch" | undefined read | **S4** (recommended `RenderRefusal::UndefinedTexel`) |
| signed overflow in renderer arithmetic | UB in ISO C; wraps under the genuine strict `-fwrapv` build | wrapping, documented as the declared genuine build's behaviour (Q3) |

Load-time failures (`I_Error` in `R_InitTextures`, `R_InitSprites`, `R_TextureNumForName`,
`R_FlatNumForName`, `P_InitPicAnims`, missing switch texture) are `ResourceError`s from
`Renderer::new`.

#### 4.5.5 Feeding states tic by tic without touching the simulation

The whole-level runner is `crates/doom-sim/examples/player_level_run.rs:161-304`: build the
map, select `original_player_episode1_v2_map(map, n, skill, …)` (`:214-219`), call
`spawn_map_things`, `initialize_player_weapons`, `spawn_level_specials` (`:225-227`), snapshot
tic 0 (`:232`), then per demo command `s.advance([cmd, default×3])` (`:245-256`) and read
`s.events()` / `s.snapshot()` (`:258-263`). The E1M1 script replays
(`crates/doom-sim/src/e1m1_replay.rs`) are test-internal and drive operations, not a renderer
use case.

A renderer hook is a new example in doom-render (e.g. `crates/doom-render/examples/render_level_run.rs`)
that repeats that setup using only doom-sim's public API and demo reader (`doom_sim::demo`,
`crates/doom-sim/src/lib.rs:140`):

```
let mut r = Renderer::new(&wads, &map, cfg)?;   // after spawn_level_specials
r.begin_level(&s)?; r.render(&s, &mut f)?;       // tic 0 (viewz = 1)
for cmd in demo { s.advance([cmd, ..])?; r.observe(&s)?; r.render(&s, &mut f)?; write digest/PNG }
```

No simulation code changes; the simulation's digests are unaffected because nothing is
written to it. The same loop is the Rust half of a pixel comparison with a genuine C runner
that calls `R_RenderPlayerView` after each tic (the existing `tools/reference/m4a_driver.c`
lineage, under a new schema so the old one stays byte-identical). For CI without IWADs the
synthetic mechanism WADs (`crates/doom-sim/tests/episode1_synthetic.rs`) would need synthetic
PLAYPAL/COLORMAP/PNAMES/TEXTURE1/patches/flats/sprites added by a new generator (existing
fixtures unchanged). Committed results: frame digests and counts only (AGENTS.md); PNG/GIF
output local only.

#### 4.5.6 Persistent renderer state across frames

`fuzzpos` (`linuxdoom-1.10/r_draw.c:274, 358-362`) is a never-reset global, so the fuzz pattern
of a frame depends on every fuzz pixel drawn since program start. The renderer owns it and the
comparison contract must fix the frame sequence (render every tic from tic 0, or an explicit
list) identically on both sides (S3). The texture/flat translation tables (§4.1.7) are the
other cross-frame carry-over; composites are built lazily but deterministic except for their
undefined bytes (S4). §2.7 lists the remaining shared globals.


### 4.6 MSRV 1.85.0, dependencies, image output

- Toolchain: `rust-toolchain.toml` is stable with clippy/rustfmt; local `rustc 1.100.0`;
  `1.85.0` toolchain installed. Workspace `rust-version = "1.85"`, edition 2024 (`Cargo.toml:9-10`).
  Nothing the renderer needs is newer than 1.85 (`Box<[u8; N]>` via `vec![0; N].into_boxed_slice().try_into()`,
  const generics, `div_euclid`, `wrapping_*`). Avoid `<[T]>::as_chunks` (1.88) and
  `unsigned_abs` on new types beyond what 1.85 has (fine for i32).
- No new dependency. Cargo.lock has 36 packages, none for PNG/GIF/deflate/CRC.
- **PNG: hand-written** (~150 lines): signature, IHDR (8-bit, colour type 3), PLTE from
  PLAYPAL palette 0 (or the `ST_doPaletteStuff` palette for viewer realism, derived from
  damagecount/bonuscount/strength, never affecting indices), IDAT as a zlib stream of
  **stored** deflate blocks (≤ 65,535 bytes each, filter byte 0 per row) with Adler-32, IEND,
  CRC-32 per chunk from a 256-entry table computed in a `const fn`. A 320×200 frame is ~64 KB.
  Recommended over a crate: no MSRV or supply-chain cost, trivially testable (decode with
  Python's `zlib`/`struct` in a tool test).
- **GIF/video:** `which ffmpeg` (also gifsicle, convert, magick) returns nothing on this
  machine. Recommend a hand-written GIF89a writer (~200 lines: global colour table from
  PLAYPAL, NETSCAPE2.0 loop, per-frame delay 3/100 s ≈ 35 Hz rounded, "uncompressed" LZW that
  emits a clear code every 254 literals so no dictionary is needed). Optional: a PNG sequence
  plus instructions for ffmpeg where installed. Both live in an example or a `viewer` module
  behind no feature flag (no deps to gate).


### 4.7 Size estimate (lines of Rust, excluding blank/doc-heavy variance ±25%)

| Module | Lines |
| --- | --- |
| `lib.rs` (API, types, errors, docs) | 300 |
| `tables.rs` (generated) | ~1,400 (generated, not hand-written) |
| `math.rs` | 200 |
| `resources.rs` (r_data, sprites, lookups, anim/switch tables) | 950 |
| `view.rs` (ViewInput, ViewShadow, derivations) | 600 |
| `r_main.rs` | 400 |
| `r_bsp.rs` | 400 |
| `r_segs.rs` | 550 |
| `r_plane.rs` | 350 |
| `r_things.rs` | 650 |
| `r_draw.rs` | 300 |
| `r_sky.rs` | 30 |
| `png.rs` + `gif.rs` | 350 |
| example runner/viewer | 300 |
| unit + integration tests (synthetic WAD builder, refusal witnesses, shadow tests) | 1,200 |
| **Total** | **≈ 6,600 hand-written + 1,400 generated** |

Plus outside the crate: generator output for the tables (Python, ~40 lines), a synthetic
graphics-WAD generator (~400 lines Python), and the genuine C frame runner (owned by the C
side, not estimated here).


### 4.8 Stop-and-ask candidates

- **S1. `viewz` is not in canonical state.** Exact re-derivation is possible only as a
  renderer-owned shadow of `P_CalcHeight`'s view half, the player half of `P_ZMovement`,
  `P_DeathThink` and `P_Teleport`'s `viewz` write, driven by the pre-tic player state, the
  stored `bob`, and the per-tic `Event::Vertical` and `MoverEvent::Teleported` of the player.
  It needs every tic observed from level start and relies on the `events.*` group, which the
  genuine comparison does not check. Recommendation: accept the shadow (no simulation change),
  and validate it by dumping `viewz`, `viewheight`, `deltaviewheight` and `fixedcolormap` from
  the genuine runner under a new schema and comparing per tic. The alternative (adding these
  fields to the state) needs a new wire, which the brief rules out.
- **S2. Reusable switch textures after a re-press.** SR/GR lines pressed or shot again while
  their button runs flip the texture back without a new button (`p_switch.c:162-169, 224-262`;
  GR 46 on every hit, `p_spec.c:988-991`). No canonical field or line-naming event records the
  parity. Options: (a) a refuse-the-frame witness from the first unexplained switch sound
  until every running button on reusable lines has expired (exact otherwise; the restore to
  `btexture` re-synchronises); (b) owner approves exposing or adding the side textures
  (simulation change). Recommendation: (a).
- **S3. `fuzzpos` is persistent renderer state** (`r_draw.c:274, 358-362`). Spectres, partial
  invisibility and the shadowed psprite depend on every fuzz pixel drawn since program start.
  Need an owner ruling on the frame-sequence contract (e.g. "both sides render every tic from
  tic 0 of a freshly started level, nothing before"), and whether a refused frame advances it
  (recommend: no).
- **S4. Undefined texel reads.** Wall columns use `& 127` regardless of texture height
  (`r_draw.c:142, 247`), so short single-patch columns read past the post into the rest of the
  patch lump and possibly past its end (zone memory), and multi-patch composites read bytes no
  patch wrote (`R_GenerateComposite`, `r_data.c:228-289`, `Z_Malloc` not cleared) or columns
  left unset after "column without a patch" (`r_data.c:353-358`). Reads inside the lump are
  deterministic and reproducible; reads past the lump or of unwritten composite bytes are not.
  Recommendation: reproduce in-lump reads exactly, refuse the frame (`UndefinedTexel`) on the
  rest. Needs owner confirmation because a frame of a real map may hit it (vanilla can show such artefacts).

## 5. Owner decisions (R1–R16) with recommendations

Each is tested against four questions: (a) consistent with prior owner decisions (E1–E9, F1–F11, G1–G13, H1–H12, I1–I13, P1–P9) and the M5a brief? (b) no silent divergence? (c) no new kind of fidelity gap beyond declared refuse-the-frame render witnesses? (d) bounded stores and work? Status: **R1–R16 confirmed by the owner on 2026-11-15** as recommended, with the notes under "Owner rulings" below (Phase 1 had stopped under the unattended rule; see "Why Phase 1 stopped").

**R1. Scope.** As the brief: R_RenderPlayerView's framebuffer (BSP, walls with pegging and offsets, masked midtextures, visplanes, sky, sector and distance light, fake contrast, extralight, fixed colormaps, sprites, psprites with bob and flash, fuzz). Out of scope (M5b): status bar, HUD, automap, menus, palette flashes, window, input, sound. Player colour translation (players 2–4 only) is not reachable in single player and is refused if met. (a)–(d) yes. **Recommendation:** as stated.

**R2. View configurations compared.** The C defaults are screenblocks 9, detail 0 (m_misc.c:279-280); the menu's low-detail toggle is disabled in this source (m_menu.c:1131-1147), and low detail is broken here (`R_DrawColumnLow` doubles `dc_x` in place, `R_DrawSpanLow` overruns; §2.2). Options: (i) screenblocks 11 (full 320×200) and 10 (320×168 above the status bar), high detail, plus 9 (the vanilla default, bordered) on synthetic rooms; low detail refused with a declared configuration refusal; (ii) the above plus low detail on synthetic rooms only, comparing whatever the genuine overrun produces where it is deterministic. The prototype ran 11, 9 and 10-low and all three variants agreed on synthetic rooms. (a)–(d) yes for (i); (ii) risks (c) on layout-dependent overrun bytes. **Recommendation:** (i): 11 and 10 everywhere, 9 on synthetic rooms; low detail refused and recorded as a known limitation for a later milestone. Comparison region: the full 320×200 buffer; outside the view window both sides keep the initial zero bytes (the runner never draws border or status bar).

**R3. Vanilla limits.** Per §1.8 and §2.9:
- MAXDRAWSEGS (256): silent drop, deterministic (r_segs.c:386-387) → reproduce exactly.
- MAXVISSPRITES (128): silent drop into `overflowsprite` (r_things.c:330-331) → reproduce exactly.
- MAXVISPLANES (128): `I_Error` in R_FindPlane, but R_CheckPlane writes past the array unchecked and the `==` test can be skipped; the only later check is in R_DrawPlanes after the out-of-bounds writes → refuse the frame with render witness `VisplaneOverflow` at the first allocation that would exceed 128 (either path).
- MAXOPENINGS (20,480): unchecked during the frame, `I_Error` after the writes → refuse with `OpeningOverflow` at the first write past the bound.
- solidsegs (MAXSEGS 32): unchecked, layout-dependent → refuse with `SolidSegOverflow`.
- RANGECHECK `I_Error`s (bad sprite frame or rotation, column y out of range) → refuse with the matching witness.
The genuine C after an `I_Error` exits; the run's frame stream ends there, so the comparison is "the Rust frame is refused at exactly that tic". (a) yes (H3's refuse-the-tic pattern, applied to frames); (b)–(d) yes. **Recommendation:** as stated.

**R4. Undefined texels.** `R_DrawColumn` indexes `dc_source[(frac>>16)&127]` whatever the texture height (r_draw.c:142); short single-patch columns read past the post into the rest of the patch lump and can run past the lump into zone memory; composite textures contain bytes no patch wrote (Z_Malloc not cleared, r_data.c:228-289). The prototype saw these differ between two strict runs of the same script (ASLR). Options: (i) reproduce reads inside the lump exactly; mark a frame **non-comparable** with witness `UndefinedTexel` when any pixel's source is outside the lump or an unwritten composite byte; the genuine runner gets a read-provenance observer (`--wrap` of the column drawers plus a lump bounds table) so the C side marks the same frames, and the reports count them; (ii) a C-side pixel mask comparing all other pixels of such frames. Under (i) Rust still renders the frame (and advances `fuzzpos`) so later frames stay in sequence. (a)–(d) yes for (i). **Recommendation:** (i), with the count of non-comparable frames reported per run; (ii) as a later refinement if the counts are large.

**R5. `ML_MAPPED` (the one dump field rendering changes).** `R_StoreWallRange` sets `linedef->flags |= ML_MAPPED` (r_segs.c:398); read only by the automap (am_map.c:1128) and savegames (p_saveg.c:141). The prototype measured that rendering every tic changes bit 256 of `L.flags` and nothing else on the real maps (2,377,888 rows); synthetic WADs never showed it. This contradicts the brief's expectation of identical game dumps, so it needs a ruling. Options: (i) the new M5a runner masks bit 256 of line flags in its game dump (a declared projection of the new runner only; frozen runners untouched), and the per-frame mapped-line set travels in the frame stream; the Rust renderer keeps its own mapped set (presentation state, outside the simulation) and that set is compared too; (ii) render in a separate process pass from the comparison runs (two runs per script). (a) yes for both; (b) yes; (d) yes. **Recommendation:** (i). The proof the brief asks for becomes: render-every-tic and never-render game dumps are identical after masking bit 256, and every unmasked byte matches the frozen M4b dumps.

**R6. `viewz` and the view shadow.** The Rust state holds no `viewz`, `viewheight`, `deltaviewheight` or `onground`. The C renders the `viewz` computed in P_CalcHeight during the player think, before the tic's movement (p_user.c:88-140, p_tick.c). It is derivable exactly by a renderer-owned "view shadow" that observes every committed tic from level start (previous tic's z, ceilingz, floorz, reactiontime, playerstate; stored `bob`; the player's vertical-move and teleport events). It relies on the events group, which the genuine comparison does not check. Options: (i) the shadow, validated against `viewz`, `viewheight`, `deltaviewheight` and `fixedcolormap` dumped per tic by the new runner (a new M5a view schema, not the game dump); (ii) add the fields to the state (new wire; the brief rules it out). (a) yes for (i); (b) yes because the validation compares every tic; (d) yes. **Recommendation:** (i).

**R7. Reusable switch textures after a re-press (not derivable).** Pressing or shooting an SR/GR line again while its button runs flips the texture back without a new button (p_switch.c:162-169, 224-262; GR 46 on every hit, p_spec.c:988-991); no canonical field or line-naming event records the parity. Non-reusable switches and first presses are derivable exactly. Options: (i) a refuse-the-frame render witness `SwitchParityUnknown` from the first unexplained switch sound until every running button on a reusable line has expired (the restore to `btexture` re-synchronises); (ii) expose or add the side textures in the simulation (simulation change: out of the brief). The brief says "if it truly can't be derived, stop and ask"; this is that case. (a) (i) adds nothing to the simulation; (b) yes; (c) a new witness kind; (d) yes. **Recommendation:** (i), with the number of such frames counted per run (expected rare: it needs a re-press within 35 tics).

**R8. Frame-sequence contract (`fuzzpos`, stale pixels).** `fuzzpos` is never reset (r_draw.c:274, 358-362) and the renderer never clears the view rectangle, so uncovered pixels (HOM) keep the previous frame. Contract: both sides start from a zeroed 320×200 buffer and `fuzzpos` 0 at level load in a freshly started process, render after every dumped tic (the point where D_Display runs, d_main.c:267), never draw HUD, border or status bar, and keep one persistent buffer. A refused frame (R3) ends the run; a non-comparable frame (R4, R7) is still rendered on both sides. (a)–(d) yes. **Recommendation:** as stated.

**R9. Which frames are compared on real maps.** Every tic of every committed script: 158,966 frames per view configuration over the E1M1–E1M9 scripts (prototype count), plus DEMO1 (1,710 tics). Local only (IWAD). **Recommendation:** every tic, both R2 configurations.

**R10. What is committed.** Synthetic rooms: PNG images (through the synthetic PLAYPAL) of selected frames plus per-frame SHA-256 lists for every frame. IWAD-derived: per-run digests of the frame stream and counts (frames compared, non-comparable, refused) only; per-frame digest lists and raw frames stay in scratch; no IWAD-derived image is ever committed. **Recommendation:** as stated.

**R11. Genuine runner and sanitizer adapters.** A new forked driver `tools/reference/m5a_driver.c`, `m5a_build.py`, `m5a_runner_contract.json` and `m5a_reference.py` (new files only), compiling the whole renderer. The prototype needed one LP64 ABI adapter (r_draw.c:464, pointer cast to `int`) and about 16 declared sanitizer adapters for the overflow and address variants (negative/overrun table indices that land in the neighbouring table in the genuine build, `abs(INT_MIN)`, visplane pad reads, span-step overflow, rowoffset sums); strict and strict32 compile the genuine text and anchor the values, and the three-variant identity proves the adapters change no frame byte. `sprnames` has no NULL terminator (info.c:40-55) while R_InitSpriteDefs counts to NULL (r_things.c:191): the driver passes a NULL-terminated copy. (a) yes (I3's pattern); (b)–(d) yes. **Recommendation:** as stated.

**R12. Arithmetic.** Renderer arithmetic wraps exactly as the strict (`-fwrapv`) genuine build, since the genuine renderer overflows on ordinary frames (span steps, scale math) and strict and strict32 agree. Where the genuine build's result depends on reading a neighbouring table (the `finesine[negative]` → `finetangent` case), Rust reproduces the specific contiguous-table value only if strict and strict32 agree on it; otherwise refuse. (a) a deliberate exception to H3's checked arithmetic for presentation only; it never touches the simulation; (b)–(d) yes. **Recommendation:** as stated.

**R13. Synthetic graphics set.** A generator `tools/reference/m5a_gfx.py` (no id art): our own PLAYPAL (14 palettes), COLORMAP (34 maps: a deterministic darkening rule plus map 32 inverse), PNAMES, TEXTURE1 (multi-patch, non-power-of-two heights, SKY1 at least 128 tall, every switch texture P_InitSwitchList needs, keeping the existing texture and flat order and sizes so gameplay indices are unchanged), flats including F_SKY1, and sprites for the actors the rooms use with all frames up to the highest letter and all 8 rotations where rotated (r_things.c:255, 267). Rooms from `tools/reference/m5a_wad.py`, one per feature, plus a VIEW room swept through screenblocks. **Recommendation:** as stated.

**R14. Rust crate.** `crates/doom-render`, depending on doom-types, doom-wad, doom-map and doom-sim; nothing depends on it; no unsafe; one module per C file; `Renderer::observe(&Simulation)` after each `advance`, `render(&Simulation, &mut Frame)`. Runtime values the simulation keeps only in its canonical snapshot (runtime floor flats, line specials, button slots) are read through `Simulation::snapshot()`, so doom-sim is not edited; three read-only getters would be cheaper but edit the simulation crate, so they are deferred to an owner decision after M5a measures the cost. Tables (`finetangent`, and copies of finesine and tantoangle) are generated from tables.c into doom-render by a generator with `--check`. Lump lookup uses the C's exact semantics (doom-wad's `is_named` upper-cases directory names; the renderer refuses directory names that would match differently). PNG and GIF writers are hand-written (no new dependency; ffmpeg is not installed). **Recommendation:** as stated.

**R15. CI.** Synthetic rooms' frames against goldens on Linux, Windows and macOS (Rust side); the genuine renderer runs on Linux in the three variants; existing check names preserved. Performance is recorded locally, not gated, when the owner says the machine is idle. **Recommendation:** as stated.

**R16. Local viewer.** An example renders a script run to an animated GIF (hand-written encoder) in `/mnt/d/DoomRenders`, never committed; first run: E1M1 walk_to_exit. **Recommendation:** as stated.

### Owner rulings (2026-11-15)

R1–R16 were confirmed by the owner on 2026-11-15 as recommended, with these notes, which bind Phase 2:

- **R4.** Where the C read tracker can identify the affected columns, every other pixel of the frame is compared and only the undefined columns count as non-comparable; the whole frame is non-comparable only where the tracker cannot identify the columns. Non-comparable counts are reported per map and per texture.
- **R7.** The number of frames the `SwitchParityUnknown` witness skips is reported per map. If it is more than a handful on any real map, work stops and the owner is told.
- **R5, R6, R7.** The renderer's own history (the view shadow, the switch parity and the mapped-line set) is a self-contained render state that can be saved and restored next to a simulation snapshot, so that a later rewind can render any tic directly. A test renders tic N straight through and, separately, restores the snapshot and render state at tic N−k and continues; the frames must be identical.
- **R12.** Every renderer wrapping adapter is declared in the sanitizer builds as in I3, including the two real-map overflows (r_segs.c:603-604) the prototype had not yet adapted.

### Why Phase 1 stopped

The unattended rule allows Phase 2 only when every decision is consistent with prior owner decisions, has one clear recommendation and changes nothing in the simulation. Three items fail the first test or need a ruling the brief reserves for the owner: R7 is the "truly can't be derived" case the brief says to stop on; R5 contradicts the brief's expected proof (rendering does change one dump field, `ML_MAPPED`); R4 and R12 introduce a new kind of comparison outcome (non-comparable frames) and a presentation-only exception to checked arithmetic. Nothing in the simulation needs to change under any recommendation.

## 6. Plan, size estimate and cap

### 6.1 Steps

Phase 2 as the brief, one commit per step, pushed: (1) the synthetic graphics generator, rooms, the M5a runner (adapters, provenance observer, view schema) and genuine goldens in the three variants; (2) the Rust renderer, feature by feature in this order, each matched against its rooms before the next: tables and view setup, BSP and solid walls, two-sided walls and pegging, visplanes and sky, lighting, masked midtextures, sprites, sprite clipping, psprites and bob, fuzz, fixed colormaps, limits; (3) the first-difference reporter with local PNG pairs; (4) real maps: E1M1 every script and tic, then E1M2–E1M9 and DEMO1; (5) the local GIF viewer and the E1M1 walk-to-exit render; (6) CI; (7) tests, a renderer fuzz target, docs, mutation testing, fuzz campaigns, the independent review, full gates and the PR.

### 6.2 Size

Rust renderer about 6.6k hand-written lines plus about 1.4k generated table lines (§4.7); tests 1.5k–2.5k; Python and C tools (runner fork, build, graphics and room generators, comparison and reporting) 3k–4.5k; docs about 2k–3k: **about 13k–17k lines** against a **25k cap** on code, tools and docs. Generated data verified by `--check` tools (goldens, digest lists, manifests, contracts, tables) is reported separately.

### 6.3 Fallback

If the projection passes the cap, M5a covers the synthetic rooms plus E1M1 (every script, every tic, both configurations), and E1M2–E1M9 and DEMO1 move to M5a.2.

## 7. Open questions and where each is decided

The Phase 1 investigators each ended with open questions; they are merged here, each with the decision (§5) that
settles it or "open".

**C world facts (§1)**

1. Does the canonical state include `line.flags` (`ML_MAPPED`, r_segs.c:398) or the validcount stamps? Settled by
   R5: the dump includes line flags, the new runner masks bit 256 and carries the mapped set in the frame stream.
2. Is the negative `finesine` index in `R_ScaleFromGlobalAngle` reachable on real maps? Partly settled by R12
   (reproduce only if strict and strict32 agree, else refuse); reachability on real maps is **open** until a C
   probe runs in Phase 2.
3. Is a viewer exactly on a vertex reachable on real maps? **Open**: R12 covers out-of-range table reads, but the
   vertex case needs the same probe.
4. Which screenblocks values and is low detail in scope? Settled by R2.
5. Persistent framebuffer bytes (uncovered pixels): settled by R8.
6. What state does a refused frame carry (counts at the point of overflow)? R3 names the witnesses and the
   first-overflow point; the carried counts are **open**.
7. LP64 pointer casts (`colormaps` alignment, r_data.c:642, r_draw.c:464): settled by R11.

**C things, drawers and view inputs (§2)**

8. Frame cadence and capture point, initial buffer contents and HOM policy: settled by R8 (every dumped tic,
   zeroed buffer, no HUD, border or status bar).
9. Undefined texels: refuse or model? Settled by R4 (mark non-comparable); the local census of composite gaps,
   patchless columns and negative-origin multi-patch columns was not run and is **open** (R4's reported counts
   will size it).
10. Admitted configurations and player translation: settled by R1 and R2.
11. `FixedDiv` precision (binary64 SSE versus an i386 x87 original build): **open**; R12 settles wrapping, not
    this. The reference build is the SSE x86-64 build used by every earlier milestone.
12. May the renderer read stored `viewz`, psprite sx/sy and the translation arrays from the snapshot? `viewz` is
    not stored: settled by R6 (shadow); sx/sy and the translation arrays through `Simulation::snapshot()`: R14.

**Harness (§3)**

13. Undefined C pixels (refuse versus a C-side mask): R4 option (i) recommended, (ii) a later refinement.
14. `ML_MAPPED`: mask and carry the set: R5.
15. Contract view configurations and low detail: R2.
16. Frame digest computed in the C driver (about 120 lines of SHA-256) or by a pipe to Python: **open**; R10
    fixes only what is committed, not who hashes.
17. Sky for Episode 1 only (`SKY1`) or also the other episodes' skies (g_game.c:462-467, 1457): **open**; R13's
    synthetic set provides `SKY1` and the real runs are Episode 1.
18. Committed PNGs of synthetic rooms and how many: R10 says selected frames; the number is **open**.
19. DEMO1 renders: R9 includes DEMO1 with the M4b extraction and patch path.

**Rust design (§4)**

20. Q1, read-only getters on doom-sim: deferred by R14 to an owner decision after M5a measures the cost; the
    default reads `Simulation::snapshot()`.
21. Q2, tables copied versus moved into doom-types: R14 (generated copy, with `--check`).
22. Q3, wrapping arithmetic: R12.
23. Q4, the genuine process starting the level straight from startup: R8.
24. Q5, is `Frame` left unchanged on a refused frame? R3 ends the run at a refused frame, so the comparison does
    not depend on it; the API behaviour is **open**.
25. Q6, screen geometry and comparison region: R2 (full 320x200 buffer, zero outside the view window).
26. Q7, IWAD-only (`modifiedgame` false) with the synthetic graphics treated as the IWAD in CI: **open**; the C
    runner loads the synthetic graphics as a PWAD over the map WAD in the prototype (§3.5), and R13 does not say
    otherwise.
27. Q8, `Lump::is_named` normalisation of directory names: R14 (exact C lookup semantics, refuse differing
    names).
28. S1 to S4 (stop-and-ask items): R6, R7, R8 and R4 respectively.

**Phase 2 findings**

29. **`SolidSegOverflow` refuses at the first out-of-bounds access, read or write** (orchestrator
    application of R3, 2026-11-15; accepted by the owner 2026-11-15). In SOLIDSEGS at screenblocks 9
    the genuine crunch loop `while (next++ != newend) *++start = *next;` (r_bsp.c:178-182) reads
    `solidsegs[32]` in the frame of tic 18, one frame before the first write past the array at tic 19
    ([evidence](validation/M5A.md#limit-rooms)). R3 refuses where the C's behaviour depends on memory
    layout, so the Rust renderer must refuse that run at tic 18, the first access past `solidsegs[]`.
    Implemented in step 2b: the Rust renderer refuses that run at tic 18 and the screenblocks 11 and 10
    runs at tic 15 ([evidence](validation/M5A.md#rust-renderer-step-2b)).
30. **Outside M5a's scope, reported to the owner and not acted on** (confirmed by the owner
    2026-11-15): outside commercial mode `P_LoadThings` stops reading THINGS entirely at the first
    DoomEd 65 (`break` out of the loop, linuxdoom-1.10/p_setup.c:317-337), while doom-sim's
    `EPISODE1_MONSTERS` (crates/doom-sim/src/episode1.rs:21) admits 65. No Episode 1 IWAD map contains
    a 65, so real-map results are unaffected, but a synthetic WAD with a 65 under the Episode 1 policy
    would diverge from the genuine engine.
31. **The frozen M4b WADs have seg angle 0 and offset 0** on every seg (`m3h6_geometry.room`). The
    renderer reads both from the WAD as the C does (`P_LoadSegs`, p_setup.c:181-182), so the genuine
    frames of the 22 M4b scripts are the output for those bytes; the M5a rooms rewrite them
    (`m5a_wad.fix_segs`).
32. **"Column without a patch" (r_data.c:353-358) leaves that column's lump and offset uninitialised**
    (Z_Malloc memory; the M5a runner's declared ZeroFill makes them 0). The Rust renderer marks such a
    column undefined (every read flagged, decision R4) and gives later columns the last patch's lump
    and offset, as the early `return` leaves them (step 2a, `ColumnEntry::Undefined`). No synthetic
    texture has such a column; whether the genuine tracker flags the same columns is open until a
    texture with one is rendered (step 4 census, open question 9).
    After step 2a this was still not reached: a texture with a patchless column must be appended to
    `TEXTURE1`, which changes `m5a_gfx.wad` and the directory every earlier room WAD carries, so no
    room could add one without changing existing outputs
    ([evidence](validation/M5A.md#rooms-added-after-step-2a)); open, for an owner decision or the step-4
    census.
33. **The view records exercise only part of the view shadow** (step 2a). Over the 8,238 records of
    the 42 items, `viewheight` is always `VIEWHEIGHT` and `deltaviewheight` always 0 (no step-up,
    hard landing or death changes them), `onground` is 0 only in the 42 load records,
    `fixedcolormap` is nonzero in 93 and the invisibility power in 86. Teleports are covered. The
    step-up, landing and death paths of the shadow are matched against the C source only until the
    real maps (step 4) or a room exercising them supply genuine records.
    **Settled after step 2a:** the VIEWZ room supplies genuine records of five step-ups (the dip reaching
    the `VIEWHEIGHT/2` clamp), the airborne path, a hard landing reaching the clamp and the death descent
    to `6*FRACUNIT` with the camera turning toward the attacker; AMPFLICK supplies the end-of-power
    flicker. The view shadow matched all 13,252 records, every field
    ([evidence](validation/M5A.md#rooms-added-after-step-2a)).
34. **Multi-patch masked midtextures (the Medusa effect) cannot have genuine goldens with the current
    graphics set** (after step 2a). `R_RenderMaskedSegRange` reads a composite column as posts
    (r_segs.c:181-184, r_things.c:351-388); no composite of `m5a_gfx.wad` holds palette index 255, so the
    post walk leaves the composite and runs through zone memory: every reported frame was a whole-frame
    fallback of the tracker, and whether the run finishes depends on the memory layout
    ([evidence](validation/M5A.md#rooms-added-after-step-2a)). The Rust renderer must refuse or mark such
    frames non-comparable as a whole (decision R4); a room with terminating composite bytes needs a new
    texture and so an owner decision on changing the graphics PWAD (open).
35. **A masked midtexture column that is a multi-patch composite is refused** (orchestrator application of
    R3, 2026-11-16). The composite holds no post headers and no 0xff terminator, so the genuine
    `R_DrawMaskedColumn` walk (r_segs.c:181-184, r_things.c:359-385) runs into zone memory (item 34). The
    Rust renderer refuses any frame that would draw such a column with `RenderRefusal::MaskedCompositeColumn`
    (step 2b); a masked column whose post walk would leave its patch lump, or whose lump and offset are
    undefined (item 32), is refused with `MaskedColumnUndefined`. No committed room reaches either. The
    patchless-column comparison (item 32) stays open.
36. **Step 2b refusals that no synthetic frame reaches** (applications of R3 and R12, step 2b): a genuine
    `RANGECHECK` `I_Error` or a draw outside the view rectangle (`RangeCheck`), a table read outside the
    contiguous genuine tables such as `tantoangle[FixedDiv(0,0)>>DBITS]` for a viewer exactly on a seg's
    start vertex (`TableRead`, hazard 2 of §1.11 and item 3), and a top or bottom texture chosen on the
    unshifted heights whose `>>4` test fails, which would draw with an earlier wall's `pixhigh`/`pixlow`
    (`StaleGlobal`, hazard 8). Over the 39,585 synthetic frames none of them occurred; their reachability on
    the real maps is measured in step 4.
37. **Step 2c refusals that no synthetic frame reaches** (applications of R1 and R3, step 2c): a sprite
    whose `MF_TRANSLATION` bits are set reaches `R_DrawTranslatedColumn` (r_things.c:417-422), which the
    genuine runner aborts on; the Rust renderer refuses the frame at that call with
    `RenderRefusal::PlayerTranslation` (R1: players 2-4 only). The sprite `RANGECHECK`s (invalid sprite
    number or frame, r_things.c:506-516 and 659-669; a texture column outside the patch,
    r_things.c:432-436; `R_DrawFuzzColumn`, r_draw.c:307-314) refuse with `RangeCheck`, a sprite post walk
    leaving its lump with `MaskedColumnUndefined`, and an `R_SortVisSprites` pass finding no scale below
    `MAXINT` (the uninitialised `best`, r_things.c:815) with `StaleGlobal`. Over the 39,585 synthetic frames
    none of them occurred ([evidence](validation/M5A.md#rust-renderer-step-2c)).
38. **The rewind test uses a simulation clone** (owner ruling on R5-R7, step 2c). doom-sim has no API that
    restores a simulation from its canonical snapshot, and M5a does not edit doom-sim; `Simulation`
    implements `Clone`, so `tests/rewind.rs` takes a clone at tic N-k as the simulation's restore point
    and restores the render state through its `DRST` encoding into a fresh renderer. A restore from the
    canonical snapshot bytes would need a new doom-sim API (an owner decision; not needed by M5a).
39. **Step 4: the real maps reach none of the step 2b/2c refusals** (items 2, 3, 36 and 37). Over the
    158,966 frames per configuration of the 61 real-map runs at screenblocks 11 and 10, no frame was
    refused (`TableRead`, `StaleGlobal`, `RangeCheck`, the masked-column refusals, `PlayerTranslation`,
    `FixedDiv` or a limit), no genuine run ended at a renderer limit, no frame was non-comparable as a
    whole and no frame met an unknown switch parity
    ([evidence](validation/M5A.md#real-maps-step-4)). So the negative `finesine` index and the viewer on
    a vertex (items 2 and 3) were not met as refusals on these inputs; whether the contiguous-table read
    of r_main.c:484-485 occurred on a matching frame was not instrumented.

**Independent review findings (2026-11-16)**

The independent review of the M5a diff ([record](validation/M5A.md#independent-review)) found the
items below. Each entry gives the C facts first, then the disposition (applications of R3, R4 and R12
under the unattended rule: each ports a well-defined C behaviour or keeps a refusal where the C
behaviour is undefined, and none admits a silent divergence).

40. **`R_GenerateLookup` counts patches per column in a byte.** `byte* patchcount` (r_data.c:299) is
    `alloca`ed and cleared (r_data.c:322-323) and incremented per covering patch (`patchcount[x]++`,
    r_data.c:343), so it wraps modulo 256. A column covered by 256 patches reads as 0: "column without
    a patch" prints and the function returns (r_data.c:350-356), leaving that column and every later
    one with the lump and offset the first loop stored (the last covering patch's, r_data.c:344-345).
    A column covered by 257 patches reads as 1: a single-patch column of the last patch. **Ported:** a
    `u8` count with wrapping increments; a wrapped 0 takes the existing "stopped" path, and a column a
    patch covered keeps that patch's lump and offset.
41. **Load-time refusals where the genuine startup is well defined** (each checked against the C):
    - A texture with `patchcount` 0: `Z_Malloc(sizeof(texture_t) + sizeof(texpatch_t)*(0-1))` is
      exactly the 16-byte header before `patches[]` (r_data.c:524-527), the patch loop does not run
      and `R_GenerateLookup` stops at column 0 (r_data.c:350-356). **Ported** (its columns are the
      undefined columns of item 32). A negative `patchcount` allocates less than the fields
      `R_InitTextures` writes (r_data.c:524-531: a heap overflow): **refusal kept**.
    - A texture with width 0: `alloca(0)`, `memset(.., 0)` and two 0-byte `Z_Malloc` column tables
      (r_data.c:322-323, 548-549); the width mask is 0 (r_data.c:551-555). **Ported** at load. Every
      `R_GetColumn` of it reads `texturecolumnlump[tex][0]` past its 0-byte block (r_data.c:391-392),
      so a frame that draws one of its columns is refused (`RenderRefusal::UnallocatedColumn`). A
      negative width is an `alloca`/`memset` of a negative size (r_data.c:322-323): **refusal kept**.
    - A texture with height 0 or a negative height: `textureheight = height<<FRACBITS`
      (r_data.c:556) as the genuine (GCC, two's complement) build computes it, like every other
      negative 16.16 conversion the renderer already reads (vertices, seg offsets). Height 0 adds
      nothing to `texturecompositesize` (r_data.c:371) and is **ported**. A negative height makes
      the composite size negative (r_data.c:365-371), and `R_GenerateComposite` would `Z_Malloc` it
      (r_data.c:244) the first time a composite column is read (r_data.c:394-398): the load is
      **ported**, and drawing one of its composite columns is refused (`UnallocatedColumn`);
      single-patch columns draw as usual.
    - No flat or no sprite lump between the markers: `numflats` and `numspritelumps` are 0
      (r_data.c:585-590, 608-614) and the tables are `Z_Malloc(4)` and `Z_Malloc(0)`. **Ported.**
      Markers in the wrong order give a negative count that sizes a `Z_Malloc` with a negative byte
      count (z_zone.c:195-203 adds the block header to a negative size and splits the zone at it):
      **refusal kept**. With no flat, `R_FlatNumForName(SKYFLATNAME)` returns an out-of-range number
      unchecked (r_data.c:672-687); `skyflatnum` is only ever compared with a sector's pic
      (r_bsp.c:526, r_plane.c:225 and 396, r_segs.c:530-531 and 672), so an out-of-range sky flat is
      **ported** as a number no in-range pic equals (a sector pic outside the range is still refused:
      its `flattranslation` read, r_plane.c:423-425, is out of bounds).
    - A two-sided line with no back side and no seg: `P_LoadLineDefs` sets `backsector = 0`
      (p_setup.c:425-428) and only `P_LoadSegs` reads `sides[ldef->sidenum[side^1]]`
      (p_setup.c:189-190). **Ported:** `Level::load` no longer refuses the line; a seg of such a line
      reads `sides[-1]` and is still refused. In practice doom-map's parser (outside M5a's scope,
      crates/doom-map/src/parse.rs:252) refuses every two-sided line without a back side first, so
      such a map never reaches the renderer; recorded for the owner, not acted on.
    - A negative TEXTURE1 or TEXTURE2 count: `numtextures = numtextures1 + numtextures2`
      (r_data.c:480) sizes seven `Z_Malloc`s (r_data.c:482-488), and the directory switches to
      TEXTURE2 only at `i == numtextures1` (r_data.c:509-515). A nonnegative total is **ported**:
      with a negative `numtextures1` every entry is read from TEXTURE1's directory, and with a
      negative `numtextures2` the first `numtextures` entries of TEXTURE1 are read (a directory entry
      outside its lump stays refused, as before). A negative total (a negative allocation) or an
      overflowing sum (signed overflow) is **refused**.
42. **The stale `pixhigh`/`pixlow` is deterministic.** `pixhigh`, `pixlow`, `pixhighstep` and
    `pixlowstep` are file-scope globals (r_segs.c:81-84), written only for a two-sided seg whose
    shifted heights differ (r_segs.c:693-704) and advanced per column while a top or bottom texture is
    drawn (r_segs.c:297-298, 325-326). The textures themselves are chosen on the unshifted heights
    (r_segs.c:570-573, 589-592), so a seg whose shifted test fails draws with the values the previous
    such seg left, in the same frame or any earlier one; nothing resets them (zero in a fresh process,
    decision R8). **Ported:** the four values persist in the render state (`WallSteps`), which makes
    the `DRST` encoding version 2 (version 1 never left this branch; docs/STATE_FORMAT.md); the
    `StaleGlobal("pixhigh"/"pixlow")` refusals are gone. `StaleGlobal` remains only for
    `R_SortVisSprites`' uninitialised local `best` (r_things.c:792, 815), which is undefined.
43. **`MaskedCompositeColumn` is not "never terminates".** `R_RenderMaskedSegRange` passes
    `R_GetColumn(texnum, col) - 3` as a post list (r_segs.c:181-184), so for a composite column the walk
    reads the previous composite column's texels (or, for the first composite column, the zone memory
    before the block) as a post header and continues through texels until one reads 0xff
    (r_things.c:359-385). The outcome depends on the texel values and, once the walk leaves the
    composite, on the memory layout. **Declared refusal for the owner:** the renderer keeps refusing
    every such frame rather than following the walk through the composite (a port would be bounded
    while the walk stays inside defined composite bytes, and would still refuse once it leaves them).
44. **`FINEANGLES/20*leveltime` overflows `int`** (p_user.c:108) from `leveltime` 5,250,572 on (409 x
    5,250,572 > `INT_MAX`; about 41.7 hours), on the tics `P_CalcHeight` finds the player on the
    ground; `leveltime++` (p_tick.c:157) itself overflows at `INT_MAX`. Both are signed overflow
    (undefined) in this simulation code, which the simulation's checked-arithmetic policy refuses.
    **Refused:** the view shadow returns `ShadowError::LevelTimeOverflow` at that tic instead of
    panicking in a debug build.
45. **The sky depends on the episode.** `G_InitNew` sets `skytexture` to `SKY1`, `SKY2`, `SKY3` or
    `SKY4` by episode outside commercial mode (g_game.c:1464-1478). **Refused:** `Level::load` takes the
    game's episode and refuses any episode other than 1 (`ResourceError::SkyEpisode`), so an Episode 2
    level can no longer be drawn with `SKY1`.
46. **Unavailable player powers are refused.** The renderer read `player->powers` through
    `original_player_powers` and assumed zero when the profile has none. **Refused:** the view shadow
    returns `ShadowError::NoPlayer` and the frame `RenderRefusal::NoPlayer`.
47. **A restored render state is checked against the level** (Rust-only safety; no C counterpart). A
    `DRST` state decoded from another level, or built by hand, could make `render` index the mapped
    set out of bounds (r_segs.rs) or underflow a power in the view shadow's countdown. **Fixed:**
    `Renderer::restore` returns `RestoreError` unless the mapped set and sector floors have the level's
    counts, every running button and unknown-parity line is a line of the level, every cleared line is
    one of its tracked switch lines, the framebuffer and taint cover the screen and every bounded value
    is in range; the decoder also refuses negative powers (the simulation keeps them nonnegative,
    p_user.c:339-353).
48. **The harness checks every item's goldens for gaps.** The comparison loop stopped at the first tic
    without a genuine row, so later rows were neither compared nor counted (only the global frame total
    caught it). **Fixed:** every item and configuration must have one row per tic from 0 and the frame
    count of `m5a_results.json`, every row is compared or counted as not reached, and a tic without a row
    while later rows exist is counted as missing (`tests/frames.rs`; `m5a_real` exits 1 on either).
49. **Three source facts behind surviving mutants** (independent review of the mutation dispositions,
    2026-11-17; [evidence](validation/M5A.md#independent-review-of-the-dispositions-2026-11-17)). (a)
    Nothing checks a seg's stored angle (p_setup.c:181) against its vertices, which `R_AddLine` culls on
    (r_bsp.c:267-277), so a front-facing seg can reach `rw_normalangle - rw_angle1 == ANG180`, where the
    strict test of r_segs.c:632-633 keeps `rw_offset`'s sign. (b) `P_TryMove` sets `floorz` (p_map.c:492)
    in `P_XYMovement` before `P_ZMovement` runs in the same `P_MobjThinker` (p_mobj.c:416-431), so the
    player can enter `P_ZMovement` with `z == floorz` and `momz != 0`; the smooth-step test `z < floorz`
    (p_mobj.c:252) is then false. (c) The `&127` wrap of a slightly negative frac reads texel 127 of a post
    (r_draw.c:142), beyond the end of an unpadded lump in its last column; the committed graphics pad
    every patch and sprite except the tracker's short patches, IWAD lumps are not padded, and such reads
    are undefined (decision R4). The Rust renderer already did all three; tests and the rooms LANDING,
    SEGANGLE and MASKPAD now pin them (no code changed).

Declared refusals, **accepted by the owner 2026-11-16**: `MaskedCompositeColumn` (item 43;
also items 34-35) and the whole-run refusals kept where the C startup is undefined (item 41: negative
width or patch count, markers in the wrong order, a negative or overflowing texture total). The other
refusals of items 41-47 follow from undefined C behaviour (signed overflow, reads outside an
allocation) or from the Episode 1 scope.
