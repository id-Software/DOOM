/*
 * M5b input driver (contract DOOM-M5B-INPUT-v1): declared stubs for symbols of files the input
 * driver does not compile. Every stub line: STUB <symbol> [io|reads|writes|var|adapter] <vanilla
 * file:lines>: <reason>. The functions the input path never reaches are declared one by one in
 * m5b_input_unreached.c, where each aborts. tools/reference/m5b_input_build.py lists every STUB
 * line in tests/reference/m5b_input_contract.json and refuses a build whose defined symbols
 * differ from the declared ones.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>
#include <stdarg.h>
#include "doomdef.h"
#include "doomstat.h"
#include "d_net.h"
#include "r_defs.h"
#include "doomdata.h"
#include "z_zone.h"
#include "st_stuff.h"
#include "hu_stuff.h"

/* driver hooks (not vanilla symbols; prefix stub_) */
jmp_buf stub_jmp; int stub_armed; char stub_msg[256]; int stub_end;   /* 1 I_Error, 2 I_Quit */
int stub_time;                 /* I_GetTime value, advanced by the driver once per tic */
int stub_sounds;               /* S_StartSound calls (menu/status feedback) */
int stub_givepower = -1;       /* last P_GivePower power */
int stub_setblocks = -1, stub_setdetail = -1;
mobj_t stub_mo;                /* the console player's mobj */

/* STUB I_BaseTiccmd [reads] i_system.c:64-68: genuine body (returns the zeroed emptycmd) */
ticcmd_t emptycmd_stub;
ticcmd_t* I_BaseTiccmd (void) { return &emptycmd_stub; }
/* STUB I_GetTime [io] i_system.c:88: wall clock replaced by a counter advanced once per tic (read by M_Responder's mouse/joystick pacing, m_menu.c:1362-1399, and G_DoLoadLevel's starttime) */
int I_GetTime (void) { return stub_time; }
/* STUB I_Error [io] i_system.c:162: records the message and returns to the driver (vanilla exits) */
void I_Error (char *error, ...)
{
    va_list ap; va_start (ap, error); vsnprintf (stub_msg, sizeof stub_msg, error, ap); va_end (ap);
    stub_end = 1;
    if (stub_armed) longjmp (stub_jmp, 1);
    fprintf (stderr, "I_Error: %s\n", stub_msg); exit (2);
}
/* STUB I_Quit [io] i_system.c:116: records and returns to the driver (vanilla exits) */
void I_Quit (void)
{
    strcpy (stub_msg, "I_Quit"); stub_end = 2;
    if (stub_armed) longjmp (stub_jmp, 1);
    exit (2);
}
/* STUB I_WaitVBL [io] i_system.c:126: a sleep */
void I_WaitVBL (int count) { (void) count; }

/* STUB Z_Malloc [io] z_zone.c:184: calloc with a memblock_t header carrying ZONEID (z_zone.h:58-66), so the
   Z_ChangeTag macro's check (z_zone.h:72-77) passes as on zone blocks; sets *user like the zone; presentation and demo buffers only */
void* Z_Malloc (int size, int tag, void *user)
{
    memblock_t *b = calloc (1, sizeof (memblock_t) + (size > 0 ? size : 1));
    if (!b) { fprintf (stderr, "calloc failed\n"); exit (4); }
    b->id = 0x1d4a11; b->tag = tag; b->size = size; b->user = user;
    if (user) *(void**) user = b + 1;
    return b + 1;
}
/* STUB Z_Free [io] z_zone.c:122: releases a Z_Malloc block of this file (free of the block with its header) */
void Z_Free (void *p) { free ((memblock_t *) p - 1); }
/* STUB Z_ChangeTag2 [io] z_zone.c:429: purge levels are not modelled */
void Z_ChangeTag2 (void *p, int tag) { (void) p; (void) tag; }
/* STUB Z_CheckHeap [io] z_zone.c:399: no zone heap exists; G_DoLoadLevel calls it (g_game.c:486) */
void Z_CheckHeap (void) { }

