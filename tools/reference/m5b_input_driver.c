/*
 * M5b input driver (contract DOOM-M5B-INPUT-v1, tools/reference/m5b_input_build.py): a headless
 * driver of the genuine linuxdoom-1.10 input path. Every line number cites linuxdoom-1.10 at
 * a77dfb96. Design and decisions: docs/M5B_REFERENCE_INVESTIGATION.md section 2.2 and S3-S7,
 * S11, S22; the golden format is specified in tests/reference/m5b_input/README.md.
 *
 * Start-up mirrors D_DoomMain for what the input path reads (d_main.c:1015, 1092, 1110, 1113):
 * M_LoadDefaults (with no `config set` line there is no -config and basedefault is "", so
 * fopen ("") fails and the values are the genuine table of m_misc.c:236-297; with `config set`
 * lines the driver writes them to m5b_input.cfg in the working directory and passes
 * `-config m5b_input.cfg`, which the genuine M_LoadDefaults reads, m_misc.c:357-399), M_Init,
 * HU_Init, ST_Init; then, with recording on (the default), G_RecordDemo ("m5binput"),
 * g_game.c:1530-1545, G_InitNew (sk_medium, 1, 1) as the -warp/-record autostart does, and
 * G_BeginRecording (d_main.c:356-357). P_SetupLevel and P_Ticker are declared adapters
 * (m5b_input_stubs.c).
 *
 * Each tic is the singletics branch of D_DoomLoop (d_main.c:376-386), with the scripted X
 * events queued in the fake X server (m5b_fakex.c) first:
 *   I_StartTic; D_ProcessEvents; G_BuildTiccmd (&netcmds[consoleplayer][maketic%BACKUPTICS]);
 *   if (advancedemo) D_DoAdvanceDemo; M_Ticker; G_Ticker; gametic++; maketic++;
 * G_Ticker is genuine (the copy into players[].cmd, the G_WriteDemoTiccmd write and re-read,
 * g_game.c:664-669, the special buttons and the ST, AM and HU tickers); only the simulation
 * (P_Ticker) is replaced.
 *
 * Script (one directive per line, '#' comments; config lines only before every other line):
 *   config grab 0|1          i_video.c grabMouse (vanilla: -grabmouse, i_video.c:738)
 *   config warpmodel 0|1     the fake server answers a moving XWarpPointer with a MotionNotify
 *   config record 0|1        recording is on by default
 *   config set NAME VALUE    one row of the configuration file read by M_LoadDefaults
 *   press K | release K | tap K       X KeyPress / KeyRelease / both, queued for the next tic
 *   bpress B | brelease B | motion X Y | expose       X button, pointer and Expose events
 *   ev keydown N | ev keyup N | ev mouse BUTTONS DX DY   event_t level: D_PostEvent now
 *   tic [N]                  run N tics (default 1); queued X events are delivered in the first
 *   loadlevel                G_DoLoadLevel () now (between tics), as G_Ticker does for ga_loadlevel
 *   enddemo                  G_CheckDemoStatus () now (g_game.c:1676-1683), as 'q' does; the run ends
 * K: a keysym name of the table below, a single printable character, or 0xNNNN.
 *
 * Golden rows (written to --out FILE; README beside the goldens):
 *   h HEX                    the 13 demo header bytes after G_BeginRecording (recording only)
 *   s TIC STATE              the observed state vector at start (TIC -1) and whenever it changes
 *   L TIC                    a loadlevel directive ran before tic TIC
 *   e TIC TYPE D1 D2 D3 WHO  an event delivered by D_ProcessEvents in tic TIC, in order
 *   q TIC POSTED DELIVERED   only when the counts differ (events lost to the MAXEVENTS ring)
 *   t TIC F S A B XF XS XA XB DEMO TURNHELD DCT DCS DCL DCT2 DCS2 DCL2 MB KEYS
 *   r N                      the previous t row repeats N more times, one tic each
 *   c TIC C H XC XH          consistancy and chatchar, only when one of them is nonzero
 *   x TIC KIND MESSAGE       I_Error or I_Quit; the run stops
 *   z TIC DEMOLEN            end of the script: the next tic number and the demo bytes written
 *
 * Test-only options (never change genuine code text): --selftest-cmd TIC flips bit 0 of the built
 * forwardmove of tic TIC after G_BuildTiccmd; --selftest-drop-event N does not post the N-th
 * posted event (0-based, counted over the run) and reports its tic on stderr.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>
#include <stdarg.h>
#include <X11/Xlib.h>
#include "doomdef.h"
#include "doomstat.h"
#include "d_event.h"
#include "d_main.h"
#include "d_net.h"
#include "g_game.h"
#include "m_menu.h"
#include "m_misc.h"
#include "m_argv.h"
#include "hu_stuff.h"
#include "st_stuff.h"
#include "am_map.h"
#include "i_video.h"
#include "r_defs.h"
#include "r_state.h"

/* genuine globals read by the driver */
extern Display *X_display; extern int X_width, X_height; extern boolean grabMouse;
extern boolean automapactive, menuactive, chat_on, viewactive, sendpause, sendsave, paused, advancedemo, usergame;
extern boolean demorecording; extern byte *demo_p, *demobuffer, *demoend;
extern int messageToPrint, saveStringEnter, showMessages, usegamma, mouseSensitivity;
extern int turnheld, dclicktime, dclickstate, dclicks, dclicktime2, dclickstate2, dclicks2;
extern boolean gamekeydown[256]; extern boolean mousearray[4];
extern int eventhead, eventtail;
extern ticcmd_t netcmds[MAXPLAYERS][BACKUPTICS];
extern int maketic;
/* fake X (m5b_fakex.c) */
extern struct FakeDisplay fake_display_object; extern int fake_warp_model, fake_warps, fake_drained;
void fake_begin_tic (void); void fake_key (int, unsigned long); void fake_button (int, int); void fake_motion (int, int);
void fake_other (int);
/* stubs (m5b_input_stubs.c) */
extern jmp_buf stub_jmp; extern int stub_armed, stub_end, stub_time, stub_sounds, stub_givepower, stub_setblocks, stub_setdetail;
extern char stub_msg[256];

