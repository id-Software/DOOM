/*
 * M5b headless genuine-engine runner: the driver (declared adapter).
 *
 * This is the M5b fork of tools/reference/m5a_driver.c, which stays byte-for-byte as merged
 * (frozen M5a evidence). tools/reference/m5b_build.py builds it with the M5a runner's genuine
 * sources, adapters and flags plus the genuine status bar and HUD compiled whole (st_stuff.c,
 * st_lib.c, hu_stuff.c, hu_lib.c, and m_cheat.c, which ST_Responder references) and the stubs of
 * tools/reference/m5b_stubs.c, into the contract tests/reference/m5b_runner_contract.json
 * (schema DOOM-M5B-RUNNER-v1; decisions S1, S14-S17, S22 and S23 of
 * docs/M5B_REFERENCE_INVESTIGATION.md). The fork changes the M5a driver text in exactly these ways
 * (the block "M5b: the status bar, the HUD, the palette and the D_Display mirror" holds most of it):
 *   1. Init: HU_Init and ST_Init after the D_CheckNetGame values (d_main.c:1110, 1113); ST_Start
 *      and HU_Start run inside the genuine P_SpawnPlayer for the console player (p_mobj.c:693-699);
 *      ST_Ticker and HU_Ticker run inside the genuine G_Ticker after P_Ticker (g_game.c:729-734).
 *   2. Display: with --render every, a mirror of D_Display (d_main.c:191-355) for GS_LEVEL without
 *      menu, automap, pause or wipe after the dump of every dumped tic: the first display runs
 *      R_ExecuteSetViewSize (setsizeneeded from R_Init, d_main.c:215-220; M5a called it after the
 *      load instead), then HU_Erase, ST_Drawer (viewheight == 200, redrawsbar) before
 *      R_RenderPlayerView, HU_Drawer after it, the R_FillBackScreen and border steps (which
 *      return at once at screenblocks 10 and 11) and the state copies. One display per tic, as
 *      vanilla under -timedemo (singletics, d_main.c:375-395; finding FH2). The frame is the whole
 *      320x200 screens[0] after the display; the palette is the PLAYPAL index I_SetPalette
 *      received (st_stuff.c:1045-1050), recorded by the I_SetPalette stub's hook.
 *   3. The M_Random contract (decision S15): sound-free and wipe-free. s_sound.c and f_wipe.c are
 *      not compiled, so the index is 0 at a new game (M_ClearRandom from G_InitNew, g_game.c:1414)
 *      and only ST_Ticker's st_randomnumber draw (st_stuff.c:992) advances it. Frames are identical
 *      to the genuine engine under this declared sound-free, wipe-free contract; the idle face's
 *      glance differs from a vanilla session with sound (section 2.5, FH1).
 *   4. Record and mask (decision S16), in this runner only: the H row's rndindex is printed without
 *      the presentation draws ((rndindex - draws) & 0xff, the value every frozen runner prints), and
 *      the P row's message is the pointer the genuine HU_Ticker found before it cleared it
 *      (hu_stuff.c:519-530; the M4a stub's meaning). The genuine rndindex of every frame (P rows of
 *      the frame stream) and every message HU_Ticker took (X rows, with its code and text) are
 *      recorded in the frame stream and compared separately. --no-mask-rnd and --no-hu-record
 *      (diagnostic, used by the self-test) print the genuine values.
 *   5. Frame stream (schema DOOM-M5B-FRAMES-v1): the M5a F row, now the digest of the whole frame
 *      after the display, and per frame a P row (tic, palette, palette set during the frame or -1,
 *      face index, message on, genuine rndindex), X rows (messages taken), a D row (diagnostic: the
 *      M_Random draws, the 2D tracker counts and the digest with the rows below the view zeroed,
 *      for the M5a comparison), W rows (per widget: calls, first, min and max value, changes) and a
 *      Z row of totals at the end.
 *   6. The read-provenance tracker is extended to the 2D drawers (V_DrawPatch, V_DrawPatchDirect,
 *      V_CopyRect): a post byte outside its lump, a write below the screen or below row 31 of
 *      screens[4], a skipped patch, and a pixel copied from a screens[4] byte no patch wrote are
 *      counted in the D row (decision S17, finding FH5).
 *   7. S17: the face reads plyr->attacker->x,y (st_stuff.c:799-813) and the pointer is never cleared
 *      when its mobj is removed. Before the genuine ST_Ticker the driver checks that an attacker the
 *      face could read is still in the thinker list; if not, it writes the witness AttackerRemoved (an
 *      O row) and ends the run before that tic is dumped (end=refused).
 *   8. The command source is behind command_source_begin and command_before_tic: a version-110
 *      demo through the genuine -playdemo path, the only command source of DOOM-M5B-RUNNER-v1.
 *   9. --bespoke-keys TIC TEXT (the bespoke scenario of decision S1 only): key events through the
 *      genuine G_Responder before G_Ticker of tic TIC (the cheats iddqd and idbeholds).
 *  10. Wrapped symbols beyond M5a's: HU_Ticker, ST_Ticker, M_Random, M_ClearRandom,
 *      STlib_updateMultIcon, STlib_updateNum, STlib_updatePercent, STlib_updateBinIcon,
 *      HUlib_drawSText, V_DrawPatch, V_DrawPatchDirect, V_CopyRect, W_CacheLumpName,
 *      W_GetNumForName; each records and calls the genuine function (every call is cross-object, so
 *      the linker redirects all of them; V_DrawPatchDirect's own call to V_DrawPatch, v_video.c:343,
 *      stays inside v_video.c, which is why both are wrapped).
 *  11. Test and diagnostic options: --selftest-bar TIC X Y (flips a pixel after the display),
 *      --selftest-palette TIC, --selftest-rnd TIC (record the value with its low bit flipped),
 *      --selftest-message TIC (records a message taken at TIC one tic late), --diag-fill N (fills
 *      screens[0] after V_Init) and --lump-trace FILE. None touches genuine code text.
 * Everything below the M5b block is the M5a driver text, changed only where the list above says.
 *
 * M5a headless genuine-engine runner: the driver (declared adapter).
 *
 * This is the M5a fork of tools/reference/m4a_driver.c, which stays byte-for-byte as
 * merged (frozen M4a/M4b evidence). tools/reference/m5a_build.py builds this file with the
 * M4b runner's genuine sources, adapters and flags plus the whole genuine renderer (r_main.c,
 * r_bsp.c, r_segs.c, r_plane.c, r_things.c, r_draw.c, v_video.c) and the stubs of
 * tools/reference/m5a_stubs.c into the contract tests/reference/m5a_runner_contract.json
 * (schema DOOM-M5A-RUNNER-v1; decisions R1-R16 of docs/M5A_REFERENCE_INVESTIGATION.md). The
 * fork changes the M4a driver text in exactly these ways:
 *   1. Init (R8, section 3.2): V_Init (d_main.c:1012) before Z_Init; R_Init (d_main.c:1095)
 *      instead of the separate R_InitTextures/R_InitFlats calls; after P_InitSwitchList and
 *      P_InitPicAnims, R_InitSprites (p_setup.c:704) on a 139-entry copy of sprnames with an
 *      explicit NULL terminator (R11: info.c:40-55 has exactly NUMSPRITES entries while
 *      R_InitSpriteDefs counts to NULL, r_things.c:191; equivalent to vanilla P_Init when the
 *      word after sprnames is NULL); after the load, R_ExecuteSetViewSize () as the first
 *      D_Display does when setsizeneeded is set (d_main.c:215-217).
 *   2. Rendering (R8): with --render every, R_RenderPlayerView (&players[displayplayer])
 *      after the dump of every dumped tic and before the stop checks (D_Display renders when
 *      gamestate == GS_LEVEL && !automapactive && gametic, d_main.c:267-268); the load
 *      (gametic 0) is never rendered, the end=exit and end=reborn tics are; screens[0] is
 *      never cleared, nothing else is drawn (no status bar, HUD, border, menu or palette).
 *   3. The L.flags dump column has ML_MAPPED (256, doomdata.h:135) cleared (R5): the one game
 *      field the renderer writes (R_StoreWallRange, r_segs.c:398; read only by the automap,
 *      am_map.c:1128, and savegames, p_saveg.c:141). The newly mapped lines of each frame are
 *      written to the frame stream instead. --no-mask-mapped (diagnostic) prints it unmasked.
 *   4. The frame stream (--frames FILE, R4, R5, R8): per rendered frame the SHA-256 of the
 *      64,000 bytes of screens[0] (computed here), fuzzpos, the renderer limit counters, the
 *      newly mapped lines, and the read-provenance tracker's undefined columns (see the block
 *      above the drawer wrappers). --frames-raw DIR writes frame_NNNNN.raw (row-major 320x200
 *      palette indices) for every rendered frame.
 *   5. The view record (--view FILE, R6): per dumped tic the view the renderer uses
 *      (DOOM-M5A-VIEW-v1, separate from the game dump; see write_view).
 *   6. Linker-level observers for the renderer (-Wl,--wrap; each records and calls the genuine
 *      function): R_DrawColumn, R_DrawFuzzColumn, R_DrawSpan (the tracker), R_GetColumn and
 *      W_CacheLumpNum (source blocks), R_StoreWallRange and R_CheckPlane (limit witnesses);
 *      R_DrawColumnLow, R_DrawSpanLow and R_DrawTranslatedColumn abort (low detail is refused,
 *      R2; player colour translation is unreachable in single player and refused, R1).
 *   7. --detail 1 is refused (R2): the driver exits with status 8 before loading anything.
 *   8. In builds without the table adapters (strict, strict32) the driver checks at start that
 *      finesine immediately follows finetangent (tables.c:67, 584), the layout the genuine
 *      out-of-range reads of r_main.c:484-485 and r_segs.c:266 rely on, and refuses to run
 *      (status 9) otherwise.
 *   9. A limit witness that writes past a renderer array (visplanes in R_CheckPlane,
 *      r_plane.c:313-317; openings, r_segs.c; solidsegs in R_ClipSolidWallSegment,
 *      r_bsp.c:122-133) ends the run after that frame with end=refused (R3); I_Error during
 *      a frame is written to the frame stream (E row) before the stub aborts.
 * Everything else below is the M4a driver text unchanged, including its four additions to the
 * M3h9c driver (H.secretexit, P.message codes 38..40, the P_RemoveThinker observer and the R
 * rows' fn,sector).
 *
 * Mirrors the vanilla -playdemo path (d_main.c:948-958, 1018-1141; d_net.c:742-746)
 * without video, sound, menus or network:
 *   Z_Init (d_main.c:1018); W_InitMultipleFiles over the --wad files, added as D_AddFile
 *   does (d_main.c:543-556, 1021): the IWAD, then the .lmp, which W_AddFile's single-lump
 *   branch names after its upper-cased base name (w_wad.c:172-180, 86-114);
 *   gamemode = registered (the doom.wad branch of IdentifyVersion, d_main.c:697-701);
 *   R_InitTextures and R_InitFlats (the first two calls of R_InitData, r_data.c:654-658,
 *   with the declared adapters; sprites and colormaps are not loaded);
 *   P_InitSwitchList and P_InitPicAnims (P_Init, p_setup.c:700-705, without R_InitSprites);
 *   the D_CheckNetGame single-player values (d_net.c:567-592); singledemo = true and
 *   G_DeferedPlayDemo (d_main.c:1135-1138); G_DoPlayDemo () called directly (the
 *   ga_playdemo case of G_Ticker, g_game.c:617-652), so the loaded level is dumped as
 *   "load" before tic 0: the first G_Ticker then finds gameaction == ga_nothing, the
 *   state vanilla reaches inside that G_Ticker after its gameaction loop; then
 *   { G_Ticker (); gametic++; } as TryRunTics does (d_net.c:742-746), with a dump after
 *   every tic.
 * The demo header selects skill, episode and map (G_DoPlayDemo, g_game.c:1596-1598), so
 * the driver has no map argument.
 *
 * Declared driver behaviour beyond vanilla:
 *   - End of the demo: G_CheckDemoStatus calls I_Quit at the marker (g_game.c:1660-1661),
 *     which in vanilla exits the process (i_system.c:116-124; D_DoomLoop never returns,
 *     d_main.c:1140). The stub I_Quit sets stub_quit_flag and longjmps back to the setjmp
 *     around G_Ticker below, so, as with exit, no later statement of G_CheckDemoStatus
 *     (Z_ChangeTag, the global resets) or of G_Ticker (P_Ticker) runs.
 *   - End on the exit: G_Ticker runs its gameaction loop (g_game.c:617-652) before it reads
 *     the tic's ticcmd (g_game.c:656-669), and G_ExitLevel / G_SecretExitLevel set
 *     gameaction = ga_completed during P_Ticker (g_game.c:1002-1018, from the exit
 *     switch or line, p_switch.c:359-363). So the G_Ticker after an exit always runs
 *     G_DoCompleted (g_game.c:636-637, 1020-1141), which enters the intermission
 *     (gamestate = GS_INTERMISSION and WI_Start, g_game.c:1133-1140), presentation the
 *     runner does not model, and the demo marker is never reached. After dumping each
 *     tic the driver therefore stops if gameaction is ga_completed, ga_victory or
 *     ga_worlddone and reports end=exit with that tic; a demo that reaches its marker
 *     reports end=marker. The WI_Start stub still aborts as a safety net.
 *   - End on a reborn request: P_DeathThink sets playerstate = PST_REBORN when a dead
 *     player presses BT_USE (p_user.c:227-228). The next G_Ticker would start with its
 *     reborn loop (g_game.c:612-614), whose G_DoReborn sets gameaction = ga_loadlevel in
 *     single player (g_game.c:924-931), and its gameaction loop would reload the level
 *     (G_DoLoadLevel, g_game.c:621-623): outside the admitted domain (decision G11; Rust
 *     refuses that tic with PlayerReborn). After dumping each tic the driver therefore
 *     also stops if any player in the game is PST_REBORN and reports end=reborn with
 *     that tic.
 *   - Linker-level observers (see the block above the wrappers below): P_AddThinker,
 *     Z_Free, P_SetThingPosition and P_InitThinkers are wrapped with -Wl,--wrap; each
 *     wrapper records thinker serials, link serials, frees and removed-mobj copies in
 *     side tables and calls the genuine function unchanged. They add the T, K and RM rows.
 *   - K rows carry `adds`, the number of P_AddThinker calls made before the free (the
 *     next thinker serial at that moment), which places each free among the tic's spawns.
 *   - A pending removal (function -1, still in the list) keeps its R row; for a mobj an
 *     RR row follows with the M columns (idx = list position; bpos/spos -1, unlinked by
 *     P_RemoveMobj), so a run's last snapshot shows every pending mobj's fields.
 *   - The HU_Ticker stub records players[consoleplayer].message before clearing it as
 *     vanilla does; P rows report it as a pickup-message index.
 *   - ZeroFill: Z_Malloc is also wrapped (-Wl,--wrap=Z_Malloc) and every block it returns
 *     is zero-filled after the genuine allocation. Finding (real-E1M1 barrels run): the
 *     strict and address dumps differed only in plat_t.count and floormove_t.newspecial,
 *     zone memory the genuine code never initialises before the dump reads it. EV_DoPlat
 *     allocates the plat (p_plats.c:173) without setting count; T_PlatRaise reads count
 *     only in the waiting state (p_plats.c:120) after setting it on every transition
 *     into waiting (75, 84, 113). EV_DoFloor allocates the floor (p_floor.c:282) and
 *     sets newspecial only for lowerAndChange when a neighbour with the destination
 *     height exists (p_floor.c:422, 433) and donutRaise sets it (p_spec.c:1203); T_MoveFloor
 *     reads it only for those two types (p_floor.c:231, 242). Vanilla demos play back
 *     identically across machines, so gameplay never depends on uninitialised zone
 *     contents; the fill fixes only values undefined in the original and keeps the
 *     three-variant identity check meaningful. One genuine path does read undefined
 *     memory: lowerAndChange with no neighbour at the destination height reads
 *     newspecial and texture uninitialised (p_floor.c:242-243); the fill makes them 0
 *     there (not reachable on E1M1, which has no lowerAndChange line).
 *   - After every tic the driver calls the genuine Z_CheckHeap (z_zone.c:399-420), a
 *     read-only walk of the zone block list that calls I_Error on corruption; vanilla
 *     calls it only in G_DoLoadLevel (g_game.c:488). --no-checkheap disables it.
 *   - --selftest-extra-draw TIC (test only) calls the public P_Random () once before
 *     G_Ticker of tic TIC; it never touches gameplay code text.
 *   - --zone-mb N sets the zone size (vanilla mb_used = 6, i_system.c:51).
 *
 * Columns added for M4a (appended at the end of their rows, so every earlier column keeps
 * its position and name; readers select columns by name):
 *   - H.secretexit: g_game.c's `boolean secretexit` (g_game.c:999), set by
 *     G_SecretExitLevel (g_game.c:1009-1018, line special 51, p_switch.c:431-435) and
 *     cleared by G_ExitLevel (g_game.c:1002-1006); it is what tells a secret exit
 *     (end=exit, secretexit 1) from a normal one (secretexit 0) on the stopping tic.
 *   - P.message codes 38..40: the three locked-door strings PD_BLUEK, PD_YELLOWK and
 *     PD_REDK (d_englsh.h:128-130) that EV_VerticalDoor assigns to player->message
 *     (p_doors.c:378, 392, 405), appended after GOTSHOTGUN2 so codes 1..37 are unchanged.
 * Rows that already carried what M4a reads (no change): S.floorpic (sector_t.floorpic,
 * written by EV_DoPlat raiseToNearestAndChange p_plats.c:185-195 and by the donut's
 * T_MoveFloor pastdest p_floor.c:230-232), S.lightlevel (EV_LightTurnOn p_lights.c:271-306),
 * P.pw0..pw5, card0..card5, backpack, ammo0..3, maxammo0..3 (player_t, d_player.h), and
 * M.flags (MF_SHADOW, p_inter.c:300-305; MF_JUSTATTACKED, p_pspr.c:542).
 *
 * Dumps are written only to --out: they hold map-derived values and stay local.
 * tools/reference/m3h9c_dump.py documents and reads the format.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>
#include <time.h>
#include "doomdef.h"
#include "doomstat.h"
#include "z_zone.h"
#include "w_wad.h"
#include "g_game.h"
#include "m_random.h"
#include "p_local.h"
#include "p_spec.h"
#include "r_state.h"
#include "dstrings.h"
#include "st_stuff.h"
#include "st_lib.h"
#include "hu_stuff.h"
#undef BG                                /* st_lib.h and hu_lib.h both define BG and FG; the driver uses neither */
#undef FG
#include "hu_lib.h"

