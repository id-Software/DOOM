// Emacs style mode select   -*- C++ -*- 
//-----------------------------------------------------------------------------
//
// $Id:$
//
// Copyright (C) 1993-1996 by id Software, Inc.
//
// This source is available for distribution and/or modification
// only under the terms of the DOOM Source Code License as
// published by id Software. All rights reserved.
//
// The source is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// FITNESS FOR A PARTICULAR PURPOSE. See the DOOM Source Code License
// for more details.
//
// $Log:$
//
// DESCRIPTION:
//	DOOM graphics stuff for X11, UNIX.
//
//-----------------------------------------------------------------------------

static const char
rcsid[] = "$Id: i_x.c,v 1.6 1997/02/03 22:45:10 b1 Exp $";

#include <stdlib.h>
#include <unistd.h>
#include <sys/ipc.h>
#include <sys/shm.h>

#include <stdarg.h>
#include <sys/time.h>
#include <sys/types.h>
#include <sys/socket.h>

#include <netinet/in.h>
#include <errno.h>
// #include <signal.h>

#include "doomstat.h"
#include "i_system.h"
#include "v_video.h"
#include "m_argv.h"
#include "d_main.h"

#include "doomdef.h"
#include "doomtype.h"

#define POINTER_WARP_COUNTDOWN	1

// We can't use X11, so lets define some extern functions to provide a similar interface

typedef struct {
	byte red;
	byte green;
	byte blue;
	byte _alpha;
} ZwareDoomColor;

extern int ZwareDoomOpenWindow();
extern int ZwareDoomPendingEvent();
extern int ZwareDoomNextEvent(int* event, int* data1, int* data2, int* data3);
extern int ZwareDoomRenderFrame(byte* screen, int size);
extern void ZwareDoomSetPalette(ZwareDoomColor* palette, int palette_len);


// Fake mouse handling.
// This cannot work properly w/o DGA.
// Needs an invisible mouse cursor at least.
boolean		grabMouse;
int		doPointerWarp = POINTER_WARP_COUNTDOWN;

// Blocky mode,
// replace each 320x200 pixel with multiply*multiply pixels.
// According to Dave Taylor, it still is a bonehead thing
// to use ....
static int	multiply=1;


//
//  Translates the key currently in X_event
//

int xlatekey(int zware_key)
{

    int rc = zware_key;

	// scancodes from glfw
    switch(rc = zware_key)
    {
      case 123:	rc = KEY_LEFTARROW;	break;
      case 124:	rc = KEY_RIGHTARROW;	break;
      case 125:	rc = KEY_DOWNARROW;	break;
      case 126:	rc = KEY_UPARROW;	break;
      case 53:	rc = KEY_ESCAPE;	break;
      case 36:	rc = KEY_ENTER;		break;
    //   case XK_Tab:	rc = KEY_TAB;		break;
    //   case XK_F1:	rc = KEY_F1;		break;
    //   case XK_F2:	rc = KEY_F2;		break;
    //   case XK_F3:	rc = KEY_F3;		break;
    //   case XK_F4:	rc = KEY_F4;		break;
    //   case XK_F5:	rc = KEY_F5;		break;
    //   case XK_F6:	rc = KEY_F6;		break;
    //   case XK_F7:	rc = KEY_F7;		break;
    //   case XK_F8:	rc = KEY_F8;		break;
    //   case XK_F9:	rc = KEY_F9;		break;
    //   case XK_F10:	rc = KEY_F10;		break;
    //   case XK_F11:	rc = KEY_F11;		break;
    //   case XK_F12:	rc = KEY_F12;		break;
	
    //   case XK_BackSpace:
    //   case XK_Delete:	rc = KEY_BACKSPACE;	break;

    //   case XK_Pause:	rc = KEY_PAUSE;		break;

    //   case XK_KP_Equal:
    //   case XK_equal:	rc = KEY_EQUALS;	break;

    //   case XK_KP_Subtract:
    //   case XK_minus:	rc = KEY_MINUS;		break;

    //   case XK_Shift_L:
    //   case XK_Shift_R:
	// rc = KEY_RSHIFT;
	// break;
	
      case 59:
    //   case XK_Control_R:
	rc = KEY_RCTRL;
	break;
	
    //   case XK_Alt_L:
    //   case XK_Meta_L:
    //   case XK_Alt_R:
    //   case XK_Meta_R:
	// rc = KEY_RALT;
	// break;
	
    //   default:
	// if (rc >= XK_space && rc <= XK_asciitilde)
	//     rc = rc - XK_space + ' ';
	// if (rc >= 'A' && rc <= 'Z')
	//     rc = rc - 'A' + 'a';
	// break;
    }

    return rc;

}

void I_ShutdownGraphics(void)
{
  // Paranoia.
//   image->data = NULL;
}



//
// I_StartFrame
//
void I_StartFrame (void)
{
    // er?

}

static int	lastmousex = 0;
static int	lastmousey = 0;
boolean		mousemoved = false;
boolean		shmFinished;