static FILE *out, *trace;
static int tic_no;
static int selftest_cmd = -1;
static long selftest_drop = -1, posted_total;

/* ---- golden output with run-length encoding of identical t rows ---- */
static char last_t[2048]; static int have_t, repeat;
static void flush_repeat (void) { if (repeat) { fprintf (out, "r %d\n", repeat); repeat = 0; } }
static void row (const char *fmt, ...) __attribute__ ((format (printf, 1, 2)));
static void row (const char *fmt, ...)
{
    va_list ap;
    flush_repeat (); have_t = 0;
    va_start (ap, fmt); vfprintf (out, fmt, ap); va_end (ap);
}
static void t_row (const char *body)
{
    if (have_t && !strcmp (body, last_t)) { repeat++; return; }
    flush_repeat ();
    fprintf (out, "t %d %s\n", tic_no, body);
    strcpy (last_t, body); have_t = 1;
}

/* ---- linker observers (-Wl,--wrap): log and call the genuine function ---- */
void __real_D_PostEvent (event_t *); boolean __real_M_Responder (event_t *); boolean __real_G_Responder (event_t *);
boolean __real_HU_Responder (event_t *); boolean __real_ST_Responder (event_t *); boolean __real_AM_Responder (event_t *);
static int posted_this_tic, delivered_this_tic;
static int hu_called, hu_r, st_r, am_r;

static void ev_text (char *b, size_t n, event_t *ev)
{
    /* key events from i_video.c:197-212 leave data2/data3 indeterminate (no consumer reads them): '-' */
    if (ev->type == ev_keydown || ev->type == ev_keyup) snprintf (b, n, "%d %d - -", ev->type, ev->data1);
    else snprintf (b, n, "%d %d %d %d", ev->type, ev->data1, ev->data2, ev->data3);
}