/* genuine definitions without header declarations */
void R_InitTextures (void);
void R_InitFlats (void);
void T_FireFlicker (fireflicker_t* flick);   /* p_lights.c:46 */
extern short numlinespecials;
extern boolean singledemo;
extern int rndindex, prndindex;
extern gameaction_t gameaction;
extern boolean secretexit;               /* g_game.c:999 */
extern thinker_t thinkercap;
extern button_t buttonlist[MAXBUTTONS];
extern plat_t* activeplats[MAXPLATS];
extern ceiling_t* activeceilings[MAXCEILINGS];
extern int bmapwidth, bmapheight;
extern fixed_t bmaporgx, bmaporgy;
extern mobj_t** blocklinks;
/* layout_check.c (generated from the adapted r_data.c text) */
int layout_patchcount_offset (void);
int layout_patches_offset (void);
int layout_mappatch_size (void);
/* stubs.c */
extern int stub_quit_flag, stub_quit_armed, stub_zone_mb;
extern jmp_buf stub_quit_jmp;
char* stub_hu_message;   /* M5b: players[consoleplayer].message as the genuine HU_Ticker found it (its wrapper) */
int stub_hu_ran;

/* ---------------- M5a: the renderer ----------------
 * Declarations of genuine renderer definitions the driver reads (r_*.c, v_video.c, tables.c,
 * w_wad.c); m5a_types.h is generated by m5a_build.py from the adapted sources: the verbatim
 * texpatch_t and texture_t typedefs of r_data.c and cliprange_t of r_bsp.c. */
#include <stdint.h>
#include "r_main.h"
#include "r_draw.h"
#include "r_things.h"
#include "r_plane.h"
#include "r_bsp.h"
#include "v_video.h"
#include "info.h"
#include "tables.h"
#include "m5a_types.h"
void R_ExecuteSetViewSize (void);        /* r_main.c:671 */
extern int screenblocks, detailLevel;    /* m_menu.c:83-84 (stubs; defaults m_misc.c:279-280) */
extern int fuzzpos;                      /* r_draw.c:274 */
extern int fuzzoffset[];                 /* r_draw.c:263 */
#define M5A_FUZZTABLE 50                 /* r_draw.c:259 FUZZTABLE */
extern byte* ylookup[];                  /* r_draw.c:69 */
extern int columnofs[];                  /* r_draw.c:70 */
extern int numtextures, numflats;        /* r_data.c */
extern texture_t** textures;
extern int* texturewidthmask;
extern int* texturecompositesize;
extern short** texturecolumnlump;
extern unsigned short** texturecolumnofs;
extern byte** texturecomposite;
#define M5A_MAXVISPLANES 128             /* r_plane.c:52 MAXVISPLANES */
#define M5A_MAXOPENINGS (SCREENWIDTH*64) /* r_plane.c:59 MAXOPENINGS */
#define M5A_MAXSEGS 32                   /* r_bsp.c:88 MAXSEGS */
extern visplane_t visplanes[];           /* r_plane.c:53 */
extern visplane_t* lastvisplane;         /* r_plane.c:54 */
extern short openings[];                 /* r_plane.c:60 */
extern cliprange_t solidsegs[];          /* r_bsp.c:92 */
extern cliprange_t* newend;              /* r_bsp.c:91 */
extern vissprite_t overflowsprite;       /* r_things.c:326 */
extern boolean onground;                 /* p_user.c:51 */
extern void (*stub_error_hook) (const char* message);   /* m5a_stubs.c: called by I_Error before it aborts */
extern void (*stub_palette_hook) (byte* palette);       /* m5b_stubs.c: called by I_SetPalette */

/* SHA-256 (FIPS 180-4), written for this driver; digests frames only. */
typedef struct { unsigned int h[8]; unsigned char buf[64]; unsigned long long len; int n; } sha_t;
static const unsigned int SHA_K[64] = {
 0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,
 0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
 0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,
 0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
 0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,
 0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2 };
#define SHA_ROR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))
static void sha_block (sha_t* c, const unsigned char* p)
{
    unsigned int w[64], a, b, d, e, f, g, h, cc, t1, t2; int i;
    for (i = 0; i < 16; i++) w[i] = (unsigned)p[4*i] << 24 | (unsigned)p[4*i+1] << 16 | (unsigned)p[4*i+2] << 8 | p[4*i+3];
    for (i = 16; i < 64; i++)
        w[i] = (SHA_ROR (w[i-2], 17) ^ SHA_ROR (w[i-2], 19) ^ (w[i-2] >> 10)) + w[i-7]
             + (SHA_ROR (w[i-15], 7) ^ SHA_ROR (w[i-15], 18) ^ (w[i-15] >> 3)) + w[i-16];
    a = c->h[0]; b = c->h[1]; cc = c->h[2]; d = c->h[3]; e = c->h[4]; f = c->h[5]; g = c->h[6]; h = c->h[7];
    for (i = 0; i < 64; i++) {
        t1 = h + (SHA_ROR (e, 6) ^ SHA_ROR (e, 11) ^ SHA_ROR (e, 25)) + ((e & f) ^ (~e & g)) + SHA_K[i] + w[i];
        t2 = (SHA_ROR (a, 2) ^ SHA_ROR (a, 13) ^ SHA_ROR (a, 22)) + ((a & b) ^ (a & cc) ^ (b & cc));
        h = g; g = f; f = e; e = d + t1; d = cc; cc = b; b = a; a = t1 + t2;
    }
    c->h[0] += a; c->h[1] += b; c->h[2] += cc; c->h[3] += d; c->h[4] += e; c->h[5] += f; c->h[6] += g; c->h[7] += h;
}
static void sha256_hex (const unsigned char* p, unsigned long n, char out[65])
{
    static const unsigned int H0[8] = { 0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19 };
    sha_t c; unsigned char tail[128]; unsigned long i, rest; int k, tl;
    memcpy (c.h, H0, sizeof H0);
    for (i = 0; i + 64 <= n; i += 64) sha_block (&c, p + i);
    rest = n - i; memset (tail, 0, sizeof tail); memcpy (tail, p + i, rest); tail[rest] = 0x80;
    tl = rest + 9 <= 64 ? 64 : 128;
    for (k = 0; k < 8; k++) tail[tl - 1 - k] = (unsigned char)(((unsigned long long)n * 8) >> (8 * k));
    sha_block (&c, tail); if (tl == 128) sha_block (&c, tail + 64);
    for (k = 0; k < 8; k++) sprintf (out + 8 * k, "%08x", c.h[k]);
}

/* ---------------- the read-provenance tracker (decision R4) ----------------
 * R_DrawColumn reads dc_source[(frac>>FRACBITS)&127] whatever the column's height
 * (r_draw.c:142), so it can read past the source block; the bytes there are not defined by
 * the WAD and the game state (zone memory, other blocks, block headers; composites also hold
 * rows no patch wrote, R_DrawColumnInCache r_data.c:184-216, which the ZeroFill wrapper makes
 * 0 here). The wrappers below run before the genuine drawers and only read genuine state:
 *   - the source block of a column is the composite of the last R_GetColumn texture (when
 *     that column is composite, texturecolumnlump <= 0, r_data.c:390-400) if dc_source lies in
 *     it, else the lump of the last W_CacheLumpNum call (the wall patch of R_GetColumn, or the
 *     sprite patch of R_DrawVisSprite, r_things.c:409) if dc_source lies in its cache block;
 *     otherwise the frame is non-comparable as a whole (W row);
 *   - each row's source index is recomputed as the genuine loop computes it (r_draw.c:132-146,
 *     modulo 2^32); a read is defined when it lies inside the lump, or inside the composite
 *     and on a byte some patch post wrote (a mask built by replicating R_GenerateLookup's
 *     composite column order, r_data.c:351-372, and R_DrawColumnInCache's clipping, from the
 *     lumps read with W_ReadLump, never through the zone);
 *   - every screen pixel carries a taint (the origin of its undefined read, 0 when defined):
 *     a column write sets it per row, a span write (R_DrawSpan reads a 64x64 flat with the
 *     index masked to 4095, r_draw.c:548-557; the lump must hold 4,096 bytes) clears it, and a
 *     fuzz write copies it from the pixel the genuine R_DrawFuzzColumn reads (fuzzoffset at the
 *     current fuzzpos, after the row clamps of r_draw.c:293-298). Taints persist with the
 *     never-cleared screen (R8).
 * After each frame every screen column holding a tainted pixel is reported (U rows, with the
 * origins: T:<texture> for a texture column, L:<lump> for a sprite patch) and the frame digest
 * is also given with those columns zeroed; only those columns are non-comparable (owner ruling
 * on R4). */
