/*
 * M5c mixer driver (contract DOOM-M5C-MIXER-v1, tools/reference/m5c_mixer_build.py).
 *
 * Runs the genuine linuxdoom-1.10 in-process mixer (i_sound.c compiled whole, with sounds.c,
 * w_wad.c and z_zone.c) headlessly, behind the declared fake OSS device of m5c_mixer_fakesnd.c.
 * Sound lumps are read through the genuine w_wad.c and z_zone.c. The in-process configuration
 * is selected by the one declared configuration adapter at doomdef.h:84 (SNDSERV); see the
 * contract tests/reference/m5c_mixer_contract.json for every adapter, stub and wrapped symbol.
 *
 * Start-up, in the genuine order: Z_Init (d_main.c:1018); W_InitMultipleFiles over the --wad
 * files (d_main.c:1021); I_InitSound (from I_Init, d_main.c:1101 and i_system.c:107-110);
 * I_SetChannels (from S_Init, d_main.c:1107 and s_sound.c:170).
 *
 * START LIST (schema DOOM-M5C-STARTS-v1, one directive per line, '#' starts a comment line):
 *   schema DOOM-M5C-STARTS-v1            first directive, required
 *   config integrated                    required: the handle convention of the run that made the
 *                                        list; the in-process mixer exists only in the integrated
 *                                        configuration (doomdef.h:84; i_sound.c:482-500)
 *   tics N                               optional: run at least tics 0..N-1 (default 0)
 *   s TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO HANDLE
 *                                        one I_StartSound call: the session tic during which
 *                                        it was made, its call order within the run (the pair
 *                                        TIC SEQ strictly increases down the list), gametic at
 *                                        the call, and exactly the five values s_sound.c passes
 *                                        to I_StartSound (s_sound.c:389-394); HANDLE is the
 *                                        handle the sound-logic run received, or '-'; the driver
 *                                        checks it against the genuine return value
 *   n COUNT TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO
 *                                        harness only: COUNT identical starts with call orders
 *                                        SEQ..SEQ+COUNT-1 (used for the handle wrap)
 *   g TIC SEQ SFX                        harness only: a call of the genuine I_GetSfxLumpNum
 *                                        (i_sound.c:451-456), the lookup s_sound.c:364-365 makes
 * SFX must be 1..NUMSFX-1 (the driver indexes S_sfx with it); PITCH must be 0..255 unless
 * --probe is given (s_sound.c clamps it, s_sound.c:332-335, 342-345; steptable has 256 entries).
 *
 * Per tic t = 0, 1, ...: every row of tic t in list order (gametic set to the row's GAMETIC
 * first; G_Ticker runs with gametic == t in a session started on the map, d_net.c:745-746);
 * then the schedule mixes buffers, each by the genuine I_UpdateSound () and I_SubmitSound ()
 * (d_main.c:396-405 with SNDSERV and SNDINTR undefined); the fake write hands the 2,048 bytes
 * to fake_capture. The run continues past the last row and the last of the N tics until every
 * mixer channel is idle (checked at tic boundaries), or until --max-buffers.
 *
 * Schedules (--schedule):
 *   ticbuffers (default, the recommended reading of decision T5): after the rows of tic t, mix
 *              every buffer k not yet mixed with 512*k < 315*(t+1), so buffer k is mixed after
 *              tic floor(512k/315): none or one per tic, 315 buffers per 512 tics; a start
 *              takes effect in the next buffer mixed.
 *   pertic     one genuine I_UpdateSound and I_SubmitSound after every tic (the singletics
 *              loop, d_main.c:375-405), kept only so that the other reading can be reproduced.
 *
 * OUTPUT ROWS (--out, schema DOOM-M5C-MIXER-v1; documented in the contract's golden_row_kinds):
 *   h DOOM-M5C-MIXER-v1 schedule S frames 512
 *   d opens O ioctls I fragment 0xF speed S stereo T fmt 0xM     the fake device after init
 *   T steptable SHA256 / T vol_lookup SHA256                    the genuine tables (LE int32)
 *   L COUNT SHA256      digest of the COUNT l rows below (always printed)
 *   l SFX LEN SHA256    with --lumps: lengths[SFX] and the digest of the LEN padded data bytes
 *   P SFX SIZE PADDED PAD OF128 KIND   with --lumps: a loaded sfx whose lump size - 8 differs
 *                       from its padded length, or that fell back to dspistol (KIND own|fallback)
 *   s TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO > HANDLE SLOT STEP LVOL RVOL BEFORE PREV FREED
 *                       the echo of one start: the genuine handle, the mixer channel it took,
 *                       its step, left and right volume rows, the occupancy mask before the call,
 *                       the sfx that occupied SLOT before the call (or -), and the other channels
 *                       the call freed (the single-instance rule, i_sound.c:285-306), or -
 *   m TIC SEQ EXPECTED GOT     the HANDLE column of the list differs from the genuine handle
 *   o TIC SEQ SFX       the started non-linked sfx has padded length 0: its first mixed frame
 *                       reads one byte past its 8-byte block (i_sound.c:230-247, 591, 606)
 *   sn TIC SEQ COUNT GAMETIC SFX VOL SEP PITCH PRIO > FIRST LAST HSHA SSHA MASK
 *                       an n row: first and last handle, SHA-256 of the handles and of the slots
 *                       (one decimal per line), and the occupancy mask after the last start
 *   g TIC SEQ SFX > LUMP       the genuine I_GetSfxLumpNum result
 *   b K TIC SHA256 MASK        buffer K, mixed after tic TIC: SHA-256 of the 2,048 bytes the
 *                       genuine I_SubmitSound wrote, and the occupancy mask after mixing
 *   r N KLAST TICLAST   the previous b row's digest and mask repeat for N more buffers, the last
 *                       of which is KLAST, mixed after tic TICLAST
 *   x TIC SEQ MESSAGE   I_Error (the genuine game exits; the run stops); x init MESSAGE at init;
 *                       x limit - MESSAGE when --max-buffers is reached
 *   q closes C writes W the fake device at the end, after the genuine I_ShutdownSound
 *   z NBUF SHA256       the number of buffers and the SHA-256 of the whole stream
 *
 * Options: --wad FILE (repeatable, in load order), --starts FILE, --out FILE, --schedule S,
 * --lumps, --raw FILE (the stream as the fake device received it: little-endian signed 16-bit,
 * left first; scratch only, never committed), --tables FILE (steptable then vol_lookup as raw
 * int32), --zone-mb N (default 6, vanilla mb_used, i_system.c:51), --max-buffers N (default
 * 2000000), --probe, --perturb-byte N (test only: flip bit 0 of stream byte N before it is
 * digested). The driver only READS i_sound.c's globals; it never writes them.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>
#include "doomdef.h"
#include "doomtype.h"
#include "sounds.h"
#include "z_zone.h"
#include "w_wad.h"
#include "i_sound.h"

/* ---------------------------------------------------------------- SHA-256 (FIPS 180-4) */
typedef struct { unsigned int h[8]; unsigned char b[64]; unsigned long long n; unsigned int bl; } sha_t;
static const unsigned int K256[64] = {
0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,
0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,
0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,
0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2};
#define RR(x,n) (((x)>>(n))|((x)<<(32-(n))))
static void sha_blk (sha_t *s)
{
    unsigned int w[64], a, b, c, d, e, f, g, h, t1, t2; int i;
    for (i = 0; i < 16; i++)
        w[i] = (unsigned) s->b[4*i] << 24 | (unsigned) s->b[4*i+1] << 16 | (unsigned) s->b[4*i+2] << 8 | s->b[4*i+3];
    for (; i < 64; i++)
        w[i] = (RR(w[i-2],17) ^ RR(w[i-2],19) ^ (w[i-2] >> 10)) + w[i-7] + (RR(w[i-15],7) ^ RR(w[i-15],18) ^ (w[i-15] >> 3)) + w[i-16];
    a = s->h[0]; b = s->h[1]; c = s->h[2]; d = s->h[3]; e = s->h[4]; f = s->h[5]; g = s->h[6]; h = s->h[7];
    for (i = 0; i < 64; i++) {
        t1 = h + (RR(e,6) ^ RR(e,11) ^ RR(e,25)) + ((e & f) ^ (~e & g)) + K256[i] + w[i];
        t2 = (RR(a,2) ^ RR(a,13) ^ RR(a,22)) + ((a & b) ^ (a & c) ^ (b & c));
        h = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
    }
    s->h[0] += a; s->h[1] += b; s->h[2] += c; s->h[3] += d; s->h[4] += e; s->h[5] += f; s->h[6] += g; s->h[7] += h;
}
static void sha_init (sha_t *s)
{
    static const unsigned int H[8] = {0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19};
    memcpy (s->h, H, sizeof H); s->n = 0; s->bl = 0;
}
static void sha_add (sha_t *s, const void *p, size_t n)
{
    const unsigned char *q = p;
    while (n--) { s->b[s->bl++] = *q++; s->n++; if (s->bl == 64) { sha_blk (s); s->bl = 0; } }
}
static void sha_hex (sha_t *s, char *out)   /* out: 65 bytes */
{
    unsigned long long bits = s->n * 8; unsigned char z = 0x80; int i;
    sha_add (s, &z, 1); z = 0;
    while (s->bl != 56) sha_add (s, &z, 1);
    for (i = 7; i >= 0; i--) { unsigned char c = (unsigned char) (bits >> (8 * i)); sha_add (s, &c, 1); }
    for (i = 0; i < 8; i++) sprintf (out + 8 * i, "%08x", s->h[i]);
}
static void sha_of (const void *p, size_t n, char *out) { sha_t s; sha_init (&s); sha_add (&s, p, n); sha_hex (&s, out); }

