// SDL2-based video backend for DOOM (Windows + Emscripten port)
// Replaces X11-based i_video.c

// Only define SDL_MAIN_HANDLED in non-Emscripten builds.
#ifndef __EMSCRIPTEN__
#define SDL_MAIN_HANDLED
#endif

#include <stdlib.h>
#include <string.h>

#include <SDL2/SDL.h>

#include "doomstat.h"
#include "i_system.h"
#include "v_video.h"
#include "m_argv.h"
#include "d_main.h"
#include "doomdef.h"

static SDL_Window   *sdl_window   = NULL;
static SDL_Renderer *sdl_renderer = NULL;
static SDL_Texture  *sdl_texture  = NULL;

// 32-bit palette (ARGB) built from the 8-bit DOOM palette
static Uint32 sdl_palette[256];

// Window scale multiplier (1 = 320x200, 2 = 640x400, etc.)
static int multiply = 2;

//
// Translate an SDL key to a DOOM key code
//
static int xlatekey(SDL_Keycode sym)
{
    switch (sym)
    {
      case SDLK_LEFT:      return KEY_LEFTARROW;
      case SDLK_RIGHT:     return KEY_RIGHTARROW;
      case SDLK_DOWN:      return KEY_DOWNARROW;
      case SDLK_UP:        return KEY_UPARROW;
      case SDLK_ESCAPE:    return KEY_ESCAPE;
      case SDLK_RETURN:    return KEY_ENTER;
      case SDLK_TAB:       return KEY_TAB;
      case SDLK_F1:        return KEY_F1;
      case SDLK_F2:        return KEY_F2;
      case SDLK_F3:        return KEY_F3;
      case SDLK_F4:        return KEY_F4;
      case SDLK_F5:        return KEY_F5;
      case SDLK_F6:        return KEY_F6;
      case SDLK_F7:        return KEY_F7;
      case SDLK_F8:        return KEY_F8;
      case SDLK_F9:        return KEY_F9;
      case SDLK_F10:       return KEY_F10;
      case SDLK_F11:       return KEY_F11;
      case SDLK_F12:       return KEY_F12;
      case SDLK_BACKSPACE:
      case SDLK_DELETE:    return KEY_BACKSPACE;
      case SDLK_PAUSE:     return KEY_PAUSE;
      case SDLK_EQUALS:    return KEY_EQUALS;
      case SDLK_MINUS:     return KEY_MINUS;
      case SDLK_LSHIFT:
      case SDLK_RSHIFT:    return KEY_RSHIFT;
      case SDLK_LCTRL:
      case SDLK_RCTRL:     return KEY_RCTRL;
      case SDLK_LALT:
      case SDLK_RALT:      return KEY_RALT;
      default:
        if (sym >= SDLK_SPACE && sym <= SDLK_z)
        {
            int rc = (int)sym;
            if (rc >= 'A' && rc <= 'Z')
                rc = rc - 'A' + 'a';
            return rc;
        }
        return 0;
    }
}

//
// I_ShutdownGraphics
//
void I_ShutdownGraphics(void)
{
    if (sdl_texture)  { SDL_DestroyTexture(sdl_texture);   sdl_texture  = NULL; }
    if (sdl_renderer) { SDL_DestroyRenderer(sdl_renderer); sdl_renderer = NULL; }
    if (sdl_window)   { SDL_DestroyWindow(sdl_window);     sdl_window   = NULL; }
    SDL_QuitSubSystem(SDL_INIT_VIDEO);
}

//
// I_StartFrame
//
void I_StartFrame(void)
{
    // nothing
}

//
// I_StartTic - poll SDL events and post DOOM events
//
void I_StartTic(void)
{
    SDL_Event sdlev;
    event_t   doom_ev;

    while (SDL_PollEvent(&sdlev))
    {
        switch (sdlev.type)
        {
          case SDL_KEYDOWN:
            doom_ev.type  = ev_keydown;
            doom_ev.data1 = xlatekey(sdlev.key.keysym.sym);
            if (doom_ev.data1) D_PostEvent(&doom_ev);
            break;

          case SDL_KEYUP:
            doom_ev.type  = ev_keyup;
            doom_ev.data1 = xlatekey(sdlev.key.keysym.sym);
            if (doom_ev.data1) D_PostEvent(&doom_ev);
            break;

          case SDL_MOUSEBUTTONDOWN:
          case SDL_MOUSEBUTTONUP:
          {
            int buttons = SDL_GetMouseState(NULL, NULL);
            doom_ev.type  = ev_mouse;
            doom_ev.data1 = ((buttons & SDL_BUTTON_LMASK) ? 1 : 0)
                          | ((buttons & SDL_BUTTON_MMASK) ? 2 : 0)
                          | ((buttons & SDL_BUTTON_RMASK) ? 4 : 0);
            doom_ev.data2 = doom_ev.data3 = 0;
            D_PostEvent(&doom_ev);
            break;
          }

          case SDL_MOUSEMOTION:
            doom_ev.type  = ev_mouse;
            doom_ev.data1 = ((sdlev.motion.state & SDL_BUTTON_LMASK) ? 1 : 0)
                          | ((sdlev.motion.state & SDL_BUTTON_MMASK) ? 2 : 0)
                          | ((sdlev.motion.state & SDL_BUTTON_RMASK) ? 4 : 0);
            doom_ev.data2 =  sdlev.motion.xrel << 2;
            doom_ev.data3 = -sdlev.motion.yrel << 2;
            D_PostEvent(&doom_ev);
            break;

          case SDL_QUIT:
            I_Quit();
            break;

          default:
            break;
        }
    }
}