static unsigned short taint[SCREENWIDTH*SCREENHEIGHT];
static int tracking, cur_tic = -1;
static int last_lump = -1;                       /* last W_CacheLumpNum lump */
static int gc_tex = -1, gc_lump = -1, gc_composite;   /* last R_GetColumn texture, its column's lump */
static unsigned char** compmask;                 /* per texture: 1 = byte written by a patch post */
#define M5A_MAXORIGINS 8192
static char origin_name[M5A_MAXORIGINS][16];
static int norigins, origins_written;
static int* lump_origin; static int* tex_origin;
static int frame_whole, frame_solid_peak, frame_drawseg_drops, frame_storewalls;
static int witness;                              /* 0 none, 1 visplanes, 2 openings, 3 solidsegs, 4 attacker removed (M5b, S17) */
static const char* const WITNESS[] = { "-", "VisplaneOverflow", "OpeningOverflow", "SolidSegOverflow", "AttackerRemoved" };
static FILE* frames_out;
/* The first limit witness of a frame, written at once (the frame may still end in I_Error). */
static void set_witness (int kind, int solid)
{
    if (witness) return;
    witness = kind;
    if (frames_out) {
        fprintf (frames_out, "O,%d,%s,%d,%d,%d\n", cur_tic, WITNESS[kind], (int)(lastvisplane - visplanes),
                 (int)(lastopening - openings), solid);
        fflush (frames_out);
    }
}

static int new_origin (const char* kind, const char* name)
{
    if (norigins == M5A_MAXORIGINS) { fprintf (stderr, "tracker: origin table full\n"); abort (); }
    snprintf (origin_name[norigins], sizeof origin_name[0], "%s:%.8s", kind, name);
    return norigins++;
}
static int lump_origin_of (int lump)
{
    if (!lump_origin) { int i; lump_origin = malloc (numlumps * sizeof (int)); for (i = 0; i < numlumps; i++) lump_origin[i] = -1; }
    if (lump_origin[lump] < 0) lump_origin[lump] = new_origin ("L", lumpinfo[lump].name);
    return lump_origin[lump];
}
static int tex_origin_of (int tex)
{
    if (!tex_origin) { int i; tex_origin = malloc (numtextures * sizeof (int)); for (i = 0; i < numtextures; i++) tex_origin[i] = -1; }
    if (tex_origin[tex] < 0) { char n[9]; memcpy (n, textures[tex]->name, 8); n[8] = 0; tex_origin[tex] = new_origin ("T", n); }
    return tex_origin[tex];
}
static int unidentified_origin (void) { static int o = -1; if (o < 0) o = new_origin ("W", "unknown"); return o; }

/* The written-byte mask of texture tex's composite block (see the block comment). */
static const unsigned char* composite_mask (int tex)
{
    texture_t* t = textures[tex]; int size = texturecompositesize[tex], i, x, x1, x2;
    unsigned char* m;
    if (!compmask) compmask = calloc (numtextures, sizeof *compmask);
    if (compmask[tex]) return compmask[tex];
    m = compmask[tex] = calloc (size > 0 ? size : 1, 1);
    for (i = 0; i < t->patchcount; i++) {
        texpatch_t* tp = &t->patches[i];
        int len = W_LumpLength (tp->patch); unsigned char* lump = malloc (len);
        short pw;
        W_ReadLump (tp->patch, lump);
        memcpy (&pw, lump, 2);
        x1 = tp->originx; x2 = x1 + pw;
        for (x = x1 < 0 ? 0 : x1; x < (x2 > t->width ? t->width : x2); x++) {
            int cofs, base;
            if (texturecolumnlump[tex][x] >= 0) continue;      /* not a composite column (r_data.c:270-272) */
            base = texturecolumnofs[tex][x];
            memcpy (&cofs, lump + 8 + 4 * (x - x1), 4);
            while (lump[cofs] != 0xff) {                       /* R_DrawColumnInCache, r_data.c:195-215 */
                int count = lump[cofs + 1], pos = tp->originy + lump[cofs], k;
                if (pos < 0) { count += pos; pos = 0; }
                if (pos + count > t->height) count = t->height - pos;
                for (k = 0; k < count; k++) if (base + pos + k < size) m[base + pos + k] = 1;
                cofs += lump[cofs + 1] + 4;
            }
        }
        free (lump);
    }
    return m;
}
static int in_block (const byte* p, const byte* lo, long size) { return (uintptr_t)p >= (uintptr_t)lo && (uintptr_t)p < (uintptr_t)lo + (uintptr_t)size; }
static long screen_offset (int y, int x) { return (long)(ylookup[y] - screens[0]) + columnofs[x]; }

static void track_column (void)
{
    const byte* lo = NULL; long size = 0; const unsigned char* mask = NULL; int origin = -1, otex = -1, olump = -1, y; long off;
    unsigned frac, step;
    if (dc_yh < dc_yl || (unsigned)dc_x >= SCREENWIDTH || dc_yl < 0 || dc_yh >= SCREENHEIGHT) return;  /* genuine: nothing or I_Error */
    if (gc_tex >= 0 && gc_composite && texturecomposite[gc_tex]
        && in_block (dc_source, texturecomposite[gc_tex], texturecompositesize[gc_tex])) {
        lo = texturecomposite[gc_tex]; size = texturecompositesize[gc_tex]; mask = composite_mask (gc_tex);
        otex = gc_tex;
    } else if (last_lump >= 0 && lumpcache[last_lump] && in_block (dc_source, lumpcache[last_lump], lumpinfo[last_lump].size)) {
        lo = lumpcache[last_lump]; size = lumpinfo[last_lump].size;
        if (gc_tex >= 0 && !gc_composite && gc_lump == last_lump) otex = gc_tex; else olump = last_lump;
    } else {
        frame_whole = 1; origin = unidentified_origin ();
    }
    off = screen_offset (dc_yl, dc_x);
    frac = (unsigned)dc_texturemid + (unsigned)(dc_yl - centery) * (unsigned)dc_iscale; step = (unsigned)dc_iscale;
    for (y = dc_yl; y <= dc_yh; y++, off += SCREENWIDTH, frac += step) {
        const byte* p = dc_source + ((frac >> FRACBITS) & 127);
        int ok = lo && in_block (p, lo, size) && (!mask || mask[p - lo]);
        if (!ok && origin < 0) origin = otex >= 0 ? tex_origin_of (otex) : lump_origin_of (olump);
        taint[off] = ok ? 0 : (unsigned short)(origin + 1);
    }
}
void __real_R_DrawColumn (void);
void __wrap_R_DrawColumn (void) { if (tracking) track_column (); __real_R_DrawColumn (); }
void __real_R_DrawFuzzColumn (void);
void __wrap_R_DrawFuzzColumn (void)
{
    int yl = dc_yl, yh = dc_yh, pos = fuzzpos, y; long off;
    if (!yl) yl = 1;                                   /* r_draw.c:293-298 */
    if (yh == viewheight - 1) yh = viewheight - 2;
    if (tracking && yh >= yl && (unsigned)dc_x < SCREENWIDTH && yl >= 0 && yh < SCREENHEIGHT)
        for (y = yl, off = screen_offset (yl, dc_x); y <= yh; y++, off += SCREENWIDTH) {
            taint[off] = taint[off + fuzzoffset[pos]];
            if (++pos == M5A_FUZZTABLE) pos = 0;
        }
    __real_R_DrawFuzzColumn ();
}
void __real_R_DrawSpan (void);
void __wrap_R_DrawSpan (void)
{
    if (tracking && ds_x2 >= ds_x1 && ds_x1 >= 0 && ds_x2 < SCREENWIDTH && (unsigned)ds_y < SCREENHEIGHT) {
        unsigned short t = 0; int x; long off = screen_offset (ds_y, ds_x1);
        if (!(last_lump >= 0 && (byte*)lumpcache[last_lump] == ds_source && lumpinfo[last_lump].size >= 4096)) {
            if (last_lump >= 0 && (byte*)lumpcache[last_lump] == ds_source) t = (unsigned short)(lump_origin_of (last_lump) + 1);
            else { frame_whole = 1; t = (unsigned short)(unidentified_origin () + 1); }
        }
        for (x = ds_x1; x <= ds_x2; x++, off++) taint[off] = t;
    }
    __real_R_DrawSpan ();
}
static void refused_drawer (const char* name, const char* why)
{
    fprintf (stderr, "%s reached: %s\n", name, why);
    abort ();
}
void __wrap_R_DrawColumnLow (void) { refused_drawer ("R_DrawColumnLow", "low detail is refused (decision R2)"); }
void __wrap_R_DrawSpanLow (void) { refused_drawer ("R_DrawSpanLow", "low detail is refused (decision R2)"); }
void __wrap_R_DrawTranslatedColumn (void) { refused_drawer ("R_DrawTranslatedColumn", "player colour translation is refused (decision R1)"); }
byte* __real_R_GetColumn (int tex, int col);
byte* __wrap_R_GetColumn (int tex, int col)
{
    byte* r = __real_R_GetColumn (tex, col);
    int c = col & texturewidthmask[tex];
    gc_tex = tex; gc_composite = texturecolumnlump[tex][c] <= 0; gc_lump = gc_composite ? -1 : texturecolumnlump[tex][c];
    return r;
}
void* __real_W_CacheLumpNum (int lump, int tag);
void* __wrap_W_CacheLumpNum (int lump, int tag)
{
    void* r = __real_W_CacheLumpNum (lump, tag);
    last_lump = lump;
    return r;
}
/* Limit witnesses (R3). MAXDRAWSEGS: R_StoreWallRange returns at once when ds_p is at the end
 * (r_segs.c:386-387, a silent drop counted here). solidsegs: R_ClipSolidWallSegment calls
 * R_StoreWallRange before its insertion writes *newend and increments it (r_bsp.c:122-133), and
 * every path that later shrinks the list stores a fragment first, so newend - solidsegs
 * > MAXSEGS at any call or at the end of the frame is exactly a write past solidsegs[MAXSEGS].
 * Openings are written only inside R_StoreWallRange (r_segs.c), so lastopening past
 * openings[MAXOPENINGS] after a call is exactly a write past the array. visplanes:
 * R_CheckPlane's split writes *lastvisplane and increments it unchecked (r_plane.c:313-317),
 * so lastvisplane past visplanes[MAXVISPLANES] after a call is exactly a write past it;
 * R_FindPlane checks first and calls I_Error (r_plane.c:245-246). */
void __real_R_StoreWallRange (int start, int stop);
void __wrap_R_StoreWallRange (int start, int stop)
{
    int n = (int)(newend - solidsegs);
    if (n > frame_solid_peak) frame_solid_peak = n;
    if (n > M5A_MAXSEGS) set_witness (3, n);
    if (ds_p == &drawsegs[MAXDRAWSEGS]) frame_drawseg_drops++;
    frame_storewalls++;
    __real_R_StoreWallRange (start, stop);
    if (lastopening - openings > M5A_MAXOPENINGS) set_witness (2, frame_solid_peak);
}
visplane_t* __real_R_CheckPlane (visplane_t* pl, int start, int stop);
visplane_t* __wrap_R_CheckPlane (visplane_t* pl, int start, int stop)
{
    visplane_t* r = __real_R_CheckPlane (pl, start, stop);
    if (lastvisplane - visplanes > M5A_MAXVISPLANES) set_witness (1, frame_solid_peak);
    return r;
}

/* ---------------- linker-level observers ----------------
 * Linked with -Wl,--wrap=P_AddThinker,--wrap=Z_Free,--wrap=P_SetThingPosition,
 * --wrap=P_InitThinkers (m3h9c_build.py LINK_FLAGS): every call from another object
 * file reaches __wrap_X, which records into the side tables below (never game memory,
 * never the RNG) and then calls the genuine __real_X with the same arguments, returning
 * its result. The one wrapper that writes game memory is ZeroFill's __wrap_Z_Malloc
 * (declared in the header: it zero-fills each new block, nothing else). Every gameplay call of the four is cross-object (p_tick.c defines
 * P_AddThinker/P_InitThinkers, p_maputl.c P_SetThingPosition, z_zone.c Z_Free); the only
 * unwrapped calls are z_zone.c's internal Z_Free calls (Z_FreeTags z_zone.c:316 and the
 * purge in Z_Malloc), which never free a live thinker except Z_FreeTags (PU_LEVEL) in
 * P_SetupLevel (p_setup.c:618), immediately followed by P_InitThinkers (p_setup.c:622),
 * whose wrapper drops every table entry (K rows with seq -1).
 * Thinker lifecycle (genuine): P_AddThinker appends (p_tick.c:65-71); P_RemoveThinker
 * only sets function.acv = -1 (p_tick.c:80-84; P_RemoveMobj first unlinks the mobj with
 * P_UnsetThingPosition and stops its sound, p_mobj.c:546-570); the thinker stays in the
 * list until P_RunThinkers reaches it, unlinks it and calls Z_Free (p_tick.c:101-122). The
 * Z_Free wrapper copies a mobj (function -1, added as a P_MobjThinker: P_SpawnMobj sets
 * the function before P_AddThinker, p_mobj.c:529-531) before the genuine free. */
