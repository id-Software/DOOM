// i_system_win.c – SDL2-based system backend (Windows + Emscripten).
// Replaces the Linux-specific gettimeofday / usleep calls.

#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <stdarg.h>

// Only define SDL_MAIN_HANDLED in non-Emscripten builds.
// emcc manages the entry-point separately and doesn't need this.
#ifndef __EMSCRIPTEN__
#define SDL_MAIN_HANDLED
#endif

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#endif

#include <SDL2/SDL.h>

#include "doomdef.h"
#include "m_misc.h"
#include "i_video.h"
#include "i_sound.h"
#include "d_net.h"
#include "g_game.h"

#ifdef __GNUG__
#pragma implementation "i_system.h"
#endif
#include "i_system.h"

int mb_used = 6;

void I_Tactile(int on, int off, int total)
{
    (void)on; (void)off; (void)total;
}

ticcmd_t    emptycmd;
ticcmd_t*   I_BaseTiccmd(void)  { return &emptycmd; }
int         I_GetHeapSize(void)  { return mb_used * 1024 * 1024; }

byte* I_ZoneBase(int *size)
{
    *size = mb_used * 1024 * 1024;
    return (byte *)malloc(*size);
}

//
// I_GetTime
// Returns time in 1/TICRATE (35 Hz) tics using SDL_GetTicks.
//
int I_GetTime(void)
{
    static Uint32 basetime = 0;
    Uint32 now = SDL_GetTicks();
    if (!basetime)
        basetime = now;
    return (int)((now - basetime) * TICRATE / 1000);
}

//
// I_Init
//
void I_Init(void)
{
    // Initialize SDL timer (video initialized later in I_InitGraphics)
    if (SDL_Init(SDL_INIT_TIMER) < 0)
        fprintf(stderr, "SDL_Init(TIMER) failed: %s\n", SDL_GetError());
    I_InitSound();
}

//
// I_Quit
//
void I_Quit(void)
{
    D_QuitNetGame();
    I_ShutdownSound();
    I_ShutdownMusic();
    M_SaveDefaults();
    I_ShutdownGraphics();
    SDL_Quit();
#ifdef __EMSCRIPTEN__
    emscripten_cancel_main_loop();
    emscripten_force_exit(0);
#else
    exit(0);
#endif
}

void I_WaitVBL(int count)
{
    SDL_Delay(count * (1000 / 70));
}

void I_BeginRead(void)  {}
void I_EndRead(void)    {}

byte* I_AllocLow(int length)
{
    byte *mem = (byte *)malloc(length);
    memset(mem, 0, length);
    return mem;
}

//
// I_Error
//
extern boolean demorecording;

void I_Error(char *error, ...)
{
    va_list argptr;

    va_start(argptr, error);
    fprintf(stderr, "Error: ");
    vfprintf(stderr, error, argptr);
    fprintf(stderr, "\n");
    va_end(argptr);
    fflush(stderr);

    if (demorecording)
        G_CheckDemoStatus();

    D_QuitNetGame();
    I_ShutdownGraphics();
    SDL_Quit();
#ifdef __EMSCRIPTEN__
    emscripten_cancel_main_loop();
    emscripten_force_exit(-1);
#else
    exit(-1);
#endif
}
