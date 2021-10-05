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
// DESCRIPTION:
//	Status bar code.
//	Does the face/direction indicator animatin.
//	Does palette indicators as well (red pain/berserk, bright pickup)
//
//-----------------------------------------------------------------------------

#ifndef __STSTUFF_H__
#define __STSTUFF_H__

#include "doomtype.h"
#include "d_event.h"

// Size of statusbar.
// Now sensitive for scaling.
#define ST_HEIGHT	32*SCREEN_MUL
#define ST_WIDTH	SCREENWIDTH
#define ST_Y		(SCREENHEIGHT - ST_HEIGHT)


//
// STATUS BAR
//

// Called by main loop.
boolean ST_Responder (event_t* ev);

// Called by main loop.
void ST_Ticker (void);

// Called by main loop.
void ST_Drawer (boolean fullscreen, boolean refresh);

// Called when the console player is spawned on each level.
void ST_Start (void);

// Called by startup code.
void ST_Init (void);



// States for status bar code.
typedef enum
{
    AutomapState,
    FirstPersonState
    
} st_stateenum_t;


// States for the chat code.
typedef enum
{
    StartChatState,
    WaitDestState,
    GetChatState
    
} st_chatstateenum_t;


boolean ST_Responder(event_t* ev);



#endif
//-----------------------------------------------------------------------------
//
// $Log:$
//
//-----------------------------------------------------------------------------