void I_GetEvent(void)
{

    event_t event;

	//
	int zware_event_type;
	int zware_event_data1;
	int zware_event_data2;
	int zware_event_data3;

    // put event-grabbing stuff in here
    ZwareDoomNextEvent(&zware_event_type, &zware_event_data1, &zware_event_data2, &zware_event_data3);
    switch (zware_event_type)
    {
      case 0x00:
	event.type = ev_keydown;
	event.data1 = xlatekey(zware_event_data1);
	D_PostEvent(&event);
	// fprintf(stderr, "k");
	break;
      case 0x01:
	event.type = ev_keyup;
	event.data1 = xlatekey(zware_event_data1);
	D_PostEvent(&event);
	// fprintf(stderr, "ku");
	break;
    //   case ButtonPress:
	// event.type = ev_mouse;
	// event.data1 =
	//     (X_event.xbutton.state & Button1Mask)
	//     | (X_event.xbutton.state & Button2Mask ? 2 : 0)
	//     | (X_event.xbutton.state & Button3Mask ? 4 : 0)
	//     | (X_event.xbutton.button == Button1)
	//     | (X_event.xbutton.button == Button2 ? 2 : 0)
	//     | (X_event.xbutton.button == Button3 ? 4 : 0);
	// event.data2 = event.data3 = 0;
	// D_PostEvent(&event);
	// // fprintf(stderr, "b");
	// break;
    //   case ButtonRelease:
	// event.type = ev_mouse;
	// event.data1 =
	//     (X_event.xbutton.state & Button1Mask)
	//     | (X_event.xbutton.state & Button2Mask ? 2 : 0)
	//     | (X_event.xbutton.state & Button3Mask ? 4 : 0);
	// // suggest parentheses around arithmetic in operand of |
	// event.data1 =
	//     event.data1
	//     ^ (X_event.xbutton.button == Button1 ? 1 : 0)
	//     ^ (X_event.xbutton.button == Button2 ? 2 : 0)
	//     ^ (X_event.xbutton.button == Button3 ? 4 : 0);
	// event.data2 = event.data3 = 0;
	// D_PostEvent(&event);
	// // fprintf(stderr, "bu");
	// break;
    //   case MotionNotify:
	// event.type = ev_mouse;
	// event.data1 =
	//     (X_event.xmotion.state & Button1Mask)
	//     | (X_event.xmotion.state & Button2Mask ? 2 : 0)
	//     | (X_event.xmotion.state & Button3Mask ? 4 : 0);
	// event.data2 = (X_event.xmotion.x - lastmousex) << 2;
	// event.data3 = (lastmousey - X_event.xmotion.y) << 2;

	// if (event.data2 || event.data3)
	// {
	//     lastmousex = X_event.xmotion.x;
	//     lastmousey = X_event.xmotion.y;
	//     if (X_event.xmotion.x != X_width/2 &&
	// 	X_event.xmotion.y != X_height/2)
	//     {
	// 	D_PostEvent(&event);
	// 	// fprintf(stderr, "m");
	// 	mousemoved = false;
	//     } else
	//     {
	// 	mousemoved = true;
	//     }
	// }
	// break;
	
    //   case Expose:
    //   case ConfigureNotify:
	// break;
	
      default:
	// if (doShm && X_event.type == X_shmeventtype) shmFinished = true;
	break;
    }

}


//
// I_StartTic
//
void I_StartTic (void)
{
    while (ZwareDoomPendingEvent())
	I_GetEvent();

    mousemoved = false;

}


//
// I_UpdateNoBlit
//
void I_UpdateNoBlit (void)
{
    // what is this?
}

//
// I_FinishUpdate
//
void I_FinishUpdate (void)
{
	// draw the image
	ZwareDoomRenderFrame(screens[0], SCREENWIDTH * SCREENHEIGHT);
}


//
// I_ReadScreen
//
void I_ReadScreen (byte* scr)
{
    memcpy (scr, screens[0], SCREENWIDTH*SCREENHEIGHT);
}


//
// Palette stuff.
//
static ZwareDoomColor	colors[256];

void UploadNewPalette(byte *palette)
{

    register int	i;
    register int	c;
    static boolean	firstcall = true;

	// set the X colormap entries
	for (i=0 ; i<256 ; i++)
	{
	c = gammatable[usegamma][*palette++];
	colors[i].red = (c<<8) + c;
	c = gammatable[usegamma][*palette++];
	colors[i].green = (c<<8) + c;
	c = gammatable[usegamma][*palette++];
	colors[i].blue = (c<<8) + c;
	}

	// store the colors to the current colormap
	ZwareDoomSetPalette(colors, 256 * 4);
}

//
// I_SetPalette
//
void I_SetPalette (byte* palette)
{
    UploadNewPalette(palette);
}

void I_InitGraphics(void)
{

    char*		displayname;
    char*		d;
    int			n;
    int			pnum;
    int			x=0;
    int			y=0;
    
    // warning: char format, different type arg
    char		xsign=' ';
    char		ysign=' ';
    
    int			oktodraw;
    unsigned long	attribmask;
    int			valuemask;
    static int		firsttime=1;

    if (!firsttime)
	return;
    firsttime = 0;

    // signal(SIGINT, (void (*)(int)) I_Quit);

    // check for command-line display name
    if ( (pnum=M_CheckParm("-disp")) ) // suggest parentheses around assignment
	displayname = myargv[pnum+1];
    else
	displayname = 0;

    // check if the user wants to grab the mouse (quite unnice)
    grabMouse = !!M_CheckParm("-grabmouse");

    // check for command-line geometry
    if ( (pnum=M_CheckParm("-geom")) ) // suggest parentheses around assignment
    {
	// warning: char format, different type arg 3,5
	n = sscanf(myargv[pnum+1], "%c%d%c%d", &xsign, &x, &ysign, &y);
	
	if (n==2)
	    x = y = 0;
	else if (n==6)
	{
	    if (xsign == '-')
		x = -x;
	    if (ysign == '-')
		y = -y;
	}
	else
	    I_Error("bad -geom parameter");
    }

    // create the main window
	ZwareDoomOpenWindow();
	
	screens[0] = (unsigned char *) malloc (SCREENWIDTH * SCREENHEIGHT);

}