void __real_P_AddThinker (thinker_t* thinker);
void __real_Z_Free (void* ptr);
void __real_P_SetThingPosition (mobj_t* thing);
void __real_P_InitThinkers (void);
void* __real_Z_Malloc (int size, int tag, void* user);

/* ZeroFill (declared, see the header): the genuine allocation, then memset to 0. */
void* __wrap_Z_Malloc (int size, int tag, void* user)
{
    void* block = __real_Z_Malloc (size, tag, user);
    memset (block, 0, size);
    return block;
}

typedef struct { void* p; int serial, linkserial, mobj, rfn; long rsector; } obs_t;
#define OBS_CAP 65536                     /* power of two; far above any thinker count */
static obs_t obs[OBS_CAP];               /* open addressing; p == NULL empty, p == OBS_DEAD deleted */
static char obs_dead_marker;
#define OBS_DEAD ((void*)&obs_dead_marker)
static int obs_used, next_serial, next_linkserial;
typedef struct { int serial, seq, adds; } freed_t;
static freed_t* freed; static int nfreed, capfreed, freeseq;
typedef struct { int serial; mobj_t m; } removed_t;
static removed_t* removed; static int nremoved, capremoved;
static int unknown_messages, observer_misses;

static unsigned obs_hash (void* p) { unsigned long v = (unsigned long)p >> 3; return (unsigned)(v ^ (v >> 17)) & (OBS_CAP - 1); }
static obs_t* obs_find (void* p)
{
    unsigned h = obs_hash (p);
    while (obs[h].p) { if (obs[h].p == p) return &obs[h]; h = (h + 1) & (OBS_CAP - 1); }
    return NULL;
}
static void obs_rehash (void)
{
    static obs_t old[OBS_CAP];
    int i;
    memcpy (old, obs, sizeof obs); memset (obs, 0, sizeof obs); obs_used = 0;
    for (i = 0; i < OBS_CAP; i++)
        if (old[i].p && old[i].p != OBS_DEAD) {
            unsigned h = obs_hash (old[i].p);
            while (obs[h].p) h = (h + 1) & (OBS_CAP - 1);
            obs[h] = old[i]; obs_used++;
        }
    if (obs_used >= OBS_CAP / 2) { fprintf (stderr, "observer table full\n"); abort (); }
}
static obs_t* obs_insert (void* p)        /* p is not in the table */
{
    unsigned h;
    if (obs_used >= OBS_CAP / 2) obs_rehash ();   /* obs_used counts live and deleted slots */
    h = obs_hash (p);
    while (obs[h].p && obs[h].p != OBS_DEAD) h = (h + 1) & (OBS_CAP - 1);
    if (!obs[h].p) obs_used++;
    obs[h].p = p; obs[h].serial = -1; obs[h].linkserial = -1; obs[h].mobj = 0; obs[h].rfn = 0; obs[h].rsector = -1;
    return &obs[h];
}
static void note_freed (int serial, int seq, int adds)
{
    if (nfreed == capfreed) { capfreed = capfreed ? capfreed * 2 : 64; freed = realloc (freed, capfreed * sizeof *freed); }
    freed[nfreed].serial = serial; freed[nfreed].seq = seq; freed[nfreed].adds = adds; nfreed++;
}
void __wrap_P_AddThinker (thinker_t* thinker)
{
    obs_t* e = obs_find (thinker);
    if (!e) e = obs_insert (thinker);
    e->serial = next_serial++;
    e->mobj = thinker->function.acp1 == (actionf_p1)P_MobjThinker;
    __real_P_AddThinker (thinker);
}
void __wrap_P_SetThingPosition (mobj_t* thing)
{
    obs_t* e = obs_find (thing);
    if (!e) e = obs_insert (thing);      /* a thing positioned before P_AddThinker (P_SpawnMobj, p_mobj.c:517) */
    e->linkserial = next_linkserial++;
    __real_P_SetThingPosition (thing);
}
void __wrap_Z_Free (void* ptr)
{
    obs_t* e = ptr ? obs_find (ptr) : NULL;
    if (e) {
        if (e->mobj && ((thinker_t*)ptr)->function.acv == (actionf_v)(-1)) {
            if (nremoved == capremoved) { capremoved = capremoved ? capremoved * 2 : 16; removed = realloc (removed, capremoved * sizeof *removed); }
            removed[nremoved].serial = e->serial;
            memcpy (&removed[nremoved].m, ptr, sizeof (mobj_t));
            nremoved++;
        }
        note_freed (e->serial, freeseq++, next_serial);
        e->p = OBS_DEAD;
    }
    __real_Z_Free (ptr);
}
static int cmp_serial (const void* a, const void* b) { return ((const obs_t*)a)->serial - ((const obs_t*)b)->serial; }
void __wrap_P_InitThinkers (void)
{
    int i, n = 0; obs_t* live = malloc (sizeof (obs_t) * (obs_used ? obs_used : 1));
    for (i = 0; i < OBS_CAP; i++) if (obs[i].p && obs[i].p != OBS_DEAD && obs[i].serial >= 0) live[n++] = obs[i];
    qsort (live, n, sizeof *live, cmp_serial);
    for (i = 0; i < n; i++) note_freed (live[i].serial, -1, next_serial);
    free (live);
    memset (obs, 0, sizeof obs); obs_used = 0;
    __real_P_InitThinkers ();
}
static int serial_of (void* p) { obs_t* e = obs_find (p); if (!e || e->serial < 0) { observer_misses++; return -1; } return e->serial; }
static int linkserial_of (void* p) { obs_t* e = obs_find (p); return e ? e->linkserial : -1; }

/* d_englsh.h GOTARMOR .. GOTSHOTGUN2 in header order, then (M4a) the locked-door strings
 * PD_BLUEK, PD_YELLOWK, PD_REDK (d_englsh.h:128-130; p_doors.c:378, 392, 405); P rows
 * report 1 + the index. After the table (index 41): G_Ticker's turbo message, built as
 * g_game.c:675-678 builds it, sprintf ("%s is turbo!", player_names[0]) (player 0 is the
 * only player in game, and ((gametic>>5)&3) == i selects it). */
static const char* const PICKUP_MESSAGES[] = {
    GOTARMOR, GOTMEGA, GOTHTHBONUS, GOTARMBONUS, GOTSTIM, GOTMEDINEED, GOTMEDIKIT, GOTSUPER,
    GOTBLUECARD, GOTYELWCARD, GOTREDCARD, GOTBLUESKUL, GOTYELWSKUL, GOTREDSKULL,
    GOTINVUL, GOTBERSERK, GOTINVIS, GOTSUIT, GOTMAP, GOTVISOR, GOTMSPHERE,
    GOTCLIP, GOTCLIPBOX, GOTROCKET, GOTROCKBOX, GOTCELL, GOTCELLBOX, GOTSHELLS, GOTSHELLBOX, GOTBACKPACK,
    GOTBFG9000, GOTCHAINGUN, GOTCHAINSAW, GOTLAUNCHER, GOTPLASMA, GOTSHOTGUN, GOTSHOTGUN2,
    PD_BLUEK, PD_YELLOWK, PD_REDK };
#define NPICKUP_MESSAGES ((int)(sizeof PICKUP_MESSAGES / sizeof *PICKUP_MESSAGES))
static char turbo_message[80];
static int message_index (const char* m)
{
    unsigned i;
    if (!m) return 0;
    for (i = 0; i < sizeof PICKUP_MESSAGES / sizeof *PICKUP_MESSAGES; i++)
        if (m == PICKUP_MESSAGES[i] || !strcmp (m, PICKUP_MESSAGES[i])) return (int)i + 1;
    if (!turbo_message[0]) {
        extern char* player_names[4];
        sprintf (turbo_message, "%s is turbo!", player_names[0]);   /* g_game.c:677 */
    }
    if (!strcmp (m, turbo_message)) return NPICKUP_MESSAGES + 1;
    unknown_messages++;
    return -1;
}

#define MAXWADS 20                       /* d_main.c MAXWADFILES */
static char* wadfiles[MAXWADS + 1];
static int numwads;
static void add_file (char* file)        /* d_main.c:543-556 */
{
    char* newfile = malloc (strlen (file) + 1);
    strcpy (newfile, file);
    wadfiles[numwads++] = newfile;
}

/* ---------------- dump ---------------- */
static thinker_t** order;        /* thinker-list order */
static int numth, capth;
typedef struct { void* p; int i; } pidx_t;
static pidx_t* sorted;
static unsigned char* linesel;   /* lines with a nonzero special at load */

static int cmp_pidx (const void* a, const void* b)
{
    const char* x = ((const pidx_t*)a)->p; const char* y = ((const pidx_t*)b)->p;
    return x < y ? -1 : x > y;
}
static void walk_thinkers (void)
{
    thinker_t* t; int i;
    numth = 0;
    for (t = thinkercap.next; t != &thinkercap; t = t->next) {
        if (numth == capth) { capth = capth ? capth * 2 : 1024; order = realloc (order, capth * sizeof *order);
                              sorted = realloc (sorted, capth * sizeof *sorted); }
        order[numth++] = t;
    }
    for (i = 0; i < numth; i++) { sorted[i].p = order[i]; sorted[i].i = i; }
    qsort (sorted, numth, sizeof *sorted, cmp_pidx);
}
/* thinker-order index; -1 NULL; -2 pointer not in the thinker list (dangling) */
static int tidx (void* p)
{
    pidx_t key, *r;
    if (!p) return -1;
    key.p = p;
    r = bsearch (&key, sorted, numth, sizeof *sorted, cmp_pidx);
    return r ? r->i : -2;
}
static long sidx (sector_t* s) { return s ? (long)(s - sectors) : -1L; }
static long stidx (state_t* s) { return s ? (long)(s - states) : -1L; }
static int in_plats (void* p) { int i; for (i = 0; i < MAXPLATS; i++) if (activeplats[i] == p) return 1; return 0; }
static int in_ceils (void* p) { int i; for (i = 0; i < MAXCEILINGS; i++) if (activeceilings[i] == p) return 1; return 0; }

void __real_P_RemoveThinker (thinker_t* thinker);
/* P_RemoveThinker observer: the kind (1 vldoor_t, 2 plat_t, 3 floormove_t, 4 ceiling_t, 0 other)
   and sector of a non-mobj thinker, read before the genuine call overwrites its function. */
void __wrap_P_RemoveThinker (thinker_t* thinker)
{
    obs_t* e = obs_find (thinker);
    if (e && !e->mobj) {
        actionf_p1 fn = thinker->function.acp1;
        if (fn == (actionf_p1)T_VerticalDoor) { e->rfn = 1; e->rsector = sidx (((vldoor_t*)thinker)->sector); }
        else if (fn == (actionf_p1)T_PlatRaise) { e->rfn = 2; e->rsector = sidx (((plat_t*)thinker)->sector); }
        else if (fn == (actionf_p1)T_MoveFloor) { e->rfn = 3; e->rsector = sidx (((floormove_t*)thinker)->sector); }
        else if (fn == (actionf_p1)T_MoveCeiling) { e->rfn = 4; e->rsector = sidx (((ceiling_t*)thinker)->sector); }
    }
    __real_P_RemoveThinker (thinker);
}