//
// I_UpdateNoBlit
//
void I_UpdateNoBlit(void)
{
    // nothing
}

//
// I_FinishUpdate - convert 8-bit DOOM framebuffer to 32-bit and blit via SDL
//
void I_FinishUpdate(void)
{
    static int lasttic = 0;
    int i, tics;

    // Draw benchmark dots in developer mode
    if (devparm)
    {
        i = I_GetTime();
        tics = i - lasttic;
        lasttic = i;
        if (tics > 20) tics = 20;
        for (i = 0; i < tics * 2; i += 2)
            screens[0][(SCREENHEIGHT - 1) * SCREENWIDTH + i] = 0xff;
        for (; i < 20 * 2; i += 2)
            screens[0][(SCREENHEIGHT - 1) * SCREENWIDTH + i] = 0x0;
    }

    // Convert 8-bit palettized to 32-bit ARGB
    {
        static Uint32 argbbuf[SCREENWIDTH * SCREENHEIGHT];
        int p;
        for (p = 0; p < SCREENWIDTH * SCREENHEIGHT; p++)
            argbbuf[p] = sdl_palette[(unsigned char)screens[0][p]];

        SDL_UpdateTexture(sdl_texture, NULL, argbbuf, SCREENWIDTH * sizeof(Uint32));
    }

    SDL_RenderClear(sdl_renderer);
    SDL_RenderCopy(sdl_renderer, sdl_texture, NULL, NULL);
    SDL_RenderPresent(sdl_renderer);
}

//
// I_ReadScreen
//
void I_ReadScreen(byte *scr)
{
    memcpy(scr, screens[0], SCREENWIDTH * SCREENHEIGHT);
}

//
// I_SetPalette - apply gamma and store 32-bit palette for blitting
//
void I_SetPalette(byte *palette)
{
    int  i;
    byte *p = palette;
    for (i = 0; i < 256; i++)
    {
        byte r = gammatable[usegamma][*p++];
        byte g = gammatable[usegamma][*p++];
        byte b = gammatable[usegamma][*p++];
        sdl_palette[i] = (0xFFu << 24) | ((Uint32)r << 16) | ((Uint32)g << 8) | b;
    }
}

//
// I_InitGraphics
//
void I_InitGraphics(void)
{
    int W, H;
    static int firsttime = 1;

    if (!firsttime) return;
    firsttime = 0;

    if (M_CheckParm("-2")) multiply = 2;
    if (M_CheckParm("-3")) multiply = 3;
    if (M_CheckParm("-4")) multiply = 4;

    W = SCREENWIDTH  * multiply;
    H = SCREENHEIGHT * multiply;

    if (SDL_InitSubSystem(SDL_INIT_VIDEO) < 0)
        I_Error("SDL video init failed: %s", SDL_GetError());

    sdl_window = SDL_CreateWindow(
        "DOOM",
        SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED,
        W, H, 0);
    if (!sdl_window)
        I_Error("SDL_CreateWindow failed: %s", SDL_GetError());

    // SDL_RENDERER_PRESENTVSYNC conflicts with emscripten_set_main_loop's
    // own RAF timing – use pure accelerated rendering on all platforms.
    sdl_renderer = SDL_CreateRenderer(
        sdl_window, -1,
        SDL_RENDERER_ACCELERATED);
    if (!sdl_renderer)
        sdl_renderer = SDL_CreateRenderer(sdl_window, -1, 0);
    if (!sdl_renderer)
        I_Error("SDL_CreateRenderer failed: %s", SDL_GetError());

    // Scale the 320x200 texture up to fill the window
    SDL_RenderSetLogicalSize(sdl_renderer, SCREENWIDTH, SCREENHEIGHT);

    sdl_texture = SDL_CreateTexture(
        sdl_renderer,
        SDL_PIXELFORMAT_ARGB8888,
        SDL_TEXTUREACCESS_STREAMING,
        SCREENWIDTH, SCREENHEIGHT);
    if (!sdl_texture)
        I_Error("SDL_CreateTexture failed: %s", SDL_GetError());

    screens[0] = (unsigned char *)malloc(SCREENWIDTH * SCREENHEIGHT);
    if (!screens[0])
        I_Error("Failed to allocate DOOM screen buffer");
}
