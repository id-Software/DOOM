/*
 * M5b headless genuine-engine runner: declared stubs (adapters).
 *
 * The M5b fork of tools/reference/m5a_stubs.c (frozen; the M5a runner keeps building it). The
 * M5b runner compiles the genuine status bar and HUD whole (st_stuff.c, st_lib.c, hu_stuff.c,
 * hu_lib.c and m_cheat.c; tools/reference/m5b_build.py), so the eight status-bar and HUD stubs of
 * the M5a text are removed here (HU_Start, HU_Ticker, HU_Responder, HU_dequeueChatChar,
 * player_names, ST_Start, ST_Ticker, ST_Responder; the HU_Ticker stub's stub_hu_message and
 * stub_hu_ran now live in the driver, set by its HU_Ticker wrapper) and four are added at the end:
 * I_SetPalette (passes the palette pointer to stub_palette_hook, the driver's recorder),
 * showMessages and inhelpscreens (menu variables the genuine code reads), and S_ChangeMusic
 * (aborting). Everything else is the M5a text unchanged, including its header below.
 *
 * M5a headless genuine-engine runner: declared stubs (adapters).
 *
 * The M5a fork of tools/reference/m3h9c_stubs.c (frozen; the M3h9c, M4a and M4b runners keep
 * building it). The M5a runner compiles the genuine renderer whole (r_main.c, r_bsp.c,
 * r_segs.c, r_plane.c, r_things.c, r_draw.c, v_video.c; tools/reference/m5a_build.py), so the
 * seven stubs of renderer symbols are removed here (R_ExecuteSetViewSize, setsizeneeded,
 * R_FillBackScreen, R_InitSprites, sprites, numsprites, screens) and four are added: NetUpdate
 * and I_AllocLow, which the renderer calls, and screenblocks and detailLevel, which R_Init
 * reads. I_Error also calls stub_error_hook (set by the driver to write the message to the
 * frame stream) before it aborts. Everything else is the M3h9c text unchanged, including its
 * header below.
 *
 * M3h9c headless genuine-engine runner: declared stubs (adapters).
 *
 * Each symbol replaces a definition in a vanilla linuxdoom-1.10 file the runner does not
 * compile (platform, sound, HUD, status bar, automap, intermission, finale, menu, demo
 * loop, network, save game, renderer). Every stub is preceded by one machine-read line
 *   STUB <symbol> [<class>] <vanilla file:lines>: <reason>
 * (tools/reference/m3h9c_build.py parses these lines into the runner contract and checks
 * that the compiled file defines exactly these symbols). Classes:
 *   io      I/O or presentation only, or unreachable on the single-demo playback path
 *   reads   vanilla reads game state only and writes none; the stub does nothing
 *   writes  vanilla writes game-loop or presentation state; the reason says why P_Ticker
 *           cannot observe the omission
 *   var     a variable owned by an excluded file, with its value on this path
 * No stub writes gameplay state or draws from the gameplay RNG (P_Random). The vanilla
 * M_Random draws of S_StartSound (s_sound.c:330,340) and ST_Ticker (st_stuff.c:991) are
 * not reproduced: rndindex is presentation-only here (read by gameplay only in the
 * netgame consistancy check, g_game.c:692). Stubs for paths this runner must never reach
 * abort, so such a run cannot be mistaken for a modelled one.
 */
#include <stdio.h>
#include <stdlib.h>
#include <stdarg.h>
#include <setjmp.h>
#include <string.h>
#include "doomdef.h"
#include "doomstat.h"
#include "d_event.h"
#include "d_ticcmd.h"
#include "d_net.h"
#include "wi_stuff.h"
#include "r_defs.h"
#include "sounds.h"
#include "dstrings.h"

/* Runner control (not vanilla symbols; prefix stub_). */
int     stub_quit_flag;        /* set by I_Quit */
jmp_buf stub_quit_jmp;         /* the driver's setjmp around G_Ticker */
int     stub_quit_armed;
int     stub_zone_mb = 6;      /* i_system.c:51 `int mb_used = 6;` (driver option --zone-mb) */
int     stub_gettime_counter;

static void unreachable (const char* name)
{
    fprintf (stderr, "%s reached (not modelled by the runner)\n", name);
    abort ();
}

