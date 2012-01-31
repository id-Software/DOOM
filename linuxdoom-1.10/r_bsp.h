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
//	Refresh module, BSP traversal and handling.
//
//-----------------------------------------------------------------------------


#ifndef __R_BSP__
#define __R_BSP__

#ifdef __GNUG__
#pragma interface
#endif


extern seg_t*		curline;
extern side_t*		sidedef;
extern line_t*		linedef;
extern sector_t*	frontsector;
extern sector_t*	backsector;

extern int		rw_x;
extern int		rw_stopx;

extern boolean		segtextured;

// false if the back side is the same plane
extern boolean		markfloor;		
extern boolean		markceiling;

extern boolean		skymap;

extern drawseg_t	drawsegs[MAXDRAWSEGS];
extern drawseg_t*	ds_p;

extern lighttable_t**	hscalelight;
extern lighttable_t**	vscalelight;
extern lighttable_t**	dscalelight;


typedef void (*drawfunc_t) (int start, int stop);


// BSP?
void R_ClearClipSegs (void);
void R_ClearDrawSegs (void);


void R_RenderBSPNode (int bspnum);


#endif
//-----------------------------------------------------------------------------
//
// $Log:$
//
//-----------------------------------------------------------------------------