static const char* COLUMNS =
 "#columns,H,tic,leveltime,gametic,prndindex,rndindex,totalkills,totalitems,totalsecret,gameaction,gamestate,numthinkers,numlinespecials,gameskill,secretexit\n"
 "#columns,M,tic,idx,type,x,y,z,momx,momy,momz,health,state,tics,flags,height,radius,reactiontime,threshold,target,lastlook,floorz,ceilingz,subsector,angle,sprite,frame,movedir,movecount,tracer,sp_x,sp_y,sp_angle,sp_type,sp_options,player,bx,by,bpos,sector,spos\n"
 "#columns,R,tic,idx,fn,sector\n"
 "#columns,RR,tic,idx,type,x,y,z,momx,momy,momz,health,state,tics,flags,height,radius,reactiontime,threshold,target,lastlook,floorz,ceilingz,subsector,angle,sprite,frame,movedir,movecount,tracer,sp_x,sp_y,sp_angle,sp_type,sp_options,player,bx,by,bpos,sector,spos\n"
 "#columns,DR,tic,idx,type,sector,topheight,speed,direction,topwait,topcountdown\n"
 "#columns,PL,tic,idx,running,sector,speed,low,high,wait,count,status,oldstatus,crush,tag,type\n"
 "#columns,FL,tic,idx,type,crush,sector,direction,newspecial,texture,floordestheight,speed\n"
 "#columns,CE,tic,idx,running,type,sector,bottomheight,topheight,speed,crush,direction,tag,olddirection\n"
 "#columns,LF,tic,idx,sector,count,maxlight,minlight\n"
 "#columns,LL,tic,idx,sector,count,maxlight,minlight,maxtime,mintime\n"
 "#columns,LS,tic,idx,sector,count,minlight,maxlight,darktime,brighttime\n"
 "#columns,LG,tic,idx,sector,minlight,maxlight,direction\n"
 "#columns,X,tic,idx\n"
 "#columns,T,tic,idx,serial,linkserial\n"
 "#columns,K,tic,serial,seq,adds\n"
 "#columns,RM,tic,serial,type,x,y,z,momx,momy,momz,health,state,tics,flags,height,radius,reactiontime,threshold,target,lastlook,floorz,ceilingz,subsector,angle,sprite,frame,movedir,movecount,tracer,sp_x,sp_y,sp_angle,sp_type,sp_options,player,bx,by,bpos,sector,spos\n"
 "#columns,S,tic,idx,floorheight,ceilingheight,floorpic,ceilingpic,lightlevel,special,tag,soundtraversed,soundtarget,specialdata,linecount\n"
 "#columns,L,tic,idx,special,tag,flags,s0top,s0mid,s0bottom,s0textureoffset,s0rowoffset,s1top,s1mid,s1bottom\n"
 "#columns,B,tic,idx,line,where,btexture,btimer\n"
 "#columns,P,tic,idx,ingame,mo,playerstate,cmd_forward,cmd_side,cmd_turn,cmd_buttons,viewheight,deltaviewheight,bob,health,armorpoints,armortype,"
 "pw0,pw1,pw2,pw3,pw4,pw5,card0,card1,card2,card3,card4,card5,backpack,frag0,frag1,frag2,frag3,readyweapon,pendingweapon,"
 "wo0,wo1,wo2,wo3,wo4,wo5,wo6,wo7,wo8,ammo0,ammo1,ammo2,ammo3,maxammo0,maxammo1,maxammo2,maxammo3,attackdown,usedown,cheats,refire,"
 "killcount,itemcount,secretcount,damagecount,bonuscount,attacker,extralight,fixedcolormap,colormap,"
 "ps0state,ps0tics,ps0sx,ps0sy,ps1state,ps1tics,ps1sx,ps1sy,didsecret,message\n";

static void block_pos (mobj_t* m, int* bx, int* by, int* bpos)
{
    int x, y, n; mobj_t* l;
    *bx = *by = *bpos = -1;
    if (m->flags & MF_NOBLOCKMAP) return;
    x = (m->x - bmaporgx) >> MAPBLOCKSHIFT;    /* as P_SetThingPosition, p_maputl.c:510-511 */
    y = (m->y - bmaporgy) >> MAPBLOCKSHIFT;
    if (x < 0 || y < 0 || x >= bmapwidth || y >= bmapheight) return;
    *bx = x; *by = y; *bpos = -2;
    for (n = 0, l = blocklinks[y * bmapwidth + x]; l; l = l->bnext, n++) if (l == m) { *bpos = n; return; }
}
static int sector_pos (mobj_t* m)
{
    int n; mobj_t* l;
    if (m->flags & MF_NOSECTOR) return -1;
    for (n = 0, l = m->subsector->sector->thinglist; l; l = l->snext, n++) if (l == m) return n;
    return -2;
}

/* One M row (kind "M", id = thinker-list index, linked) or RM row (kind "RM", id = serial,
 * unlinked by P_RemoveMobj: bpos and spos -1). */
static void print_mobj (FILE* f, const char* kind, int tic, int id, mobj_t* m, int linked)
{
    int bx = -1, by = -1, bp = -1, sp = -1;
    if (linked) { block_pos (m, &bx, &by, &bp); sp = sector_pos (m); }
    fprintf (f, "%s,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%u,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d\n",
        kind, tic, id, (int)m->type, m->x, m->y, m->z, m->momx, m->momy, m->momz, m->health, stidx (m->state), m->tics,
        m->flags, m->height, m->radius, m->reactiontime, m->threshold, tidx (m->target), m->lastlook,
        m->floorz, m->ceilingz, m->subsector ? (long)(m->subsector - subsectors) : -1L, (unsigned)m->angle,
        (int)m->sprite, m->frame, m->movedir, m->movecount, tidx (m->tracer),
        m->spawnpoint.x, m->spawnpoint.y, m->spawnpoint.angle, m->spawnpoint.type, m->spawnpoint.options,
        m->player ? (int)(m->player - players) : -1, bx, by, bp,
        ((m->flags & MF_NOSECTOR) || !m->subsector) ? -1L : sidx (m->subsector->sector), sp);
}

static int mask_mapped = 1;              /* M5a: L.flags without ML_MAPPED (decision R5) */
/* ---------------- M5b: the status bar, the HUD, the palette and the D_Display mirror ----------------
 * Everything in this block observes genuine code through -Wl,--wrap (each wrapper records into
 * driver tables and then calls the genuine function unchanged) or mirrors D_Display
 * (d_main.c:191-355) for GS_LEVEL without menu, automap, pause or wipe. Under the declared
 * sound-free, wipe-free contract (decision S15) every M_Random draw of this runner is a status
 * bar draw: s_sound.c and f_wipe.c are not compiled, and the M_Random call sites are st_stuff.c,
 * s_sound.c, f_wipe.c and wi_stuff.c only. */
#include "r_draw.h"
#include "m_swap.h"
static int mask_rnd = 1;                  /* S16: dump rndindex without the presentation draws */
static int hu_record = 1;                 /* S16: dump P.message as the genuine HU_Ticker found it */
static const char* lump_trace_path; static FILE* lump_trace;
static int selftest_bar_tic = -1, selftest_bar_x, selftest_bar_y, selftest_pal_tic = -1, selftest_msg_tic = -1, selftest_rnd_tic = -1;
static int m5b_tic = -1, diag_fill = -1;
static int pres_draws;                    /* M_Random calls since M_ClearRandom */
static int in_st_ticker, st_draws, other_draws;
static int cur_palette = -2, palette_set_this_frame = -1, palette_sets, palette_unknown;
static int frame_face = -1, frame_msg_on;
static int bar_undef_px, frame_skipped, frame_beyond, frame_lump_overrun;
static int total_undef_px, total_skipped, total_beyond, total_lump_overrun;
static unsigned char bg_def[SCREENWIDTH*32];      /* 1: the BG byte (screens[4], st_stuff.c:1468) was written from defined patch bytes */
static int bg_origin = -1;
static int attacker_refused;              /* S17: the face would read a removed attacker */

extern boolean setsizeneeded, viewactive, menuactive, inhelpscreens, paused, nodrawers;
extern gamestate_t wipegamestate;
extern int scaledviewwidth;

/* M_Random (m_random.c:47-51): counted, then the genuine draw. ST_Ticker's st_randomnumber
 * (st_stuff.c:992) is the only call in this build (other_draws stays 0, checked by the tool). */
int __real_M_Random (void);
int __wrap_M_Random (void)
{
    pres_draws++;
    if (in_st_ticker) st_draws++; else other_draws++;
    return __real_M_Random ();
}
/* M_ClearRandom (m_random.c:60-63) resets both indices at G_InitNew (g_game.c:1414). */
void __real_M_ClearRandom (void);
void __wrap_M_ClearRandom (void) { pres_draws = 0; __real_M_ClearRandom (); }
/* S16: the H row prints the gameplay-only index (the value every frozen runner prints). */
static int m5b_dump_rndindex (void) { return mask_rnd ? ((rndindex - pres_draws) & 0xff) : rndindex; }

/* HU_Ticker (hu_stuff.c:505-578): the message it finds is recorded before the genuine function
 * takes it and clears plr->message (hu_stuff.c:519-530). */
static int new_message_tic = -1, new_message_idx; static char new_message_text[96];
void __real_HU_Ticker (void);
void __wrap_HU_Ticker (void)
{
    stub_hu_ran = 1;
    stub_hu_message = players[consoleplayer].message;
    __real_HU_Ticker ();
    if (stub_hu_message && !players[consoleplayer].message) {   /* taken: HUlib_addMessageToSText, then plr->message = 0 */
        new_message_tic = m5b_tic; new_message_idx = message_index (stub_hu_message);
        snprintf (new_message_text, sizeof new_message_text, "%s", stub_hu_message);
    }
}

/* S17: is the attacker the face is about to read (st_stuff.c:799-813) still allocated? A mobj
 * stays allocated while it is in the thinker list (P_RemoveThinker only marks it, p_tick.c:84-90;
 * P_RunThinkers frees it on the next pass, p_tick.c:117-128). */
static int attacker_live (mobj_t* a)
{
    thinker_t* t;
    for (t = thinkercap.next; t != &thinkercap; t = t->next) if (t == &a->thinker) return 1;
    return 0;
}
void __real_ST_Ticker (void);
void __wrap_ST_Ticker (void)
{
    player_t* p = &players[consoleplayer];
    if (p->damagecount && p->attacker && p->attacker != p->mo && !attacker_live (p->attacker)) {
        set_witness (4, 0);                  /* AttackerRemoved: the genuine code would read freed memory */
        attacker_refused = 1;
        return;
    }
    in_st_ticker = 1; __real_ST_Ticker (); in_st_ticker = 0;
}

/* The palette index I_SetPalette received (st_stuff.c:1045-1050: PLAYPAL + palette*768). */
static void on_palette (byte* pal)
{
    static int lu = -1; long off;
    if (lu < 0) lu = W_CheckNumForName ("PLAYPAL");
    off = lu >= 0 && lumpcache[lu] ? (long)(pal - (byte*)lumpcache[lu]) : -1;
    cur_palette = (off >= 0 && off % 768 == 0 && off / 768 < 14) ? (int)(off / 768) : -1;
    if (cur_palette < 0) palette_unknown++;
    palette_set_this_frame = cur_palette;
    palette_sets++;
}

/* Widget value tables (W rows: which widgets were drawn with which values). */
typedef struct { char kind; int x, y, first, min, max, changes, last, n; } widget_t;
static widget_t widgets[64]; static int nwidgets;
static void note_widget (char kind, int x, int y, int v)
{
    int i; widget_t* w = NULL;
    for (i = 0; i < nwidgets; i++) if (widgets[i].kind == kind && widgets[i].x == x && widgets[i].y == y) { w = &widgets[i]; break; }
    if (!w) { if (nwidgets == 64) return; w = &widgets[nwidgets++]; w->kind = kind; w->x = x; w->y = y; w->first = w->min = w->max = w->last = v; w->changes = 0; w->n = 0; }
    if (v != w->last) w->changes++;
    if (v < w->min) w->min = v;
    if (v > w->max) w->max = v;
    w->last = v; w->n++;
}
#define M5B_NUMFACES 42                   /* ST_NUMFACES (st_stuff.c:110) */
static unsigned char faces_seen[M5B_NUMFACES];
void __real_STlib_updateMultIcon (st_multicon_t* mi, boolean refresh);
void __wrap_STlib_updateMultIcon (st_multicon_t* mi, boolean refresh)
{
    if (mi->x == 143 && mi->y == 168) {      /* w_faces at ST_FACESX, ST_FACESY (st_stuff.c:1342-1347) */
        frame_face = *mi->inum;
        if (frame_face >= 0 && frame_face < M5B_NUMFACES) faces_seen[frame_face] = 1;
    }
    note_widget ('M', mi->x, mi->y, *mi->inum);
    __real_STlib_updateMultIcon (mi, refresh);
}
void __real_STlib_updateNum (st_number_t* n, boolean refresh);
void __wrap_STlib_updateNum (st_number_t* n, boolean refresh) { note_widget ('N', n->x, n->y, *n->num); __real_STlib_updateNum (n, refresh); }
void __real_STlib_updatePercent (st_percent_t* per, int refresh);
void __wrap_STlib_updatePercent (st_percent_t* per, int refresh) { note_widget ('P', per->n.x, per->n.y, *per->n.num); __real_STlib_updatePercent (per, refresh); }
void __real_STlib_updateBinIcon (st_binicon_t* bi, boolean refresh);
void __wrap_STlib_updateBinIcon (st_binicon_t* bi, boolean refresh) { note_widget ('B', bi->x, bi->y, *bi->val); __real_STlib_updateBinIcon (bi, refresh); }
void __real_HUlib_drawSText (hu_stext_t* s);
void __wrap_HUlib_drawSText (hu_stext_t* s) { frame_msg_on = *s->on; __real_HUlib_drawSText (s); }

/* Lump-lookup trace (--lump-trace FILE, diagnostic): which lumps the genuine code asks for by name. */
void* __real_W_CacheLumpName (char* name, int tag);
void* __wrap_W_CacheLumpName (char* name, int tag) { if (lump_trace) fprintf (lump_trace, "C %.8s\n", name); return __real_W_CacheLumpName (name, tag); }
int __real_W_GetNumForName (char* name);
int __wrap_W_GetNumForName (char* name) { if (lump_trace) fprintf (lump_trace, "G %.8s\n", name); return __real_W_GetNumForName (name); }