/* Runner control (M5a): the driver's frame-stream writer for I_Error (not a vanilla symbol). */
void (*stub_error_hook) (const char* message);
/* STUB I_Error [io] i_system.c:162-183: prints the message, shuts down and exits; the stub prints it, passes it to stub_error_hook and aborts. */
void I_Error (char *error, ...)
{
    va_list argptr;
    char message[512];
    va_start (argptr, error);
    vsnprintf (message, sizeof message, error, argptr);
    va_end (argptr);
    fprintf (stderr, "Error: %s\n", message);
    if (stub_error_hook) stub_error_hook (message);
    fflush (stderr);
    fflush (stdout);
    abort ();
}
/* STUB I_Quit [writes] i_system.c:116-124: exit(0) after shutdowns, reached from G_CheckDemoStatus at the demo marker (g_game.c:1660-1661); vanilla never returns, so the stub sets stub_quit_flag and longjmps to the driver and no later statement of G_CheckDemoStatus or G_Ticker runs. */
void I_Quit (void)
{
    stub_quit_flag = 1;
    if (stub_quit_armed) longjmp (stub_quit_jmp, 1);
    fprintf (stderr, "I_Quit outside G_Ticker\n");
    exit (3);
}
/* STUB I_GetTime [writes] i_system.c:88-100: wall-clock tic counter; the result is stored in starttime (g_game.c:486) and read only by the timingdemo branch (g_game.c:1653-1655), never by p_*.c; the stub is a deterministic counter. */
int I_GetTime (void) { return ++stub_gettime_counter; }
/* STUB I_Tactile [io] i_system.c:55-62: the vanilla body is empty. */
void I_Tactile (int on, int off, int total) { (void)on; (void)off; (void)total; }
/* STUB I_ZoneBase [io] i_system.c:76-80: *size = mb_used MiB and malloc; the zone size affects addresses only. */
byte* I_ZoneBase (int* size)
{
    *size = stub_zone_mb*1024*1024;
    return (byte *) malloc (*size);
}
/* STUB I_BaseTiccmd [io] i_system.c:65-68: zeroed static ticcmd, used only by G_BuildTiccmd (not called). */
static ticcmd_t emptycmd;
ticcmd_t* I_BaseTiccmd (void) { return &emptycmd; }

/* STUB S_Start [reads] s_sound.c:202-248: stops channels and changes music from gamemode/gameepisode/gamemap. */
void S_Start (void) {}
/* STUB S_StartSound [reads] s_sound.c:398-466: reads the origin and the listener; M_Random pitch draws only. */
void S_StartSound (void* origin, int sound_id) { (void)origin; (void)sound_id; }
/* STUB S_StopSound [reads] s_sound.c:471-484: stops the channel playing origin. */
void S_StopSound (void* origin) { (void)origin; }
/* STUB S_PauseSound [io] s_sound.c:497-504: music pause. */
void S_PauseSound (void) {}
/* STUB S_ResumeSound [io] s_sound.c:506-513: music resume. */
void S_ResumeSound (void) {}

/* STUB AM_Stop [writes] am_map.c:560-568: automapactive = false, as vanilla; only called when the automap is active, which it never is headless. */
void AM_Stop (void) { automapactive = false; }
/* STUB AM_Ticker [io] am_map.c:805-827: returns at once when !automapactive. */
void AM_Ticker (void) {}
/* STUB AM_Responder [io] am_map.c:614: event responder. */
boolean AM_Responder (event_t* ev) { (void)ev; return false; }
/* STUB automapactive [var] am_map.c:221: false. */
boolean automapactive = false;

/* STUB WI_Start [writes] wi_stuff.c:1838-1850: edits wminfo after a level exit (wi_stuff.c:1824-1835); post-exit only, aborts. */
void WI_Start (wbstartstruct_t* wbstartstruct) { (void)wbstartstruct; unreachable ("WI_Start"); }
/* STUB WI_Ticker [writes] wi_stuff.c:1503-1536: intermission (G_WorldDone); post-exit only, aborts. */
void WI_Ticker (void) { unreachable ("WI_Ticker"); }

/* STUB F_StartFinale [writes] f_finale.c:96-191: writes gameaction and gamestate; E1M8 victory only, aborts. */
void F_StartFinale (void) { unreachable ("F_StartFinale"); }
/* STUB F_Ticker [io] f_finale.c:207: GS_FINALE only, aborts. */
void F_Ticker (void) { unreachable ("F_Ticker"); }
/* STUB F_Responder [io] f_finale.c:195: event responder. */
boolean F_Responder (event_t* event) { (void)event; return false; }

/* STUB D_AdvanceDemo [writes] d_main.c:444-447: advancedemo = true; unreachable with singledemo (I_Quit comes first, g_game.c:1660), aborts. */
void D_AdvanceDemo (void) { unreachable ("D_AdvanceDemo"); }
/* STUB D_PageTicker [io] d_main.c:423-427: GS_DEMOSCREEN only, aborts. */
void D_PageTicker (void) { unreachable ("D_PageTicker"); }
/* STUB nomonsters [var] d_main.c:96: written from the demo header (g_game.c:1602). */
boolean nomonsters;
/* STUB respawnparm [var] d_main.c:97: written from the demo header (g_game.c:1600). */
boolean respawnparm;
/* STUB fastparm [var] d_main.c:98: written from the demo header (g_game.c:1601). */
boolean fastparm;
/* STUB singletics [var] d_main.c:102: false. */
boolean singletics = false;
/* STUB wipegamestate [var] d_main.c:188: GS_DEMOSCREEN (written by g_game.c:472-473). */
gamestate_t wipegamestate = GS_DEMOSCREEN;

/* STUB netcmds [var] d_net.c:62: zero; single-player playback overwrites the four ticcmd fields from the demo (g_game.c:664-667). */
ticcmd_t netcmds[MAXPLAYERS][BACKUPTICS];
/* STUB maketic [var] d_net.c:71: zero. */
int maketic;
/* STUB ticdup [var] d_net.c:74: 1 as D_CheckNetGame sets it (d_net.c:567-592); divisor at g_game.c:656. */
int ticdup = 1;