void __wrap_D_PostEvent (event_t *ev)
{
    char b[96];
    if (posted_total++ == selftest_drop)
    {
        fprintf (stderr, "selftest: event %ld not posted at tic %d\n", selftest_drop, tic_no);
        return;
    }
    posted_this_tic++;
    if (trace) { ev_text (b, sizeof b, ev); fprintf (trace, "P %d %s head=%d tail=%d\n", tic_no, b, eventhead, eventtail); }
    __real_D_PostEvent (ev);
}

static void delivered (event_t *ev, const char *who)
{
    char b[96];
    ev_text (b, sizeof b, ev);
    row ("e %d %s %s\n", tic_no, b, who);
    delivered_this_tic++;
}

boolean __wrap_M_Responder (event_t *ev)
{
    boolean r = __real_M_Responder (ev);
    if (trace) { char b[96]; ev_text (b, sizeof b, ev); fprintf (trace, "R %d M_Responder %s %d\n", tic_no, b, r); }
    if (r) delivered (ev, "M");          /* menu ate the event (d_main.c:173-174) */
    return r;
}

boolean __wrap_G_Responder (event_t *ev)
{
    boolean r;
    hu_called = hu_r = st_r = am_r = 0;
    r = __real_G_Responder (ev);
    if (trace) { char b[96]; ev_text (b, sizeof b, ev); fprintf (trace, "R %d G_Responder %s %d\n", tic_no, b, r); }
    /* g: returned before the HU/ST/AM chain (spy mode or the demo-screen menu, g_game.c:507-532);
       H, S, A: that responder ate it (g_game.c:543-548); G: reached the switch (g_game.c:557) */
    delivered (ev, !hu_called ? "g" : hu_r ? "H" : st_r ? "S" : am_r ? "A" : "G");
    return r;
}

