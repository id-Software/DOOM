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

#include <SFML/Graphics.h>
#include <SFML/System.h>
#include <SFML/Window.h>

#include <stdarg.h>
#include <sys/time.h>
#include <sys/types.h>
#include <sys/socket.h>

#include <netinet/in.h>
#include <errno.h>
#include <signal.h>

#include "doomstat.h"
#include "i_system.h"
#include "v_video.h"
#include "m_argv.h"
#include "d_main.h"

#include "doomdef.h"

#define POINTER_WARP_COUNTDOWN	1

sfRenderWindow* window;
sfTexture* texture;
sfSprite* image;
int		X_width;
int		X_height;

sfEvent event;


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


//converts sfKey to key
int sfKeyConvert(void)
{
	int rc;
    switch(event.key.code)
    {
      case sfKeyLeft:	rc = KEY_LEFTARROW;	break;
      case sfKeyRight:	rc = KEY_RIGHTARROW;	break;
      case sfKeyDown:	rc = KEY_DOWNARROW;	break;
      case sfKeyUp:	rc = KEY_UPARROW;	break;
      case sfKeyEscape:	rc = KEY_ESCAPE;	break;
      case sfKeyReturn:	rc = KEY_ENTER;		break;
      case sfKeyTab:	rc = KEY_TAB;		break;
      case sfKeyF1:	rc = KEY_F1;		break;
      case sfKeyF2:	rc = KEY_F2;		break;
      case sfKeyF3:	rc = KEY_F3;		break;
      case sfKeyF4:	rc = KEY_F4;		break;
      case sfKeyF5:	rc = KEY_F5;		break;
      case sfKeyF6:	rc = KEY_F6;		break;
      case sfKeyF7:	rc = KEY_F7;		break;
      case sfKeyF8:	rc = KEY_F8;		break;
      case sfKeyF9:	rc = KEY_F9;		break;
      case sfKeyF10:	rc = KEY_F10;		break;
      case sfKeyF11:	rc = KEY_F11;		break;
      case sfKeyF12:	rc = KEY_F12;		break;
	
      case sfKeyBackspace:
      case sfKeyDelete:	rc = KEY_BACKSPACE;	break;

      case sfKeyPause:	rc = KEY_PAUSE;		break;

      case sfKeyEqual:	rc = KEY_EQUALS;	break;

      case sfKeySubtract:	rc = KEY_MINUS;		break;

      case sfKeyLShift:
      case sfKeyRShift:
		rc = KEY_RSHIFT;
	break;
	
      case sfKeyLControl:
      case sfKeyRControl:
		rc = KEY_RCTRL;
	break;
	
      case sfKeyLAlt:
		rc = KEY_LALT;
	  case sfKeyRAlt:
	  	rc = KEY_RALT;
	break;
	
      default:
	if (rc >= sfKeySpace && rc <= sfKeyTilde)
	    rc = rc - sfKeySpace + ' ';
	if (rc >= 'A' && rc <= 'Z')
	    rc = rc - 'A' + 'a';
	break;
    }

    return rc;

}

void I_ShutdownGraphics(void)
{
	if(window)
	{
		sfRenderWindow_close(window);
	}
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
	event_t d_event;
    while(sfRenderWindow_pollEvent(window, &event))
	{
		switch (event.type)
		{
		case sfEvtKeyPressed:
			d_event.type = ev_keydown;
			d_event.data1 = sfKeyConvert();
			D_PostEvent(&d_event);
			break;

		case sfEvtClosed:
			I_Quit();
			break;

		default:
			break;
		}
	}

}


//
// I_StartTic
//
void I_StartTic (void)
{

    if (!window)
		return;
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

static byte	colors[768];

void I_FinishUpdate (void)
{
    static int	lasttic;
    int		tics;
    int		i;
    // UNUSED static unsigned char *bigscreen=0;

    	byte argb_buffer[SCREENWIDTH * SCREENHEIGHT * 4];
		for(i = 0; i < SCREENWIDTH * SCREENHEIGHT; i++)
		{
			byte colorIndex = screens[0][i];
            byte r = colors[(3 * colorIndex)];
            byte g = colors[(3 * colorIndex) + 1];
            byte b = colors[(3 * colorIndex) + 2];

			argb_buffer[4 * i] = r;
            argb_buffer[4 * (i) + 1] = g;
            argb_buffer[4 * (i) + 2] = b;
            argb_buffer[4 * (i) + 3] = 255;
		}

		sfTexture_updateFromPixels(texture, argb_buffer, SCREENWIDTH, SCREENHEIGHT, 0, 0);
		sfSprite_setTexture(image, texture, true);
		sfRenderWindow_drawSprite(window, image, NULL);
		sfRenderWindow_display(window);

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

void UploadNewPalette(byte *palette)
{

//     register int	i;
//     register int	c;
//     static boolean	firstcall = true;

// #ifdef __cplusplus
//     if (X_visualinfo.c_class == PseudoColor && X_visualinfo.depth == 8)
// #else
//     if (X_visualinfo.class == PseudoColor && X_visualinfo.depth == 8)
// #endif
// 	{
// 	    // initialize the colormap
// 	    if (firstcall)
// 	    {
// 		firstcall = false;
// 		for (i=0 ; i<256 ; i++)
// 		{
// 		    colors[i].pixel = i;
// 		    colors[i].flags = DoRed|DoGreen|DoBlue;
// 		}
// 	    }

// 	    // set the X colormap entries
// 	    for (i=0 ; i<256 ; i++)
// 	    {
// 		c = gammatable[usegamma][*palette++];
// 		colors[i].red = (c<<8) + c;
// 		c = gammatable[usegamma][*palette++];
// 		colors[i].green = (c<<8) + c;
// 		c = gammatable[usegamma][*palette++];
// 		colors[i].blue = (c<<8) + c;
// 	    }

// 	    // store the colors to the current colormap
// 	    XStoreColors(X_display, cmap, colors, 256);

// 	}
}

//
// I_SetPalette
//
void I_SetPalette (byte* palette)
{
	memcpy(colors, palette, 768);
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

    signal(SIGINT, (void (*)(int)) I_Quit);

    if (M_CheckParm("-2"))
	multiply = 2;

    if (M_CheckParm("-3"))
	multiply = 3;

    if (M_CheckParm("-4"))
	multiply = 4;

    X_width = SCREENWIDTH * multiply;
    X_height = SCREENHEIGHT * multiply;

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

	printf("starting sfwindow..\n");

	sfVideoMode mode;
	mode.width = X_width;
	mode.height = X_height;

	window = sfRenderWindow_create(mode, displayname, sfDefaultStyle, NULL);
	sfRenderWindow_setFramerateLimit(window, 35);
	texture = sfTexture_create(X_width, X_height);

	image = sfSprite_create();
	//screens[0] is the framebuffer btw

	
	screens[0] = (unsigned char *) malloc (SCREENWIDTH * SCREENHEIGHT);

}