/* STUB M_StartControlPanel [io] m_menu.c:1723: opens the menu (events only). */
void M_StartControlPanel (void) {}
/* STUB menuactive [var] m_menu.c:126: false (P_Ticker's pause test, p_tick.c:139-145). */
boolean menuactive = false;
/* STUB mouseSensitivity [var] m_menu.c:76: read only by G_BuildTiccmd. */
int mouseSensitivity;

/* STUB M_ReadFile [io] m_misc.c:140-163: load-game path, aborts. */
int M_ReadFile (char const* name, byte** buffer) { (void)name; (void)buffer; unreachable ("M_ReadFile"); return 0; }
/* STUB M_WriteFile [io] m_misc.c:113-133: save-game and demo-recording paths, aborts. */
boolean M_WriteFile (char const* name, void* source, int length) { (void)name; (void)source; (void)length; unreachable ("M_WriteFile"); return false; }
/* STUB M_ScreenShot [io] m_misc.c:503: ga_screenshot only, aborts. */
void M_ScreenShot (void) { unreachable ("M_ScreenShot"); }

/* STUB P_ArchivePlayers [io] p_saveg.c:47: save game only (p_saveg.c is excluded), aborts. */
void P_ArchivePlayers (void) { unreachable ("P_ArchivePlayers"); }
/* STUB P_ArchiveWorld [io] p_saveg.c:114: save game only, aborts. */
void P_ArchiveWorld (void) { unreachable ("P_ArchiveWorld"); }
/* STUB P_ArchiveThinkers [io] p_saveg.c:232: save game only, aborts. */
void P_ArchiveThinkers (void) { unreachable ("P_ArchiveThinkers"); }
/* STUB P_ArchiveSpecials [io] p_saveg.c:355: save game only, aborts. */
void P_ArchiveSpecials (void) { unreachable ("P_ArchiveSpecials"); }
/* STUB P_UnArchivePlayers [io] p_saveg.c:79: load game only, aborts. */
void P_UnArchivePlayers (void) { unreachable ("P_UnArchivePlayers"); }
/* STUB P_UnArchiveWorld [io] p_saveg.c:167: load game only, aborts. */
void P_UnArchiveWorld (void) { unreachable ("P_UnArchiveWorld"); }
/* STUB P_UnArchiveThinkers [io] p_saveg.c:266: load game only, aborts. */
void P_UnArchiveThinkers (void) { unreachable ("P_UnArchiveThinkers"); }
/* STUB P_UnArchiveSpecials [io] p_saveg.c:475: load game only, aborts. */
void P_UnArchiveSpecials (void) { unreachable ("P_UnArchiveSpecials"); }
/* STUB save_p [var] p_saveg.c:35: unused on this path. */
byte* save_p;

/* STUB NetUpdate [io] d_net.c:368-470: builds local ticcmds from events (none headless) and polls the network; called four times by R_RenderPlayerView (r_main.c:881-897); single-player demo playback overwrites the console player's ticcmd from the demo (g_game.c:664-667). */
void NetUpdate (void) {}
/* STUB I_AllocLow [io] i_system.c:147-154: malloc and memset 0, as vanilla; V_Init allocates the four screens with it (v_video.c:489). */
byte* I_AllocLow (int length) { byte* m = (byte*)malloc (length); memset (m, 0, length); return m; }
/* STUB screenblocks [var] m_menu.c:84: the view size R_Init passes to R_SetViewSize (r_main.c:783; default 9, m_misc.c:279); the driver sets it from --screenblocks. */
int screenblocks = 9;
/* STUB detailLevel [var] m_menu.c:83: the detail R_Init passes to R_SetViewSize (default 0, m_misc.c:280); the driver accepts 0 only (low detail is refused, decision R2). */
int detailLevel = 0;

/* Runner control (M5b): the driver's palette recorder (not a vanilla symbol). */
void (*stub_palette_hook) (byte* palette);
/* STUB I_SetPalette [io] i_video.c:582-585: uploads the 768 bytes to the video hardware (UploadNewPalette); the stub passes the pointer to stub_palette_hook, which records which of the 14 PLAYPAL palettes it is. */
void I_SetPalette (byte* palette) { if (stub_palette_hook) stub_palette_hook (palette); }
/* STUB showMessages [var] m_menu.c:79: 1, the default of m_misc.c:239 ("show_messages"); read by HU_Ticker (hu_stuff.c:519). */
int showMessages = 1;
/* STUB inhelpscreens [var] m_menu.c:125: false (no menu); read by the driver's D_Display mirror (d_main.c:244, 299). */
boolean inhelpscreens = false;
/* STUB S_ChangeMusic [io] s_sound.c:650-686: referenced only by ST_Responder's music cheat (st_stuff.c:611, 620); the runner posts key events only for the bespoke scenario's cheats iddqd and idbeholds, so it aborts. */
void S_ChangeMusic (int musicnum, int looping) { (void)musicnum; (void)looping; unreachable ("S_ChangeMusic"); }
