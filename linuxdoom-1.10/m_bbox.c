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
//	Main loop menu stuff.
//	Random number LUT.
//	Default Config File.
//	PCX Screenshots.
//
//-----------------------------------------------------------------------------

static const char
rcsid[] = "$Id: m_bbox.c,v 1.1 1997/02/03 22:45:10 b1 Exp $";


#ifdef __GNUG__
#pragma implementation "m_bbox.h"
#endif
#include "m_bbox.h"




void M_ClearBox (fixed_t *box)
{
    box[BOXTOP] = box[BOXRIGHT] = MININT;
    box[BOXBOTTOM] = box[BOXLEFT] = MAXINT;
}

void
M_AddToBox
( fixed_t*	box,
  fixed_t	x,
  fixed_t	y )
{
    if (x<box[BOXLEFT])
	box[BOXLEFT] = x;
    else if (x>box[BOXRIGHT])
	box[BOXRIGHT] = x;
    if (y<box[BOXBOTTOM])
	box[BOXBOTTOM] = y;
    else if (y>box[BOXTOP])
	box[BOXTOP] = y;
}