/* The read-provenance tracker on the 2D drawers (section 2.1.9 of the M5b note). V_DrawPatch and
 * V_DrawPatchDirect (v_video.c:210-262, 331-344) read post bytes until 0xff with no bound but the
 * lump: a post byte outside its lump taints the pixel with the lump's origin (lump_overrun); a
 * write below row 199 of the screen or row 31 of screens[4] is counted (beyond_rows); a patch the
 * range check skips (v_video.c:221-232, printed and ignored) is counted (skipped_patches). The
 * pixels written from defined bytes clear the screen taint. V_CopyRect (v_video.c:165-195) from
 * screens[4] copies a "defined" mask: a BG byte no patch wrote is an undefined read (bar_undef_px,
 * origin BG:unset). This reads genuine state only and then calls the genuine drawer. */
static int lump_of_ptr (const void* p)
{
    int i;
    for (i = numlumps - 1; i >= 0; i--) if (lumpcache[i] == p) return i;
    return -1;
}
static void track_patch (int x, int y, int scrn, patch_t* patch)
{
    int lump = lump_of_ptr (patch), w, col; long size;
    if (scrn != 0 && scrn != 4) return;
    size = lump >= 0 ? lumpinfo[lump].size : 0;
    y -= SHORT (patch->topoffset); x -= SHORT (patch->leftoffset);
    if (x < 0 || x + SHORT (patch->width) > SCREENWIDTH || y < 0 || y + SHORT (patch->height) > SCREENHEIGHT) { frame_skipped++; return; }
    w = SHORT (patch->width);
    for (col = 0; col < w; col++, x++) {
        long cofs = LONG (patch->columnofs[col]);
        const byte* c;
        if (lump < 0 || cofs < 0 || cofs >= size) { frame_lump_overrun++; continue; }
        c = (const byte*)patch + cofs;
        while (1) {
            int k, top, len, ok;
            if (c - (const byte*)patch >= size) { frame_lump_overrun++; break; }
            if (c[0] == 0xff) break;
            top = c[0]; len = (c - (const byte*)patch + 1 < size) ? c[1] : 0;
            ok = (c - (const byte*)patch) + 3 + len <= size;
            if (!ok) frame_lump_overrun++;
            for (k = 0; k < len; k++) {
                int yy = y + top + k;
                if (scrn == 0) {
                    if (yy >= SCREENHEIGHT) { frame_beyond++; continue; }
                    taint[yy*SCREENWIDTH + x] = ok ? 0 : (unsigned short)(lump_origin_of (lump) + 1);
                } else {
                    if (yy >= 32) { frame_beyond++; continue; }
                    bg_def[yy*SCREENWIDTH + x] = ok;
                }
            }
            c += len + 4;
        }
    }
}
void __real_V_DrawPatch (int x, int y, int scrn, patch_t* patch);
void __wrap_V_DrawPatch (int x, int y, int scrn, patch_t* patch) { track_patch (x, y, scrn, patch); __real_V_DrawPatch (x, y, scrn, patch); }
void __real_V_DrawPatchDirect (int x, int y, int scrn, patch_t* patch);
void __wrap_V_DrawPatchDirect (int x, int y, int scrn, patch_t* patch) { track_patch (x, y, scrn, patch); __real_V_DrawPatchDirect (x, y, scrn, patch); }
void __real_V_CopyRect (int srcx, int srcy, int srcscrn, int width, int height, int destx, int desty, int destscrn);
void __wrap_V_CopyRect (int srcx, int srcy, int srcscrn, int width, int height, int destx, int desty, int destscrn)
{
    int i, j;
    if (srcscrn == 4 && destscrn == 0 && srcx >= 0 && srcy >= 0 && srcx + width <= SCREENWIDTH && srcy + height <= 32
        && destx >= 0 && desty >= 0 && destx + width <= SCREENWIDTH && desty + height <= SCREENHEIGHT)
        for (j = 0; j < height; j++) for (i = 0; i < width; i++) {
            int d = bg_def[(srcy + j)*SCREENWIDTH + srcx + i];
            if (!d) { if (bg_origin < 0) bg_origin = new_origin ("BG", "unset"); bar_undef_px++; }
            taint[(desty + j)*SCREENWIDTH + destx + i] = d ? 0 : (unsigned short)(bg_origin + 1);
        }
    __real_V_CopyRect (srcx, srcy, srcscrn, width, height, destx, desty, destscrn);
}

/* ---- the command source ----
 * DOOM-M5B-RUNNER-v1 has one command source: a version-110 demo lump played through the genuine
 * -playdemo path. command_source_begin loads it (G_DeferedPlayDemo, d_main.c:1135-1138, and
 * G_DoPlayDemo, g_game.c:634-636 and 1580-1617); each tic's command is then read by the genuine
 * G_Ticker from the demo buffer (G_ReadDemoTiccmd, g_game.c:656-669 and 1484-1504), so
 * command_before_tic has nothing to do. Every other command source is a later, separately
 * declared mode added here; it does not change what this one means. */
static void command_source_begin (char* demo)
{
    G_DeferedPlayDemo (demo);                    /* d_main.c:1138 */
    G_DoPlayDemo ();                             /* G_Ticker's ga_playdemo case, g_game.c:634-636 */
}
static void command_before_tic (int tic) { (void)tic; }

/* ---- the bespoke scenario (decision S1): genuine cheat key events ----
 * --bespoke-keys TIC TEXT posts a key-down and a key-up event for every character of TEXT to the
 * genuine G_Responder (g_game.c:504-580) before G_Ticker of tic TIC, where D_ProcessEvents runs in
 * the -timedemo loop (d_main.c:375-383). D_ProcessEvents offers each event to M_Responder first
 * (d_main.c:161-177), which returns false for a letter key while no menu, message or save string is
 * up (m_menu.c:1349-1617), so the events reach G_Responder unchanged: HU_Responder (genuine; a letter
 * does nothing outside a netgame, hu_stuff.c:654-700), ST_Responder (genuine; the cheats of
 * st_stuff.c:518-700) and the AM_Responder stub (am_map.c:625-633 returns false for every key but
 * AM_STARTKEY while the automap is off). Used
 * only by the bespoke scenario: the god face (iddqd, st_stuff.c:549-560, 870-882) and the berserk
 * term of the palette (idbeholds, st_stuff.c:636-648, 1008-1016), which wire 24 never reaches. */
#define M5B_MAXBESPOKE 8
static int bespoke_tic[M5B_MAXBESPOKE]; static char* bespoke_text[M5B_MAXBESPOKE]; static int nbespoke;
static void bespoke_events (int tic)
{
    int i; char* s; event_t ev;
    for (i = 0; i < nbespoke; i++) if (bespoke_tic[i] == tic)
        for (s = bespoke_text[i]; *s; s++) {
            ev.type = ev_keydown; ev.data1 = (unsigned char)*s; ev.data2 = ev.data3 = 0; G_Responder (&ev);
            ev.type = ev_keyup; G_Responder (&ev);
        }
}