/* STUB W_CacheLumpName [io] w_wad.c:508: a zero-filled dummy lump (a patch of width and height 0); responders only store these pointers, drawers are never called */
static struct { memblock_t h; int lump[16384]; } dummy = { { 0, 0, 101, 0x1d4a11, 0, 0 }, { 0 } };
#define dummy_lump ((void *) dummy.lump)
void* W_CacheLumpName (char *name, int tag) { (void) name; (void) tag; return dummy_lump; }
/* STUB W_CacheLumpNum [io] w_wad.c:476: as W_CacheLumpName */
void* W_CacheLumpNum (int lump, int tag) { (void) lump; (void) tag; return dummy_lump; }
/* STUB W_GetNumForName [io] w_wad.c:399: lump 0 */
int W_GetNumForName (char *name) { (void) name; return 0; }
/* STUB W_CheckNumForName [io] w_wad.c:351: lump 0 (D_ProcessEvents asks only when gamemode == commercial, d_main.c:166-168; the driver runs registered) */
int W_CheckNumForName (char *name) { (void) name; return 0; }

/* STUB S_StartSound [io] s_sound.c:398: counted (the menu and the status bar announce actions with sounds) */
void S_StartSound (void *o, int id) { (void) o; (void) id; stub_sounds++; }
/* STUB S_ChangeMusic [io] s_sound.c:650: music is not played */
void S_ChangeMusic (int m, int l) { (void) m; (void) l; }
/* STUB S_StartMusic [io] s_sound.c:644: music is not played */
void S_StartMusic (int m) { (void) m; }
/* STUB S_PauseSound [io] s_sound.c:497: sound is not played (G_Ticker pause, g_game.c:708-714) */
void S_PauseSound (void) { }
/* STUB S_ResumeSound [io] s_sound.c:506: sound is not played */
void S_ResumeSound (void) { }
/* STUB S_SetSfxVolume [io] s_sound.c:631: sound is not played (menu volume) */
void S_SetSfxVolume (int v) { (void) v; }
/* STUB S_SetMusicVolume [io] s_sound.c:616: sound is not played (menu volume) */
void S_SetMusicVolume (int v) { (void) v; }

/* STUB setsizeneeded [var] r_main.c:652: written by the R_SetViewSize stub */
boolean setsizeneeded;
/* STUB R_SetViewSize [writes] r_main.c:657-666: genuine effect recorded (setsizeneeded, setblocks, setdetail); the view is never executed */
void R_SetViewSize (int blocks, int detail) { setsizeneeded = true; stub_setblocks = blocks; stub_setdetail = detail; }
/* STUB R_FlatNumForName [reads] r_data.c:672: G_DoLoadLevel's sky flat (g_game.c:466) */
int R_FlatNumForName (char *name) { (void) name; return 0; }
/* STUB R_TextureNumForName [reads] r_data.c:718: commercial sky only (g_game.c:470-480) */
int R_TextureNumForName (char *name) { (void) name; return 0; }

/* STUB P_SetupLevel [adapter] p_setup.c:584: no map; for the console player does what P_SpawnPlayer does that the input path
   and the responders read (p_mobj.c:640-699): G_PlayerReborn on PST_REBORN (p_mobj.c:651-652), the mobj link, PST_LIVE, then
   ST_Start and HU_Start (p_mobj.c:694-699). P_SetupPsprites is not run (no weapon states without the simulation). */
void G_PlayerReborn (int player);
void P_SetupLevel (int episode, int map, int playermask, skill_t skill)
{
    player_t *p = &players[consoleplayer];
    (void) episode; (void) map; (void) playermask; (void) skill;
    if (p->playerstate == PST_REBORN) G_PlayerReborn (consoleplayer);
    memset (&stub_mo, 0, sizeof stub_mo);
    stub_mo.player = p; stub_mo.health = 100;
    p->mo = &stub_mo; p->playerstate = PST_LIVE; p->health = 100;
    leveltime = 0;
    ST_Start ();
    HU_Start ();
}
/* STUB P_GivePower [writes] p_inter.c:290: the cheat's power-up recorded, not applied */
boolean P_GivePower (player_t *p, int power) { (void) p; stub_givepower = power; return true; }

