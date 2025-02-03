#include <vga.h>
#include <vgagl.h>
#include "doomdef.h"
#include "i_system.h"
#include "v_video.h"
#include "d_main.h"

static int current_mode;
static byte* screen_buffer;

boolean I_InitSVGA(void) 
{
    // Initialize SVGA library
    vga_init();
    
    // Try to set highest supported mode
    // Common SVGA modes: 640x480, 800x600, 1024x768
    if (vga_hasmode(G1024x768x256)) {
        current_mode = G1024x768x256;
        SCREENWIDTH = 1024;
        SCREENHEIGHT = 768;
    } else if (vga_hasmode(G800x600x256)) {
        current_mode = G800x600x256;
        SCREENWIDTH = 800;
        SCREENHEIGHT = 600;
    } else {
        current_mode = G640x480x256;
        SCREENWIDTH = 640;
        SCREENHEIGHT = 480;
    }
    
    if (vga_setmode(current_mode) == -1)
        I_Error("Could not set SVGA mode");
        
    // Allocate screen buffer
    screen_buffer = Z_Malloc(SCREENWIDTH * SCREENHEIGHT, PU_STATIC, NULL);
    screens[0] = screen_buffer;
    
    return true;
}

void I_UpdateSVGAScreen(void)
{
    // Copy buffer to SVGA memory
    vga_setpage(0);
    memcpy(vga_getgraphmem(), screen_buffer, SCREENWIDTH * SCREENHEIGHT);
}