/* ---- the D_Display mirror (d_main.c:191-355) ---- */
static boolean dd_viewactivestate = false, dd_menuactivestate = false, dd_inhelpscreensstate = false, dd_fullscreen = false;
static gamestate_t dd_oldgamestate = -1;
static int dd_borderdrawcount, dd_wipe;
static void render (int tic);
static void m5b_display (int tic)
{
    boolean redrawsbar;
    cur_tic = tic;
    palette_set_this_frame = -1; frame_face = -1; frame_msg_on = 0;
    bar_undef_px = frame_skipped = frame_beyond = frame_lump_overrun = 0;
    if (nodrawers) return;                                  /* d_main.c:209-210 (false) */
    redrawsbar = false;
    if (setsizeneeded) {                                    /* d_main.c:215-220 */
        R_ExecuteSetViewSize ();
        dd_oldgamestate = -1;
        dd_borderdrawcount = 3;
    }
    dd_wipe = gamestate != wipegamestate;                   /* d_main.c:223-229; the melt is not run (S15) */
    if (gamestate == GS_LEVEL && gametic) HU_Erase ();      /* d_main.c:231-232 */
    if (gamestate != GS_LEVEL || automapactive) { fprintf (stderr, "display: not GS_LEVEL first-person\n"); abort (); }
    if (gametic) {                                          /* d_main.c:237-248 */
        if (dd_wipe || (viewheight != 200 && dd_fullscreen)) redrawsbar = true;
        if (dd_inhelpscreensstate && !inhelpscreens) redrawsbar = true;
        ST_Drawer (viewheight == 200, redrawsbar);
        dd_fullscreen = viewheight == 200;
    }
    if (gamestate == GS_LEVEL && !automapactive && gametic) render (tic);   /* d_main.c:267-268, then m5b_after_view */
}
static void m5b_after_view (int tic)
{
    if (gamestate == GS_LEVEL && gametic) HU_Drawer ();     /* d_main.c:270-271 */
    /* d_main.c:274-275: I_SetPalette (PLAYPAL) only when gamestate != GS_LEVEL: never here */
    if (gamestate == GS_LEVEL && dd_oldgamestate != GS_LEVEL) {   /* d_main.c:278-282 */
        dd_viewactivestate = false;
        R_FillBackScreen ();                                /* returns at once when scaledviewwidth == 320 (r_draw.c:746-747) */
    }
    if (gamestate == GS_LEVEL && !automapactive && scaledviewwidth != 320) {   /* d_main.c:285-295 */
        if (menuactive || dd_menuactivestate || !dd_viewactivestate) dd_borderdrawcount = 3;
        if (dd_borderdrawcount) { R_DrawViewBorder (); dd_borderdrawcount--; }
    }
    dd_menuactivestate = menuactive;                        /* d_main.c:297-300 */
    dd_viewactivestate = viewactive;
    dd_inhelpscreensstate = inhelpscreens;
    dd_oldgamestate = wipegamestate = gamestate;
    if (paused) { fprintf (stderr, "display: paused\n"); abort (); }   /* d_main.c:303-312: never in a demo without pause */
    /* M_Drawer (d_main.c:316): no menu is ever active; NetUpdate (d_main.c:317) is the declared stub */
    NetUpdate ();
    if (tic == selftest_bar_tic) screens[0][selftest_bar_y*SCREENWIDTH + selftest_bar_x] ^= 1;   /* --selftest-bar: test only */
}
/* The P row (golden) and the D row (diagnostic) of a frame, written after its F row. */
static void m5b_status_rows (int tic)
{
    static unsigned char v[SCREENWIDTH*SCREENHEIGHT]; char vsha[65]; int y0 = viewwindowy + viewheight;
    memcpy (v, screens[0], sizeof v);
    if (y0 < SCREENHEIGHT) memset (v + y0*SCREENWIDTH, 0, (SCREENHEIGHT - y0)*SCREENWIDTH);   /* view-only digest (M5a frames are zero below the view) */
    sha256_hex (v, sizeof v, vsha);
    fprintf (frames_out, "P,%d,%d,%d,%d,%d,%d\n", tic,
             tic == selftest_pal_tic ? cur_palette ^ 1 : cur_palette,              /* --selftest-palette: test only */
             palette_set_this_frame, frame_face, frame_msg_on,
             tic == selftest_rnd_tic ? rndindex ^ 1 : rndindex);                   /* --selftest-rnd: test only */
    if (new_message_tic >= 0) {
        fprintf (frames_out, "X,%d,%d,%s\n", new_message_tic == selftest_msg_tic ? new_message_tic + 1 : new_message_tic,   /* --selftest-message: test only */
                 new_message_idx, new_message_text);
        new_message_tic = -1;
    }
    fprintf (frames_out, "D,%d,%d,%d,%d,%d,%d,%d,%s\n", tic, st_draws, other_draws, bar_undef_px, frame_skipped, frame_beyond,
             frame_lump_overrun, vsha);
    total_undef_px += bar_undef_px; total_skipped += frame_skipped; total_beyond += frame_beyond; total_lump_overrun += frame_lump_overrun;
}
static void m5b_finish (void)
{
    int i;
    if (frames_out) {
        for (i = 0; i < nwidgets; i++)
            fprintf (frames_out, "W,%c,%d,%d,%d,%d,%d,%d,%d\n", widgets[i].kind, widgets[i].x, widgets[i].y, widgets[i].n, widgets[i].first,
                     widgets[i].min, widgets[i].max, widgets[i].changes);
        fprintf (frames_out, "Z,palette_sets,%d,palette_unknown,%d,st_draws,%d,other_draws,%d,undef_px,%d,skipped,%d,beyond,%d,lump_overrun,%d\n",
                 palette_sets, palette_unknown, st_draws, other_draws, total_undef_px, total_skipped, total_beyond, total_lump_overrun);
    }
    if (lump_trace) fclose (lump_trace);
}
static void dump (const char* dir, const char* name, int tic)
{
    char path[1024]; FILE* f; int i, j;
    snprintf (path, sizeof path, "%s/%s", dir, name);
    f = fopen (path, "w");
    if (!f) { perror (path); exit (4); }
    walk_thinkers ();
    fputs (COLUMNS, f);
    fprintf (f, "H,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic, leveltime, gametic, prndindex, m5b_dump_rndindex (),
             totalkills, totalitems, totalsecret, (int)gameaction, (int)gamestate, numth, (int)numlinespecials,
             (int)gameskill, (int)secretexit);
    for (i = 0; i < numth; i++) {
        thinker_t* t = order[i];
        actionf_p1 fn = t->function.acp1;
        if (fn == (actionf_p1)P_MobjThinker) {
            print_mobj (f, "M", tic, i, (mobj_t*)t, 1);
        } else if (t->function.acv == (actionf_v)(-1)) {
            { obs_t* e = obs_find (t);
              fprintf (f, "R,%d,%d,%d,%ld\n", tic, i, e ? e->rfn : 0, e ? e->rsector : -1L);
              if (e && e->mobj) print_mobj (f, "RR", tic, i, (mobj_t*)t, 0); }
        } else if (fn == (actionf_p1)T_VerticalDoor) {
            vldoor_t* d = (vldoor_t*)t;
            fprintf (f, "DR,%d,%d,%d,%ld,%d,%d,%d,%d,%d\n", tic, i, (int)d->type, sidx (d->sector), d->topheight, d->speed,
                     d->direction, d->topwait, d->topcountdown);
        } else if (fn == (actionf_p1)T_PlatRaise || (fn == NULL && in_plats (t))) {
            plat_t* p = (plat_t*)t;
            fprintf (f, "PL,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic, i, fn != NULL, sidx (p->sector), p->speed, p->low,
                     p->high, p->wait, p->count, (int)p->status, (int)p->oldstatus, (int)p->crush, p->tag, (int)p->type);
        } else if (fn == (actionf_p1)T_MoveFloor) {
            floormove_t* p = (floormove_t*)t;
            fprintf (f, "FL,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d\n", tic, i, (int)p->type, (int)p->crush, sidx (p->sector),
                     p->direction, p->newspecial, (int)p->texture, p->floordestheight, p->speed);
        } else if (fn == (actionf_p1)T_MoveCeiling || (fn == NULL && in_ceils (t))) {
            ceiling_t* p = (ceiling_t*)t;
            fprintf (f, "CE,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d\n", tic, i, fn != NULL, (int)p->type, sidx (p->sector),
                     p->bottomheight, p->topheight, p->speed, (int)p->crush, p->direction, p->tag, p->olddirection);
        } else if (fn == (actionf_p1)T_FireFlicker) {
            fireflicker_t* p = (fireflicker_t*)t;
            fprintf (f, "LF,%d,%d,%ld,%d,%d,%d\n", tic, i, sidx (p->sector), p->count, p->maxlight, p->minlight);
        } else if (fn == (actionf_p1)T_LightFlash) {
            lightflash_t* p = (lightflash_t*)t;
            fprintf (f, "LL,%d,%d,%ld,%d,%d,%d,%d,%d\n", tic, i, sidx (p->sector), p->count, p->maxlight, p->minlight,
                     p->maxtime, p->mintime);
        } else if (fn == (actionf_p1)T_StrobeFlash) {
            strobe_t* p = (strobe_t*)t;
            fprintf (f, "LS,%d,%d,%ld,%d,%d,%d,%d,%d\n", tic, i, sidx (p->sector), p->count, p->minlight, p->maxlight,
                     p->darktime, p->brighttime);
        } else if (fn == (actionf_p1)T_Glow) {
            glow_t* p = (glow_t*)t;
            fprintf (f, "LG,%d,%d,%ld,%d,%d,%d\n", tic, i, sidx (p->sector), p->minlight, p->maxlight, p->direction);
        } else {
            fprintf (f, "X,%d,%d\n", tic, i);
        }
    }
    for (i = 0; i < numth; i++)
        fprintf (f, "T,%d,%d,%d,%d\n", tic, i, serial_of (order[i]), linkserial_of (order[i]));
    for (i = 0; i < nfreed; i++) fprintf (f, "K,%d,%d,%d,%d\n", tic, freed[i].serial, freed[i].seq, freed[i].adds);
    for (i = 0; i < nremoved; i++) print_mobj (f, "RM", tic, removed[i].serial, &removed[i].m, 0);
    nfreed = nremoved = freeseq = 0;
    for (i = 0; i < numsectors; i++) {
        sector_t* s = &sectors[i];
        fprintf (f, "S,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic, i, s->floorheight, s->ceilingheight,
                 (int)s->floorpic, (int)s->ceilingpic, (int)s->lightlevel, (int)s->special, (int)s->tag,
                 s->soundtraversed, tidx (s->soundtarget), tidx (s->specialdata), s->linecount);
    }
    for (i = 0; i < numlines; i++) {
        line_t* l = &lines[i]; side_t* s0; side_t* s1;
        if (!linesel[i]) continue;
        s0 = l->sidenum[0] >= 0 ? &sides[l->sidenum[0]] : NULL;
        s1 = l->sidenum[1] >= 0 ? &sides[l->sidenum[1]] : NULL;
        fprintf (f, "L,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic, i, (int)l->special, (int)l->tag, (int)(mask_mapped ? l->flags & ~ML_MAPPED : l->flags),
                 s0 ? (int)s0->toptexture : -1, s0 ? (int)s0->midtexture : -1, s0 ? (int)s0->bottomtexture : -1,
                 s0 ? s0->textureoffset : 0, s0 ? s0->rowoffset : 0,
                 s1 ? (int)s1->toptexture : -1, s1 ? (int)s1->midtexture : -1, s1 ? (int)s1->bottomtexture : -1);
    }
    for (i = 0; i < MAXBUTTONS; i++) {
        button_t* b = &buttonlist[i];
        fprintf (f, "B,%d,%d,%ld,%d,%d,%d\n", tic, i, b->line ? (long)(b->line - lines) : -1L, (int)b->where,
                 b->btexture, b->btimer);
    }
    for (i = 0; i < MAXPLAYERS; i++) {
        player_t* p = &players[i];
        fprintf (f, "P,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d", tic, i, (int)playeringame[i], tidx (p->mo),
                 (int)p->playerstate, (int)p->cmd.forwardmove, (int)p->cmd.sidemove, (int)p->cmd.angleturn,
                 (int)p->cmd.buttons, p->viewheight, p->deltaviewheight, p->bob, p->health, p->armorpoints, p->armortype);
        for (j = 0; j < NUMPOWERS; j++) fprintf (f, ",%d", p->powers[j]);
        for (j = 0; j < NUMCARDS; j++) fprintf (f, ",%d", (int)p->cards[j]);
        fprintf (f, ",%d", (int)p->backpack);
        for (j = 0; j < MAXPLAYERS; j++) fprintf (f, ",%d", p->frags[j]);
        fprintf (f, ",%d,%d", (int)p->readyweapon, (int)p->pendingweapon);
        for (j = 0; j < NUMWEAPONS; j++) fprintf (f, ",%d", (int)p->weaponowned[j]);
        for (j = 0; j < NUMAMMO; j++) fprintf (f, ",%d", p->ammo[j]);
        for (j = 0; j < NUMAMMO; j++) fprintf (f, ",%d", p->maxammo[j]);
        fprintf (f, ",%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d", p->attackdown, p->usedown, p->cheats, p->refire,
                 p->killcount, p->itemcount, p->secretcount, p->damagecount, p->bonuscount, tidx (p->attacker),
                 p->extralight, p->fixedcolormap, p->colormap);
        for (j = 0; j < NUMPSPRITES; j++)
            fprintf (f, ",%ld,%d,%d,%d", stidx (p->psprites[j].state), p->psprites[j].tics, p->psprites[j].sx, p->psprites[j].sy);
        fprintf (f, ",%d,%d\n", (int)p->didsecret,
                 message_index (i == consoleplayer && stub_hu_ran && hu_record ? stub_hu_message : p->message));
    }
    fclose (f);
}


/* ---------------- M5a: frames and view records ---------------- */
static int render_mode;                  /* 0 none, 1 every dumped tic */
static FILE* view_out;
static const char* raw_dir;
static unsigned char* mapped_before;
static int frames_rendered, frames_flagged, columns_flagged, refused;
static int selftest_tic = -1, selftest_x, selftest_y;   /* --selftest-pixel TIC X Y (test only) */

static void on_error (const char* message)
{
    if (frames_out) { fprintf (frames_out, "E,%d,%s\n", cur_tic, message); fflush (frames_out); }
}

static const char* FRAME_COLUMNS =
 "#schema,DOOM-M5B-FRAMES-v1\n"
 "#columns,F,tic,sha256,masked_sha256,fuzzpos,mapped,columns,visplanes,drawsegs,vissprites,openings,solidsegs_peak,drawseg_drops,vissprite_drop,whole\n"
 "#columns,M,tic,lines\n"
 "#columns,N,origin,name\n"
 "#columns,U,tic,x,origins\n"
 "#columns,O,tic,witness,visplanes,openings,solidsegs_peak\n"
 "#columns,E,tic,message\n"
 "#columns,P,tic,palette,palette_set,face,message_on,rndindex\n"
 "#columns,X,tic,code,text\n"
 "#columns,D,tic,st_draws,other_draws,bar_undef_px,skipped_patches,beyond_rows,lump_overrun,view_sha256\n"
 "#columns,W,kind,x,y,calls,first,min,max,changes\n"
 "#columns,Z,totals\n";

static void render (int tic)
{
    static unsigned char masked[SCREENWIDTH*SCREENHEIGHT];
    char full[65], msha[65]; int i, x, y, ncols = 0, mapped = 0, vdrop, first;
    static int colflag[SCREENWIDTH];
    for (i = 0; i < numlines; i++) mapped_before[i] = (lines[i].flags & ML_MAPPED) != 0;
    frame_whole = frame_solid_peak = frame_drawseg_drops = frame_storewalls = 0;
    memset (&overflowsprite, 0x5a, sizeof overflowsprite);   /* write-only sink of R_NewVisSprite, r_things.c:326-331 */
    cur_tic = tic; tracking = 1;
    R_RenderPlayerView (&players[displayplayer]);            /* d_main.c:267-268 */
    tracking = 0;
    if (tic == selftest_tic) screens[0][selftest_y*SCREENWIDTH + selftest_x] ^= 1;   /* --selftest-pixel: test only */
    m5b_after_view (tic);                                     /* M5b: HU_Drawer and the rest of D_Display */
    { int n = (int)(newend - solidsegs); if (n > frame_solid_peak) frame_solid_peak = n; if (n > M5A_MAXSEGS) set_witness (3, n); }
    { unsigned char probe[sizeof overflowsprite]; memset (probe, 0x5a, sizeof probe); vdrop = memcmp (probe, &overflowsprite, sizeof probe) != 0; }
    frames_rendered++;
    sha256_hex (screens[0], SCREENWIDTH*SCREENHEIGHT, full);
    memcpy (masked, screens[0], sizeof masked);
    for (x = 0; x < SCREENWIDTH; x++) {
        colflag[x] = 0;
        for (y = 0; y < SCREENHEIGHT; y++) if (taint[y*SCREENWIDTH + x]) { colflag[x] = 1; break; }
        if (colflag[x]) { ncols++; for (y = 0; y < SCREENHEIGHT; y++) masked[y*SCREENWIDTH + x] = 0; }
    }
    if (ncols) sha256_hex (masked, sizeof masked, msha); else strcpy (msha, "-");
    for (i = 0; i < numlines; i++) mapped += (lines[i].flags & ML_MAPPED) != 0;
    if (frames_out) {
        while (origins_written < norigins) { fprintf (frames_out, "N,%d,%s\n", origins_written, origin_name[origins_written]); origins_written++; }
        fprintf (frames_out, "F,%d,%s,%s,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic, full, msha, fuzzpos, mapped, ncols,
                 (int)(lastvisplane - visplanes), (int)(ds_p - drawsegs), (int)(vissprite_p - vissprites),
                 (int)(lastopening - openings), frame_solid_peak, frame_drawseg_drops, vdrop, frame_whole);
        for (first = 1, i = 0; i < numlines; i++)
            if ((lines[i].flags & ML_MAPPED) && !mapped_before[i]) {
                if (first) fprintf (frames_out, "M,%d,%d", tic, i); else fprintf (frames_out, " %d", i);
                first = 0;
            }
        if (!first) fputc ('\n', frames_out);
        m5b_status_rows (tic);
        for (x = 0; x < SCREENWIDTH; x++) if (colflag[x]) {
            int seen[16], ns = 0, k;
            fprintf (frames_out, "U,%d,%d,", tic, x);
            for (y = 0; y < SCREENHEIGHT; y++) {
                int t = taint[y*SCREENWIDTH + x];
                if (!t) continue;
                for (k = 0; k < ns && seen[k] != t; k++) ;
                if (k == ns && ns < 16) { fprintf (frames_out, ns ? "+%d" : "%d", t - 1); seen[ns++] = t; }
            }
            fputc ('\n', frames_out);
        }
    }
    if (ncols) { frames_flagged++; columns_flagged += ncols; }
    if (raw_dir) {
        char path[1024]; FILE* f;
        snprintf (path, sizeof path, "%s/frame_%05d.raw", raw_dir, tic);
        if (!(f = fopen (path, "wb"))) { perror (path); exit (4); }
        fwrite (screens[0], 1, SCREENWIDTH*SCREENHEIGHT, f); fclose (f);
    }
    if (frames_out) fflush (frames_out);                     /* a later abort must not lose rows */
    if (witness) refused = 1;
}

/* The view record (decision R6, schema DOOM-M5A-VIEW-v1): what R_SetupFrame (r_main.c:846-875)
 * and R_DrawPlayerSprites (r_things.c:783-815) read from the player, written after every
 * dumped tic whether or not the tic is rendered. viewz, onground (p_user.c:51) and the
 * animation translations are not in the game dump; viewheight, deltaviewheight and bob are
 * repeated from the P row so the view shadow is validated from one record. A rows list the
 * flats and textures whose translation (P_UpdateSpecials, p_spec.c:1158-1185) is not the
 * identity. */