/* variables of excluded files */
/* STUB bmaporgx [var] p_setup.c:93: 0 */
fixed_t bmaporgx;
/* STUB bmaporgy [var] p_setup.c:94: 0 */
fixed_t bmaporgy;
/* STUB deathmatch_p [var] p_setup.c:106: unused (no deathmatch) */
mapthing_t* deathmatch_p;
/* STUB deathmatchstarts [var] p_setup.c:105: unused */
mapthing_t deathmatchstarts[10];
/* STUB finecosine [var] r_main.c:113: &finesine[FINEANGLES/4] as vanilla */
fixed_t* finecosine = &finesine[FINEANGLES/4];
/* STUB gammatable [var] v_video.c:51: zero (only I_SetPalette's PseudoColor path reads it, never taken: the fake visual class is 0) */
byte gammatable[5][256];
/* STUB leveltime [var] p_tick.c:34: advanced by the P_Ticker adapter, reset by the P_SetupLevel adapter */
int leveltime;
/* STUB lines [var] p_setup.c:76: the driver's map (am_map.c draws only) */
line_t* lines;
/* STUB maketic [var] d_net.c:71: advanced by the driver once per tic like NetUpdate (d_net.c:410) */
int maketic;
/* STUB maxammo [var] p_inter.c:58: genuine values (ST cheats) */
int maxammo[NUMAMMO] = {200, 50, 300, 50};
/* STUB mb_used [var] i_system.c:51: 6 (SNDSERV defaults row, m_misc.c:258) */
int mb_used = 6;
/* STUB netcmds [var] d_net.c:62: the command buffer G_BuildTiccmd writes and G_Ticker reads */
ticcmd_t netcmds[MAXPLAYERS][BACKUPTICS];
/* STUB numChannels [var] s_sound.c:129: defaults row m_misc.c:282 */
int numChannels;
/* STUB numlines [var] p_setup.c:75: 0 (no map) */
int numlines;
/* STUB numsectors [var] p_setup.c:69: 0 (no map) */
int numsectors;
/* STUB numvertexes [var] p_setup.c:63: the driver's map (AM_findMinMaxBoundaries, am_map.c:424-445) */
int numvertexes;
/* STUB playerstarts [var] p_setup.c:107: zero (no map) */
mapthing_t playerstarts[MAXPLAYERS];
/* STUB save_p [var] p_saveg.c:33: unused (no savegame) */
byte* save_p;
/* STUB scaledviewwidth [var] r_draw.c:65: 0 (no view) */
int scaledviewwidth;
/* STUB screens [var] v_video.c:44: ST_Init allocates screens[4] (st_stuff.c ST_Init) */
byte* screens[5];
/* STUB sectors [var] p_setup.c:70: NULL (no map) */
sector_t* sectors;
/* STUB skyflatnum [var] r_sky.c:44: written by G_DoLoadLevel (g_game.c:466) */
int skyflatnum;
/* STUB skytexture [var] r_sky.c:47: written for commercial games only (g_game.c:470-480) */
int skytexture;
/* STUB snd_MusicVolume [var] s_sound.c:82: 15 (overwritten by the defaults table, m_misc.c:238) */
int snd_MusicVolume = 15;
/* STUB snd_SfxVolume [var] s_sound.c:79: 15 (overwritten by the defaults table, m_misc.c:237) */
int snd_SfxVolume = 15;
/* STUB sndserver_filename [var] i_sound.c:65: "./sndserver " (defaults row, m_misc.c:257) */
char* sndserver_filename = "./sndserver ";
/* STUB ticdup [var] d_net.c:74: 1 (single player, D_CheckNetGame) */
int ticdup = 1;
/* STUB usegamma [var] v_video.c:137: defaults row m_misc.c:286 */
int usegamma;
/* STUB vertexes [var] p_setup.c:64: the driver's map */
vertex_t* vertexes;
/* STUB viewheight [var] r_draw.c:66: 0 (no view) */
int viewheight;
/* STUB viewwidth [var] r_draw.c:64: 0 (no view) */
int viewwidth;
/* STUB viewwindowx [var] r_draw.c:67: 0 (no view) */
int viewwindowx;
/* STUB viewwindowy [var] r_draw.c:68: 0 (no view) */
int viewwindowy;
/* STUB P_Ticker [adapter] p_tick.c:130: the simulation is not run by the input driver (its subject is the event-to-command path; G_Ticker's command copy, demo round trip and special buttons are genuine) */
void P_Ticker (void) { if (paused) return; leveltime++; }