/* ---------------------------------------------------------------- genuine globals (read only) */
extern int lengths[NUMSFX];                     /* i_sound.c:103 */
extern unsigned char* channels[8];              /* i_sound.c:122 */
extern unsigned char* channelsend[8];           /* i_sound.c:123 */
extern int channelhandles[8];                   /* i_sound.c:137 */
extern int channelids[8];                       /* i_sound.c:141 */
extern unsigned int channelstep[8];             /* i_sound.c:116 */
extern int steptable[256];                      /* i_sound.c:144 */
extern int vol_lookup[128*256];                 /* i_sound.c:147 */
extern int* channelleftvol_lookup[8];           /* i_sound.c:150 */
extern int* channelrightvol_lookup[8];          /* i_sound.c:151 */
extern int gametic;                             /* the stub of g_game.c:123 */
/* driver hooks of m5c_mixer_stubs.c and m5c_mixer_fakesnd.c */
extern jmp_buf stub_jmp; extern int stub_armed; extern char stub_msg[256]; extern int stub_zone_mb;
extern int fake_dsp_opens, fake_dsp_closes, fake_dsp_writes, fake_ioctls, fake_speed, fake_stereo, fake_fmt, fake_fragment;

#define FRAMES 512                              /* SAMPLECOUNT, i_sound.c:93 */
#define BUFBYTES 2048                           /* SAMPLECOUNT*BUFMUL, i_sound.c:96, 669 */
#define TICFRAMES 315                           /* 11025 / 35 */