#define OBS(name, flag) boolean __wrap_##name (event_t *ev) { boolean r = __real_##name (ev); flag; \
    if (trace) { char b[96]; ev_text (b, sizeof b, ev); fprintf (trace, "R %d %s %s %d\n", tic_no, #name, b, r); } return r; }
OBS (HU_Responder, (hu_called = 1, hu_r = r))
OBS (ST_Responder, st_r = r)
OBS (AM_Responder, am_r = r)

/* ---- script parsing ---- */
static struct { const char *n; unsigned long k; } names[] = {
    {"Left",0xff51},{"Up",0xff52},{"Right",0xff53},{"Down",0xff54},{"Escape",0xff1b},{"Return",0xff0d},{"Tab",0xff09},
    {"F1",0xffbe},{"F2",0xffbf},{"F3",0xffc0},{"F4",0xffc1},{"F5",0xffc2},{"F6",0xffc3},{"F7",0xffc4},{"F8",0xffc5},
    {"F9",0xffc6},{"F10",0xffc7},{"F11",0xffc8},{"F12",0xffc9},{"BackSpace",0xff08},{"Delete",0xffff},{"Pause",0xff13},
    {"equal",0x3d},{"minus",0x2d},{"KP_Equal",0xffbd},{"KP_Subtract",0xffad},{"Shift_L",0xffe1},{"Shift_R",0xffe2},
    {"Control_L",0xffe3},{"Control_R",0xffe4},{"Alt_L",0xffe9},{"Alt_R",0xffea},{"Meta_L",0xffe7},{"Meta_R",0xffe8},
    {"space",0x20},{"comma",0x2c},{"period",0x2e},{"Home",0xff50},{"KP_Enter",0xff8d},{"Caps_Lock",0xffe5},{"Super_L",0xffeb},
    {0,0}};

static void die (int line, const char *what, const char *s)
{
    fprintf (stderr, "line %d: %s %s\n", line, what, s ? s : "");
    exit (5);
}

static unsigned long keysym (const char *s, int line)
{
    int i;
    for (i = 0; names[i].n; i++) if (!strcmp (s, names[i].n)) return names[i].k;
    if (s[0] == '0' && s[1] == 'x') return strtoul (s, 0, 16);
    if (s[0] && !s[1]) return (unsigned char) s[0];
    die (line, "unknown key", s);
    return 0;
}

static long number (const char *s, int line)
{
    char *end; long v = strtol (s, &end, 0);
    if (!*s || *end) die (line, "not a number:", s);
    return v;
}

/* ---- observation ---- */
static int bits (boolean *a, int n) { int i, m = 0; for (i = 0; i < n; i++) if (a[i]) m |= 1 << i; return m; }
static char prev_state[1024];
static void dump_state (void)
{
    char s[1024];
    player_t *p = &players[consoleplayer];
    snprintf (s, sizeof s, "gamestate=%d,gameaction=%d,menuactive=%d,messageToPrint=%d,saveStringEnter=%d,automapactive=%d,"
              "viewactive=%d,chat_on=%d,paused=%d,sendpause=%d,sendsave=%d,setblocks=%d,setdetail=%d,showMessages=%d,usegamma=%d,"
              "cheats=%d,message=%s,givepower=%d,advancedemo=%d,demorecording=%d,usergame=%d,displayplayer=%d,sounds=%d,"
              "gamemap=%d,weapons=0x%x,cards=0x%x,armor=%d,ammo0=%d,readyweapon=%d,pendingweapon=%d",
              gamestate, gameaction, menuactive, messageToPrint, saveStringEnter, automapactive, viewactive, chat_on, paused,
              sendpause, sendsave, stub_setblocks, stub_setdetail, showMessages, usegamma, p->cheats,
              p->message ? "set" : "null", stub_givepower, advancedemo, demorecording, usergame, displayplayer, stub_sounds,
              gamemap, bits (p->weaponowned, NUMWEAPONS), bits (p->cards, NUMCARDS), p->armorpoints, p->ammo[0], p->readyweapon,
              p->pendingweapon);
    if (strcmp (s, prev_state)) { row ("s %d %s\n", tic_no, s); strcpy (prev_state, s); }
}

static int cap (int v, int c) { return v > c ? c : v; }

static void dump_tic (ticcmd_t *b, ticcmd_t *x, byte *p0, byte *p1)
{
    char body[2048], demo[16], keys[256 * 5 + 2];
    int i, n = 0;
    if (p0 && p1 == p0 + 4) snprintf (demo, sizeof demo, "%02x%02x%02x%02x", p0[0], p0[1], p0[2], p0[3]);
    else strcpy (demo, "-");
    keys[0] = 0;
    for (i = 0; i < 256; i++) if (gamekeydown[i]) n += snprintf (keys + n, sizeof keys - n, "%s%x", n ? "," : "", i);
    if (!n) strcpy (keys, "-");
    /* turnheld is shown capped at SLOWTURNTICS (6) and the double-click timers at 21: G_BuildTiccmd
       reads them only through turnheld < 6, dclicktime > 1 and dclicktime > 20 (g_game.c:262-275, 352, 372, 381, 388) */
    snprintf (body, sizeof body, "%d %d %d %d %d %d %d %d %s %d %d %d %d %d %d %d %d %s",
              b->forwardmove, b->sidemove, b->angleturn, b->buttons, x->forwardmove, x->sidemove, x->angleturn, x->buttons, demo,
              cap (turnheld, 6), cap (dclicktime, 21), dclickstate, dclicks, cap (dclicktime2, 21), dclickstate2, dclicks2,
              bits (mousearray, 4), keys);
    t_row (body);
    if (b->consistancy || b->chatchar || x->consistancy || x->chatchar)
        row ("c %d %d %d %d %d\n", tic_no, b->consistancy, b->chatchar, x->consistancy, x->chatchar);
}

/* ---- start-up and the tic ---- */
static vertex_t map_vertexes[4];
static int record = 1, started;
static char config_text[8192]; static int config_rows;

static void start (void)
{
    static char *argv0[] = {"m5b_input", 0}, *argv1[] = {"m5b_input", "-config", "m5b_input.cfg", 0};
    if (config_rows)
    {
        FILE *f = fopen ("m5b_input.cfg", "w");
        if (!f || fputs (config_text, f) < 0 || fclose (f)) { perror ("m5b_input.cfg"); exit (4); }
        myargc = 3; myargv = argv1;
    }
    else { myargc = 1; myargv = argv0; }
    gamemode = registered; language = english;
    M_LoadDefaults ();                      /* d_main.c:1015 */
    M_Init (); HU_Init (); ST_Init ();      /* d_main.c:1092, 1110, 1113 */
    /* a 4-vertex map for AM_findMinMaxBoundaries (am_map.c:424-445), presentation only */
    map_vertexes[1].x = 256 << FRACBITS; map_vertexes[2].x = 256 << FRACBITS;
    map_vertexes[2].y = 256 << FRACBITS; map_vertexes[3].y = 256 << FRACBITS;
    vertexes = map_vertexes; numvertexes = 4;
    playeringame[0] = true; consoleplayer = displayplayer = 0;
    netgame = false; deathmatch = 0; demoplayback = false; singledemo = false;
    X_display = (Display *) &fake_display_object; X_width = SCREENWIDTH; X_height = SCREENHEIGHT;
    if (record) G_RecordDemo ("m5binput");  /* g_game.c:1530 */
    G_InitNew (sk_medium, 1, 1);            /* usergame = true, gamestate = GS_LEVEL (g_game.c:1443, G_DoLoadLevel) */
    if (record)
    {
        G_BeginRecording ();                /* d_main.c:356-357 */
        row ("h %02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x\n", demobuffer[0], demobuffer[1], demobuffer[2],
             demobuffer[3], demobuffer[4], demobuffer[5], demobuffer[6], demobuffer[7], demobuffer[8], demobuffer[9],
             demobuffer[10], demobuffer[11], demobuffer[12]);
    }
    started = 1;
    tic_no = -1; dump_state ();
}

static int run_tic (void)
{
    ticcmd_t built;
    byte *p0;
    stub_armed = 1;
    if (setjmp (stub_jmp))
    {
        row ("x %d %s %s\n", tic_no, stub_end == 1 ? "I_Error" : "I_Quit", stub_msg);
        return 0;
    }
    fake_drained = 0;
    I_StartTic ();                          /* d_main.c:378 */
    D_ProcessEvents ();
    if (posted_this_tic != delivered_this_tic) row ("q %d %d %d\n", tic_no, posted_this_tic, delivered_this_tic);
    G_BuildTiccmd (&netcmds[consoleplayer][maketic%BACKUPTICS]);
    if (tic_no == selftest_cmd) netcmds[consoleplayer][maketic%BACKUPTICS].forwardmove ^= 1;
    built = netcmds[consoleplayer][maketic%BACKUPTICS];
    if (advancedemo) D_DoAdvanceDemo ();
    M_Ticker ();
    p0 = demorecording ? demo_p : 0;
    G_Ticker ();
    gametic++; maketic++;
    stub_armed = 0;
    if (trace) fprintf (trace, "Q %d xevents=%d posted=%d head=%d tail=%d warps=%d\n", tic_no, fake_drained, posted_this_tic,
                        eventhead, eventtail, fake_warps);
    dump_tic (&built, &players[consoleplayer].cmd, p0, demorecording ? demo_p : 0);
    dump_state ();
    stub_time++;
    posted_this_tic = delivered_this_tic = 0;
    return 1;
}

static int finish (int ended)
{
    if (!ended && started) row ("z %d %ld\n", tic_no, demobuffer && demorecording ? (long) (demo_p - demobuffer) : -1L);
    flush_repeat ();
    if (trace) fclose (trace);
    if (fclose (out)) { perror ("--out"); return 4; }
    return 0;
}

int main (int argc, char **argv)
{
    FILE *f; char line[256]; int ln = 0, i;
    const char *outname = 0, *script = 0;
    for (i = 1; i < argc; i++)
    {
        if (!strcmp (argv[i], "--trace") && i + 1 < argc) { trace = fopen (argv[++i], "w"); if (!trace) { perror ("--trace"); return 1; } }
        else if (!strcmp (argv[i], "--out") && i + 1 < argc) outname = argv[++i];
        else if (!strcmp (argv[i], "--selftest-cmd") && i + 1 < argc) selftest_cmd = atoi (argv[++i]);
        else if (!strcmp (argv[i], "--selftest-drop-event") && i + 1 < argc) selftest_drop = atol (argv[++i]);
        else if (!script && argv[i][0] != '-') script = argv[i];
        else { fprintf (stderr, "unknown argument %s\n", argv[i]); return 1; }
    }
    if (!script || !outname)
    {
        fprintf (stderr, "usage: m5b_input SCRIPT --out FILE [--trace FILE] [--selftest-cmd TIC] [--selftest-drop-event N]\n");
        return 1;
    }
    f = fopen (script, "r"); if (!f) { perror (script); return 1; }
    out = fopen (outname, "w"); if (!out) { perror (outname); return 1; }
    tic_no = 0;
    while (fgets (line, sizeof line, f))
    {
        char a[64] = "", b[64] = "", c[64] = "", d[64] = "", e[64] = "";
        int n;
        ln++;
        if (!strchr (line, '\n') && !feof (f)) die (ln, "line too long", 0);
        if (line[0] == '#' || sscanf (line, "%63s %63s %63s %63s %63s", a, b, c, d, e) < 1) continue;
        if (!strcmp (a, "config"))
        {
            if (started) die (ln, "config after the first event or tic", 0);
            if (!strcmp (b, "grab")) grabMouse = number (c, ln);
            else if (!strcmp (b, "warpmodel")) fake_warp_model = number (c, ln);
            else if (!strcmp (b, "record")) record = number (c, ln);
            else if (!strcmp (b, "set") && c[0] && d[0] && !e[0])
            {
                if (d[0] == '"') die (ln, "string configuration values are refused:", c);
                size_t used = strlen (config_text);
                if (snprintf (config_text + used, sizeof config_text - used, "%s\t\t%s\n", c, d) >= (int) (sizeof config_text - used))
                    die (ln, "configuration too long", 0);
                config_rows++;
            }
            else die (ln, "unknown config", b);
            continue;
        }
        if (!started) { start (); tic_no = 0; fake_begin_tic (); }
        if (!strcmp (a, "press")) fake_key (1, keysym (b, ln));
        else if (!strcmp (a, "release")) fake_key (0, keysym (b, ln));
        else if (!strcmp (a, "tap")) { fake_key (1, keysym (b, ln)); fake_key (0, keysym (b, ln)); }
        else if (!strcmp (a, "bpress")) fake_button (1, number (b, ln));
        else if (!strcmp (a, "brelease")) fake_button (0, number (b, ln));
        else if (!strcmp (a, "motion")) fake_motion (number (b, ln), number (c, ln));
        else if (!strcmp (a, "expose")) fake_other (Expose);
        else if (!strcmp (a, "ev"))
        {
            event_t ev; memset (&ev, 0, sizeof ev);
            if (!strcmp (b, "keydown") && !d[0]) { ev.type = ev_keydown; ev.data1 = number (c, ln); }
            else if (!strcmp (b, "keyup") && !d[0]) { ev.type = ev_keyup; ev.data1 = number (c, ln); }
            else if (!strcmp (b, "mouse") && e[0])
            { ev.type = ev_mouse; ev.data1 = number (c, ln); ev.data2 = number (d, ln); ev.data3 = number (e, ln); }
            else die (ln, "bad ev line", b);
            D_PostEvent (&ev);
        }
        else if (!strcmp (a, "loadlevel"))
        {
            stub_armed = 0;
            G_DoLoadLevel ();
            row ("L %d\n", tic_no);
            dump_state ();
        }
        else if (!strcmp (a, "enddemo"))
        {
            stub_armed = 1;
            if (!setjmp (stub_jmp)) { G_CheckDemoStatus (); row ("x %d none not recording\n", tic_no); }
            else row ("x %d %s %s\n", tic_no, stub_end == 1 ? "I_Error" : "I_Quit", stub_msg);
            return finish (1);
        }
        else if (!strcmp (a, "tic"))
        {
            n = b[0] ? (int) number (b, ln) : 1;
            if (n < 1) die (ln, "tic count", b);
            while (n-- > 0)
            {
                if (!run_tic ()) return finish (1);
                tic_no++; fake_begin_tic ();
            }
        }
        else die (ln, "unknown directive", a);
    }
    if (!started) die (ln, "no tic", 0);
    return finish (0);
}
