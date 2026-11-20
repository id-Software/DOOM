/*
 * M5c mixer driver: the declared stubs (contract DOOM-M5C-MIXER-v1, field stubs) for the symbols
 * the four genuine files reference from files the driver does not compile. Every stub has one line
 *   STUB <symbol> [io|var] <vanilla file:lines>: <reason>
 * which tools/reference/m5c_mixer_build.py parses into the contract and checks against the
 * symbols this object defines. Driver hooks (prefix stub_) are not vanilla symbols.
 */
#include <stdio.h>
#include <stdlib.h>
#include <stdarg.h>
#include <setjmp.h>
#include "doomtype.h"

jmp_buf stub_jmp; int stub_armed; char stub_msg[256]; int stub_zone_mb = 6;

/* STUB gametic [var] g_game.c:123: the tic counter; the driver sets it to each start's GAMETIC before I_StartSound, as G_Ticker runs with gametic = t (d_net.c:745-746); read by addsfx (i_sound.c:276, 348) and I_SoundIsPlaying (i_sound.c:520) */
int gametic;
/* STUB snd_SfxVolume [var] s_sound.c:113: written by I_SetSfxVolume (i_sound.c:434), which nothing calls here; the mixer never reads it (volume reaches it only as an argument of I_StartSound) */
int snd_SfxVolume = 15;
/* STUB snd_MusicVolume [var] s_sound.c:116: written by I_SetMusicVolume (i_sound.c:441), which nothing calls here; music is out of scope */
int snd_MusicVolume = 15;

/* STUB I_Error [io] i_system.c:162-183: formats the message and longjmps back to the driver, which writes an x row and stops the run (vanilla prints it and exits) */
void I_Error (char *error, ...)
{
    va_list ap;
    va_start (ap, error); vsnprintf (stub_msg, sizeof stub_msg, error, ap); va_end (ap);
    if (stub_armed) longjmp (stub_jmp, 1);
    fprintf (stderr, "I_Error: %s\n", stub_msg);
    exit (2);
}
/* STUB I_ZoneBase [io] i_system.c:76-80: *size = mb_used MiB (6, i_system.c:51, unless --zone-mb) and malloc, as vanilla; the zone size moves addresses only */
byte* I_ZoneBase (int* size)
{
    *size = stub_zone_mb * 1024 * 1024;
    return (byte *) malloc ((size_t) *size);
}