static const char* VIEW_COLUMNS =
 "#schema,DOOM-M5A-VIEW-v1\n"
 "#columns,V,tic,viewx,viewy,viewz,viewangle,viewheight,deltaviewheight,bob,onground,extralight,fixedcolormap,invisibility,ps0state,ps0tics,ps0sx,ps0sy,ps1state,ps1tics,ps1sx,ps1sy,sector,lightlevel\n"
 "#columns,A,tic,kind,index,translation\n";
static void write_view (int tic)
{
    player_t* p = &players[displayplayer]; mobj_t* mo = p->mo; int i;
    fprintf (view_out, "V,%d,%d,%d,%d,%u,%d,%d,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%ld,%d,%d,%d,%ld,%d\n", tic, mo->x, mo->y, p->viewz,
             (unsigned)mo->angle, p->viewheight, p->deltaviewheight, p->bob, (int)onground, p->extralight, p->fixedcolormap,
             p->powers[pw_invisibility], stidx (p->psprites[0].state), p->psprites[0].tics, p->psprites[0].sx, p->psprites[0].sy,
             stidx (p->psprites[1].state), p->psprites[1].tics, p->psprites[1].sx, p->psprites[1].sy,
             sidx (mo->subsector->sector), (int)mo->subsector->sector->lightlevel);
    for (i = 0; i < numflats; i++) if (flattranslation[i] != i) fprintf (view_out, "A,%d,flat,%d,%d\n", tic, i, flattranslation[i]);
    for (i = 0; i < numtextures; i++) if (texturetranslation[i] != i) fprintf (view_out, "A,%d,texture,%d,%d\n", tic, i, texturetranslation[i]);
}

static double now (void)
{
    struct timespec ts;
    clock_gettime (CLOCK_MONOTONIC, &ts);   /* driver timing only, never read by gameplay */
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

int main (int argc, char** argv)
{
    char* demo = NULL; char* out = NULL; char* frames_path = NULL; char* view_path = NULL; int extra_draw = -1, i, tic, checkheap = 1, nodump = 0, exited = 0, reborn = 0, ran;
    double t0, tick_time = 0, dump_time = 0;
    for (i = 1; i < argc; i++) {
        if (!strcmp (argv[i], "--wad") && i + 1 < argc) { if (numwads == MAXWADS) { fprintf (stderr, "too many --wad\n"); return 2; } add_file (argv[++i]); }
        else if (!strcmp (argv[i], "--demo") && i + 1 < argc) demo = argv[++i];
        else if (!strcmp (argv[i], "--out") && i + 1 < argc) out = argv[++i];
        else if (!strcmp (argv[i], "--zone-mb") && i + 1 < argc) stub_zone_mb = atoi (argv[++i]);
        else if (!strcmp (argv[i], "--selftest-extra-draw") && i + 1 < argc) extra_draw = atoi (argv[++i]);
        else if (!strcmp (argv[i], "--no-checkheap")) checkheap = 0;
        else if (!strcmp (argv[i], "--no-dump")) nodump = 1;
        else if (!strcmp (argv[i], "--render") && i + 1 < argc) {
            i++;
            if (!strcmp (argv[i], "every")) render_mode = 1;
            else if (!strcmp (argv[i], "none")) render_mode = 0;
            else { fprintf (stderr, "--render every|none\n"); return 2; }
        }
        else if (!strcmp (argv[i], "--screenblocks") && i + 1 < argc) screenblocks = atoi (argv[++i]);
        else if (!strcmp (argv[i], "--detail") && i + 1 < argc) detailLevel = atoi (argv[++i]);
        else if (!strcmp (argv[i], "--frames") && i + 1 < argc) frames_path = argv[++i];
        else if (!strcmp (argv[i], "--view") && i + 1 < argc) view_path = argv[++i];
        else if (!strcmp (argv[i], "--frames-raw") && i + 1 < argc) raw_dir = argv[++i];
        else if (!strcmp (argv[i], "--no-mask-mapped")) mask_mapped = 0;
        else if (!strcmp (argv[i], "--no-mask-rnd")) mask_rnd = 0;
        else if (!strcmp (argv[i], "--no-hu-record")) hu_record = 0;
        else if (!strcmp (argv[i], "--selftest-message") && i + 1 < argc) selftest_msg_tic = atoi (argv[++i]);
        else if (!strcmp (argv[i], "--selftest-rnd") && i + 1 < argc) selftest_rnd_tic = atoi (argv[++i]);
        else if (!strcmp (argv[i], "--bespoke-keys") && i + 2 < argc) {
            if (nbespoke == M5B_MAXBESPOKE) { fprintf (stderr, "too many --bespoke-keys\n"); return 2; }
            bespoke_tic[nbespoke] = atoi (argv[++i]); bespoke_text[nbespoke++] = argv[++i];
        }
        else if (!strcmp (argv[i], "--diag-fill") && i + 1 < argc) diag_fill = atoi (argv[++i]);
        else if (!strcmp (argv[i], "--lump-trace") && i + 1 < argc) lump_trace_path = argv[++i];
        else if (!strcmp (argv[i], "--selftest-bar") && i + 3 < argc) { selftest_bar_tic = atoi (argv[++i]); selftest_bar_x = atoi (argv[++i]); selftest_bar_y = atoi (argv[++i]); }
        else if (!strcmp (argv[i], "--selftest-palette") && i + 1 < argc) selftest_pal_tic = atoi (argv[++i]);
        else if (!strcmp (argv[i], "--selftest-pixel") && i + 3 < argc) {
            selftest_tic = atoi (argv[++i]); selftest_x = atoi (argv[++i]); selftest_y = atoi (argv[++i]);
            if (selftest_x < 0 || selftest_x >= SCREENWIDTH || selftest_y < 0 || selftest_y >= SCREENHEIGHT) { fprintf (stderr, "--selftest-pixel X Y\n"); return 2; }
        }
        else { fprintf (stderr, "usage: crunner --wad IWAD --wad DEMO.lmp --demo NAME --out DIR [--zone-mb N] "
                                "[--selftest-extra-draw TIC] [--no-checkheap] [--no-dump] [--render every|none] "
                                "[--screenblocks 3..11] [--detail 0] [--frames FILE] [--view FILE] [--frames-raw DIR] "
                                "[--no-mask-mapped] [--selftest-pixel TIC X Y] [--no-mask-rnd] [--no-hu-record] [--diag-fill N] "
                                "[--lump-trace FILE] [--selftest-bar TIC X Y] [--selftest-palette TIC] [--selftest-message TIC] "
                                "[--selftest-rnd TIC] [--bespoke-keys TIC TEXT]\n"); return 2; }
    }
    if (!numwads || !demo || (!out && !nodump)) { fprintf (stderr, "missing --wad/--demo/--out\n"); return 2; }
    if (layout_patchcount_offset () != 20 || layout_patches_offset () != 22 || layout_mappatch_size () != 10) {
        fprintf (stderr, "maptexture_t layout check FAILED (%d %d %d)\n", layout_patchcount_offset (),
                 layout_patches_offset (), layout_mappatch_size ());
        return 5;
    }
    if (detailLevel != 0) {                      /* decision R2: low detail is refused */
        fprintf (stderr, "configuration refused: detail %d (low detail is refused, decision R2)\n", detailLevel);
        return 8;
    }
    if (screenblocks < 3 || screenblocks > 11) { fprintf (stderr, "--screenblocks 3..11\n"); return 2; }
#ifndef M5A_TABLE_ADAPTERS
    if ((void*)(finetangent + FINEANGLES/2) != (void*)finesine) {
        fprintf (stderr, "table layout check FAILED: finesine does not follow finetangent (tables.c:67, 584)\n");
        return 9;
    }
#endif
    setvbuf (stdout, NULL, _IOLBF, 0);
    if (frames_path && !(frames_out = fopen (frames_path, "w"))) { perror (frames_path); return 4; }
    if (view_path && !(view_out = fopen (view_path, "w"))) { perror (view_path); return 4; }
    if (frames_out) { fputs (FRAME_COLUMNS, frames_out); fprintf (frames_out, "#config,screenblocks,%d,detail,%d,render,%s\n",
                                                                   screenblocks, detailLevel, render_mode ? "every" : "none"); }
    if (view_out) fputs (VIEW_COLUMNS, view_out);
    stub_error_hook = on_error;
    if (lump_trace_path && !(lump_trace = fopen (lump_trace_path, "w"))) { perror (lump_trace_path); return 4; }
    V_Init ();                                   /* d_main.c:1012 */
    if (diag_fill >= 0) memset (screens[0], diag_fill, SCREENWIDTH*SCREENHEIGHT);   /* --diag-fill: diagnostic only (finds never-written pixels) */
    Z_Init ();                                   /* d_main.c:1018 */
    W_InitMultipleFiles (wadfiles);              /* d_main.c:1021 */
    gamemode = registered;                       /* d_main.c:697-701 (doom.wad) */
    R_Init ();                                   /* d_main.c:1095 */
    P_InitSwitchList ();                         /* p_setup.c:702 */
    P_InitPicAnims ();                           /* p_setup.c:703 */
    { static char* names[NUMSPRITES + 1];        /* p_setup.c:704 with an explicit NULL terminator (R11) */
      for (i = 0; i < NUMSPRITES; i++) names[i] = sprnames[i];
      names[NUMSPRITES] = NULL;
      R_InitSprites (names); }
    consoleplayer = displayplayer = 0;           /* D_CheckNetGame, d_net.c:567-592 */
    playeringame[0] = true;
    stub_palette_hook = on_palette;
    HU_Init ();                                  /* d_main.c:1110 */
    ST_Init ();                                  /* d_main.c:1113 */
    singledemo = true;                           /* d_main.c:1137 */
    command_source_begin (demo);                 /* the demo through the genuine -playdemo path */
    if (gamestate != GS_LEVEL || !demoplayback) { fprintf (stderr, "level did not load\n"); return 6; }
    /* M5b: R_ExecuteSetViewSize runs in the first display (setsizeneeded from R_Init, d_main.c:215-217) */
    mapped_before = calloc (numlines ? numlines : 1, 1);
    linesel = calloc (numlines ? numlines : 1, 1);
    for (i = 0; i < numlines; i++) linesel[i] = lines[i].special != 0;
    if (!nodump) dump (out, "load.txt", -1);
    if (view_out) write_view (-1);
    printf ("load: gamestate %d numthinkers %d prndindex %d\n", (int)gamestate, numth, prndindex);
    if (nodump) walk_thinkers ();
    for (tic = 0; ; tic++) {
        if (tic == extra_draw) P_Random ();      /* --selftest-extra-draw: test-only */
        t0 = now ();
        stub_quit_armed = 1;
        m5b_tic = tic;
        bespoke_events (tic);                    /* --bespoke-keys (bespoke scenario only; D_ProcessEvents, d_main.c:378) */
        command_before_tic (tic);
        if (setjmp (stub_quit_jmp) == 0) G_Ticker ();
        stub_quit_armed = 0;
        if (stub_quit_flag) break;
        if (attacker_refused) break;             /* S17: the face would read a removed attacker; not dumped */
        gametic++;
        tick_time += now () - t0;
        if (checkheap) Z_CheckHeap ();           /* driver check; read-only walk of the zone */
        if (!nodump) {
            char name[64];
            t0 = now ();
            snprintf (name, sizeof name, "tic_%05d.txt", tic);
            dump (out, name, tic);
            dump_time += now () - t0;
        }
        if (view_out) { write_view (tic); fflush (view_out); }
        if (render_mode) m5b_display (tic);
        if (refused) break;                      /* a limit witness wrote past a renderer array (R3) */
        if (gameaction == ga_completed || gameaction == ga_victory || gameaction == ga_worlddone) {
            exited = 1;                          /* the next G_Ticker would enter the intermission */
            break;
        }
        for (i = 0; i < MAXPLAYERS; i++)
            if (playeringame[i] && players[i].playerstate == PST_REBORN) reborn = 1;
        if (reborn) break;                       /* the next G_Ticker would reload the level */
    }
    if (attacker_refused) refused = 1;
    ran = (exited || reborn || refused) && !attacker_refused ? tic + 1 : tic;   /* tics whose G_Ticker completed and were dumped */
    printf ("summary: tics %d end=%s end_tic %d gamestate %d prndindex %d checkheap %s zone_mb %d "
            "observer_misses %d unknown_messages %d tick_seconds %.3f dump_seconds %.3f us_per_tic %.1f\n",
            ran, refused ? "refused" : exited ? "exit" : reborn ? "reborn" : "marker", tic, (int)gamestate, prndindex,
            checkheap ? "every-tic-ok" : "load-only", stub_zone_mb, observer_misses, unknown_messages,
            tick_time, dump_time, ran ? tick_time * 1e6 / ran : 0.0);
    printf ("render: frames %d flagged_frames %d flagged_columns %d origins %d render %s screenblocks %d witness %s\n",
            frames_rendered, frames_flagged, columns_flagged, norigins, render_mode ? "every" : "none", screenblocks, WITNESS[witness]);
    m5b_finish ();
    if (frames_out) fclose (frames_out);
    if (view_out) fclose (view_out);
    return 0;
}