static FILE *out, *raw;
static sha_t whole;
static unsigned char cap[BUFBYTES]; static size_t capn; static int capcalls;
static long perturb_byte = -1, total_bytes;
static int probe;

void fake_capture (const void *buf, size_t n)
{
    capcalls++;
    if (n != BUFBYTES) { fprintf (stderr, "m5c_mixer: a write of %lu bytes (expected %d)\n", (unsigned long) n, BUFBYTES); exit (5); }
    memcpy (cap, buf, n); capn = n;
}

/* ---------------------------------------------------------------- the start list */
typedef struct { char kind; long count; int tic, seq, gametic, sfx, vol, sep, pitch, prio, handle; } row_t;
static row_t *rows; static int nrows, mintics;

static void bad_list (const char *path, int ln, const char *why)
{
    fprintf (stderr, "m5c_mixer: %s:%d: %s\n", path, ln, why);
    exit (3);
}

static int parse_ints (char *p, long *v, int n, int *dash_last)
{
    int i; char *e;
    for (i = 0; i < n; i++) {
        while (*p == ' ' || *p == '\t') p++;
        if (dash_last && i == n - 1 && *p == '-' && (p[1] == 0 || p[1] == ' ' || p[1] == '\n' || p[1] == '\r')) {
            *dash_last = 1; v[i] = -1; p++; continue;
        }
        v[i] = strtol (p, &e, 10);
        if (e == p) return 0;
        p = e;
    }
    while (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r') p++;
    return *p == 0;
}

static void load_starts (const char *path)
{
    FILE *f = fopen (path, "r"); char line[512]; int ln = 0, cap_ = 0, have_schema = 0, have_config = 0;
    if (!f) { fprintf (stderr, "m5c_mixer: cannot open %s\n", path); exit (3); }
    while (fgets (line, sizeof line, f)) {
        char *p = line; row_t r; long v[10]; int dash = 0;
        ln++;
        if (!strchr (line, '\n') && !feof (f)) bad_list (path, ln, "line too long");
        while (*p == ' ' || *p == '\t') p++;
        if (*p == '#' || *p == '\n' || *p == '\r' || !*p) continue;
        memset (&r, 0, sizeof r); r.handle = -1; r.count = 1;
        if (!have_schema) {
            if (strncmp (p, "schema DOOM-M5C-STARTS-v1", 25) || (p[25] != '\n' && p[25] != '\r' && p[25] != 0))
                bad_list (path, ln, "the first directive must be: schema DOOM-M5C-STARTS-v1");
            have_schema = 1; continue;
        }
        if (!strncmp (p, "config ", 7)) {
            if (strncmp (p + 7, "integrated", 10) || (p[17] != '\n' && p[17] != '\r' && p[17] != 0))
                bad_list (path, ln, "config must be integrated: the in-process mixer and its handles 100, 101, ... exist only "
                                    "with SNDSERV undefined (doomdef.h:84; i_sound.c:482-500)");
            have_config = 1; continue;
        }
        if (!strncmp (p, "tics ", 5)) {
            if (!parse_ints (p + 5, v, 1, NULL) || v[0] < 0 || v[0] > 100000000L) bad_list (path, ln, "bad tics");
            mintics = (int) v[0]; continue;
        }
        if (p[0] == 's' && p[1] == ' ') {
            if (!parse_ints (p + 2, v, 9, &dash)) bad_list (path, ln, "s needs TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO HANDLE");
            r.kind = 's'; r.tic = (int) v[0]; r.seq = (int) v[1]; r.gametic = (int) v[2]; r.sfx = (int) v[3];
            r.vol = (int) v[4]; r.sep = (int) v[5]; r.pitch = (int) v[6]; r.prio = (int) v[7]; r.handle = dash ? -1 : (int) v[8];
            if (!dash && (v[8] < 0 || v[8] > 65535)) bad_list (path, ln, "HANDLE must be 0..65535 or -");
        } else if (p[0] == 'n' && p[1] == ' ') {
            if (!parse_ints (p + 2, v, 9, NULL)) bad_list (path, ln, "n needs COUNT TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO");
            r.kind = 'n'; r.count = v[0]; r.tic = (int) v[1]; r.seq = (int) v[2]; r.gametic = (int) v[3]; r.sfx = (int) v[4];
            r.vol = (int) v[5]; r.sep = (int) v[6]; r.pitch = (int) v[7]; r.prio = (int) v[8];
            if (r.count < 1 || r.count > 10000000L) bad_list (path, ln, "COUNT must be 1..10000000");
        } else if (p[0] == 'g' && p[1] == ' ') {
            if (!parse_ints (p + 2, v, 3, NULL)) bad_list (path, ln, "g needs TIC SEQ SFX");
            r.kind = 'g'; r.tic = (int) v[0]; r.seq = (int) v[1]; r.sfx = (int) v[2];
        } else bad_list (path, ln, "unknown directive");
        if (!have_config) bad_list (path, ln, "config integrated must come before the first row");
        if (r.tic < 0 || r.seq < 0) bad_list (path, ln, "TIC and SEQ must be >= 0");
        if (r.sfx < 1 || r.sfx >= NUMSFX) bad_list (path, ln, "SFX must be 1..NUMSFX-1 (S_sfx and lengths have NUMSFX entries)");
        if (r.kind != 'g' && !probe && (r.pitch < 0 || r.pitch > 255))
            bad_list (path, ln, "PITCH outside 0..255 indexes past steptable (i_sound.c:495); s_sound.c clamps it "
                                "(s_sound.c:332-335, 342-345); only --probe accepts it");
        if (nrows) {
            row_t *q = &rows[nrows - 1]; long qlast = q->seq + q->count - 1;
            if (r.tic < q->tic || (r.tic == q->tic && r.seq <= qlast)) bad_list (path, ln, "TIC SEQ must strictly increase");
        }
        if (nrows == cap_) { cap_ = cap_ ? 2 * cap_ : 64; rows = realloc (rows, (size_t) cap_ * sizeof *rows); if (!rows) exit (4); }
        rows[nrows++] = r;
    }
    fclose (f);
    if (!have_schema || !have_config) bad_list (path, ln, "missing schema or config directive");
}

/* ---------------------------------------------------------------- rows */
static long nbuf;
static char last_hex[65]; static int last_mask = -1; static long rep, rep_k; static int rep_tic;

static int mask (void) { int i, m = 0; for (i = 0; i < 8; i++) if (channels[i]) m |= 1 << i; return m; }

static void flush_rep (void)
{
    if (rep) fprintf (out, "r %ld %ld %d\n", rep, rep_k, rep_tic);
    rep = 0;
}
static void row_break (void) { flush_rep (); last_mask = -1; }

static void mix_one (int tic)
{
    char hex[65]; int m;
    capn = 0; capcalls = 0;
    I_UpdateSound ();
    I_SubmitSound ();
    if (capcalls != 1 || capn != BUFBYTES) { fprintf (stderr, "m5c_mixer: I_SubmitSound wrote %d times\n", capcalls); exit (5); }
    if (perturb_byte >= total_bytes && perturb_byte < total_bytes + (long) capn) cap[perturb_byte - total_bytes] ^= 1;
    total_bytes += (long) capn;
    sha_of (cap, capn, hex);
    sha_add (&whole, cap, capn);
    if (raw && fwrite (cap, 1, capn, raw) != capn) { fprintf (stderr, "m5c_mixer: raw write failed\n"); exit (4); }
    m = mask ();
    if (m == last_mask && !strcmp (hex, last_hex)) { rep++; rep_k = nbuf; rep_tic = tic; }
    else {
        flush_rep ();
        fprintf (out, "b %ld %d %s %02x\n", nbuf, tic, hex, m);
        strcpy (last_hex, hex); last_mask = m;
    }
    nbuf++;
}

static int find_slot (const int *bh, unsigned char *const *bc, int h)
{
    int k;
    for (k = 0; k < 8; k++) if (channelhandles[k] != bh[k]) return k;
    for (k = 0; k < 8; k++) if (channels[k] && channels[k] != bc[k] && channelhandles[k] == h) return k;
    return -1;
}

/* one start; returns the slot */
static int do_start (row_t *r, int *h_out, int echo)
{
    int bh[8], bid[8], k, slot, h, before = mask (), lv, rv; unsigned char *bc[8]; char freed[32]; int fl = 0;
    for (k = 0; k < 8; k++) { bh[k] = channelhandles[k]; bc[k] = channels[k]; bid[k] = channelids[k]; }
    gametic = r->gametic;
    h = I_StartSound (r->sfx, r->vol, r->sep, r->pitch, r->prio);     /* s_sound.c:389-394 */
    slot = find_slot (bh, bc, h);
    if (slot < 0) { fprintf (stderr, "m5c_mixer: cannot find the channel of handle %d\n", h); exit (5); }
    *h_out = h;
    if (!echo) return slot;
    freed[0] = 0;
    for (k = 0; k < 8; k++)
        if (k != slot && bc[k] && !channels[k]) fl += sprintf (freed + fl, "%s%d", fl ? "," : "", k);
    lv = (int) (channelleftvol_lookup[slot] - vol_lookup) / 256;
    rv = (int) (channelrightvol_lookup[slot] - vol_lookup) / 256;
    row_break ();
    fprintf (out, "s %d %d %d %d %d %d %d %d > %d %d %u %d %d %02x ", r->tic, r->seq, r->gametic, r->sfx, r->vol, r->sep,
             r->pitch, r->prio, h, slot, channelstep[slot], lv, rv, before);
    if (bc[slot]) fprintf (out, "%d", bid[slot]); else fputc ('-', out);
    fprintf (out, " %s\n", fl ? freed : "-");
    if (r->handle >= 0 && r->handle != h) fprintf (out, "m %d %d %d %d\n", r->tic, r->seq, r->handle, h);
    if (!S_sfx[r->sfx].link && lengths[r->sfx] == 0) fprintf (out, "o %d %d %d\n", r->tic, r->seq, r->sfx);
    return slot;
}

static void lump_rows (int full)
{
    char hex[65], line[128]; int i; sha_t s; sha_init (&s);
    for (i = 0; i < NUMSFX; i++) {
        if (S_sfx[i].data) sha_of (S_sfx[i].data, (size_t) lengths[i], hex); else strcpy (hex, "-");
        sprintf (line, "l %d %d %s\n", i, lengths[i], hex);
        sha_add (&s, line, strlen (line));
        if (full) fputs (line, out);
    }
    sha_hex (&s, hex);
    fprintf (out, "L %d %s\n", NUMSFX, hex);
    if (!full) return;
    for (i = 1; i < NUMSFX; i++) {
        char name[20]; int lump, own, size, k, n128 = 0, real;
        if (S_sfx[i].link) continue;
        sprintf (name, "ds%s", S_sfx[i].name);           /* i_sound.c:201 */
        own = W_CheckNumForName (name) != -1;
        lump = own ? W_CheckNumForName (name) : W_CheckNumForName ("dspistol");
        size = W_LumpLength (lump);
        real = size - 8 > 0 ? size - 8 : 0;
        for (k = real; k < lengths[i]; k++) n128 += ((unsigned char *) S_sfx[i].data)[k] == 128;
        if (size - 8 != lengths[i] || !own)
            fprintf (out, "P %d %d %d %d %d %s\n", i, size, lengths[i], lengths[i] - real, n128, own ? "own" : "fallback");
    }
}

int main (int argc, char **argv)
{
    static char *wads[64]; int nw = 0, i, t, ri = 0, lumps = 0, pertic = 0; long max_buffers = 2000000;
    volatile int inited = 0;            /* set after setjmp, read after a longjmp */
    const char *schedule = "ticbuffers", *startsfile = 0, *tablesfile = 0, *outfile = 0, *rawfile = 0; char hex[65];
    for (i = 1; i < argc; i++) {
        if (!strcmp (argv[i], "--wad") && i + 1 < argc && nw < 63) wads[nw++] = argv[++i];
        else if (!strcmp (argv[i], "--starts") && i + 1 < argc) startsfile = argv[++i];
        else if (!strcmp (argv[i], "--schedule") && i + 1 < argc) schedule = argv[++i];
        else if (!strcmp (argv[i], "--out") && i + 1 < argc) outfile = argv[++i];
        else if (!strcmp (argv[i], "--raw") && i + 1 < argc) rawfile = argv[++i];
        else if (!strcmp (argv[i], "--tables") && i + 1 < argc) tablesfile = argv[++i];
        else if (!strcmp (argv[i], "--zone-mb") && i + 1 < argc) stub_zone_mb = atoi (argv[++i]);
        else if (!strcmp (argv[i], "--max-buffers") && i + 1 < argc) max_buffers = atol (argv[++i]);
        else if (!strcmp (argv[i], "--perturb-byte") && i + 1 < argc) perturb_byte = atol (argv[++i]);
        else if (!strcmp (argv[i], "--lumps")) lumps = 1;
        else if (!strcmp (argv[i], "--probe")) probe = 1;
        else { fprintf (stderr, "m5c_mixer: bad argument %s\n", argv[i]); return 2; }
    }
    if (!strcmp (schedule, "pertic")) pertic = 1;
    else if (strcmp (schedule, "ticbuffers")) { fprintf (stderr, "m5c_mixer: --schedule ticbuffers|pertic\n"); return 2; }
    if (!nw || stub_zone_mb < 1 || stub_zone_mb > 64 || max_buffers < 1) { fprintf (stderr, "m5c_mixer: bad arguments\n"); return 2; }
    out = outfile ? fopen (outfile, "w") : stdout;
    if (!out) { fprintf (stderr, "m5c_mixer: cannot open %s\n", outfile); return 4; }
    if (rawfile && !(raw = fopen (rawfile, "wb"))) { fprintf (stderr, "m5c_mixer: cannot open %s\n", rawfile); return 4; }
    if (startsfile) load_starts (startsfile);
    wads[nw] = 0;
    sha_init (&whole);
    fprintf (out, "h DOOM-M5C-MIXER-v1 schedule %s frames %d\n", schedule, FRAMES);

    stub_armed = 1;
    if (setjmp (stub_jmp)) { fprintf (out, "x init %s\n", stub_msg); goto end; }
    Z_Init ();                          /* d_main.c:1018 */
    W_InitMultipleFiles (wads);         /* d_main.c:1021 */
    I_InitSound ();                     /* d_main.c:1101 -> I_Init, i_system.c:107-110 */
    I_SetChannels ();                   /* d_main.c:1107 -> S_Init, s_sound.c:170 */
    inited = 1;
    fprintf (out, "d opens %d ioctls %d fragment 0x%x speed %d stereo %d fmt 0x%x\n",
             fake_dsp_opens, fake_ioctls, fake_fragment, fake_speed, fake_stereo, fake_fmt);
    sha_of (steptable, sizeof steptable, hex); fprintf (out, "T steptable %s\n", hex);
    sha_of (vol_lookup, sizeof vol_lookup, hex); fprintf (out, "T vol_lookup %s\n", hex);
    if (tablesfile) {
        FILE *f = fopen (tablesfile, "wb");
        if (!f || fwrite (steptable, 1, sizeof steptable, f) != sizeof steptable
            || fwrite (vol_lookup, 1, sizeof vol_lookup, f) != sizeof vol_lookup) { fprintf (stderr, "m5c_mixer: tables\n"); return 4; }
        fclose (f);
    }
    lump_rows (lumps);

    for (t = 0; ; t++) {
        if (t >= mintics && ri >= nrows && !mask ()) break;
        if (nbuf >= max_buffers) { row_break (); fprintf (out, "x limit - %ld buffers mixed and a channel is still busy\n", nbuf); break; }
        while (ri < nrows && rows[ri].tic == t) {
            row_t *r = &rows[ri];
            if (setjmp (stub_jmp)) { row_break (); fprintf (out, "x %d %d %s\n", r->tic, r->seq, stub_msg); goto end; }
            if (r->kind == 'g') {
                int lump = I_GetSfxLumpNum (&S_sfx[r->sfx]);     /* i_sound.c:451-456 */
                row_break ();
                fprintf (out, "g %d %d %d > %d\n", r->tic, r->seq, r->sfx, lump);
            } else if (r->kind == 's') {
                int h; do_start (r, &h, 1);
            } else {
                sha_t hs, ss; char hh[65], sh[65], num[16]; long c; int h = 0, first = 0, slot, n;
                row_t one = *r;
                sha_init (&hs); sha_init (&ss);
                for (c = 0; c < r->count; c++) {
                    one.seq = (int) (r->seq + c); one.handle = -1;
                    slot = do_start (&one, &h, 0);
                    if (c == 0) first = h;
                    n = sprintf (num, "%d\n", h); sha_add (&hs, num, (size_t) n);
                    n = sprintf (num, "%d\n", slot); sha_add (&ss, num, (size_t) n);
                }
                sha_hex (&hs, hh); sha_hex (&ss, sh);
                row_break ();
                fprintf (out, "sn %d %d %ld %d %d %d %d %d %d > %d %d %s %s %02x\n", r->tic, r->seq, r->count, r->gametic, r->sfx,
                         r->vol, r->sep, r->pitch, r->prio, first, h, hh, sh, mask ());
            }
            ri++;
        }
        if (pertic) mix_one (t);
        else while ((long) FRAMES * nbuf < (long) TICFRAMES * (t + 1)) mix_one (t);
    }
end:
    row_break ();
    stub_armed = 0;
    if (inited) I_ShutdownSound ();     /* i_system.c:119 (I_Quit); it closes audio_fd, set only by I_InitSound */
    fprintf (out, "q closes %d writes %d\n", fake_dsp_closes, fake_dsp_writes);
    if (fake_dsp_writes != nbuf) { fprintf (stderr, "m5c_mixer: %d writes for %ld buffers\n", fake_dsp_writes, nbuf); return 5; }
    sha_hex (&whole, hex);
    fprintf (out, "z %ld %s\n", nbuf, hex);
    if (raw) fclose (raw);
    if (out != stdout) fclose (out);
    return 0;
}
