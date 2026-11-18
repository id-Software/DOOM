/*
 * M5b step 7b: the command-injection mode of the genuine M5b runner (declared adapter; contract
 * tests/reference/m5b_inject_contract.json, schema DOOM-M5B-INJECT-v1; decision S3 (ii) of
 * docs/M5B_REFERENCE_INVESTIGATION.md, ruling of 2026-11-17).
 *
 * Why: a version-110 demo carries one byte of angleturn (G_WriteDemoTiccmd stores
 * (angleturn+128)>>8, g_game.c:1512; G_ReadDemoTiccmd reads it back as byte<<8, g_game.c:1500), so
 * demo playback can never execute a command whose angleturn has a nonzero low byte. A live game
 * that does not record executes the command G_BuildTiccmd built at full 16 bits: under -timedemo's
 * singletics loop G_BuildTiccmd writes it straight into netcmds[consoleplayer][maketic%BACKUPTICS]
 * (d_main.c:376-385, maketic == gametic), G_Ticker copies netcmds[i][buf] into players[i].cmd with
 * buf = (gametic/ticdup)%BACKUPTICS (g_game.c:656-664), neither demo branch runs when demoplayback
 * and demorecording are false (g_game.c:666-669), and P_MovePlayer applies cmd->angleturn<<16 whole
 * (p_user.c:148-154).
 *
 * How: this file compiles tools/reference/m5b_driver.c unchanged (its text, its contract and every
 * result that digests it stay as they are) with two object-like renames: its main becomes
 * m5b_runner_main, and its one call of G_Ticker (and the g_game.h prototype) becomes
 * m5b_inject_ticker. Without --commands the runner is the demo runner exactly: m5b_inject_ticker only
 * calls the genuine G_Ticker. With --commands FILE (one line "forward side angleturn buttons" per tic)
 * the level still loads through the runner's genuine G_DeferedPlayDemo/G_DoPlayDemo path from a
 * header-only demo lump (G_InitNew with the header's skill and map, g_game.c:1582-1619), so the load
 * dump is the demo mode's; then, before each genuine G_Ticker, demoplayback is cleared (the value
 * G_InitNew leaves in a live game, g_game.c:1445; its other readers are p_tick.c:141 with the menu,
 * never active here, G_Responder at g_game.c:522, which gets no events here, the precache test of
 * r_data.c:758, only at the load, M_NewGame at m_menu.c:875, never called, HSendPacket, HGetPacket,
 * D_QuitNetGame and TryRunTics at d_net.c:156, 205, 609 and 686, none of which runs (the runner calls
 * G_Ticker itself, and NetUpdate and I_Quit are the stubs), and G_CheckDemoStatus at g_game.c:1658,
 * never called, since the injection ends through I_Quit) and tic k's command is written to
 * netcmds[consoleplayer][(gametic/ticdup)%BACKUPTICS] in place of G_BuildTiccmd's output.
 * consistancy and chatchar are 0, as G_BuildTiccmd leaves them in single player (g_game.c:252-253,
 * where nothing writes consistancy outside a netgame, g_game.c:681; g_game.c:328 with the empty
 * chat queue, hu_stuff.c:600-614).
 * After the last command the next tic ends the run through I_Quit (the stub's longjmp), as the demo
 * marker does (g_game.c:1493-1496, 1660-1661): the summary says end=marker.
 */
int m5b_runner_main (int argc, char** argv);
#define main m5b_runner_main
#define G_Ticker m5b_inject_ticker
#include "m5b_driver.c"
#undef G_Ticker
#undef main

void G_Ticker (void);                            /* g_game.c:605 */
void I_Quit (void);                              /* the stub: longjmps to the driver */
static ticcmd_t* inject_cmd;                     /* NULL: the demo mode */
static int inject_count, inject_next;

void m5b_inject_ticker (void)
{
    if (inject_cmd) {
        if (inject_next == inject_count) I_Quit ();
        demoplayback = false;
        netcmds[consoleplayer][(gametic/ticdup)%BACKUPTICS] = inject_cmd[inject_next++];
    }
    G_Ticker ();
}

static int read_commands (const char* path)
{
    FILE* f = fopen (path, "r");
    int fw, sd, at, bt, cap = 0;
    char line[256];
    if (!f) { perror (path); return 0; }
    while (fgets (line, sizeof line, f)) {
        if (line[0] == '#' || line[0] == '\n') continue;
        if (sscanf (line, "%d %d %d %d", &fw, &sd, &at, &bt) != 4 || fw < -128 || fw > 127 || sd < -128 || sd > 127
            || at < -32768 || at > 32767 || bt < 0 || bt > 255) { fprintf (stderr, "bad command line: %s", line); fclose (f); return 0; }
        if (inject_count == cap) {
            cap = cap ? 2 * cap : 256;
            if (!(inject_cmd = realloc (inject_cmd, cap * sizeof (ticcmd_t)))) { fclose (f); return 0; }
        }
        memset (&inject_cmd[inject_count], 0, sizeof (ticcmd_t));
        inject_cmd[inject_count].forwardmove = (char)fw;
        inject_cmd[inject_count].sidemove = (char)sd;
        inject_cmd[inject_count].angleturn = (short)at;
        inject_cmd[inject_count].buttons = (byte)bt;
        inject_count++;
    }
    fclose (f);
    if (!inject_cmd) inject_cmd = calloc (1, sizeof (ticcmd_t));
    return inject_cmd != NULL;
}

int main (int argc, char** argv)
{
    int i, n = 0;
    char** rest = calloc (argc + 1, sizeof (char*));
    if (!rest) return 4;
    for (i = 0; i < argc; i++) {
        if (i > 0 && !strcmp (argv[i], "--commands") && i + 1 < argc) { if (!read_commands (argv[++i])) return 2; }
        else rest[n++] = argv[i];
    }
    return m5b_runner_main (n, rest);
}
