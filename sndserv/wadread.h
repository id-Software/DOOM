// Emacs style mode select   -*- C++ -*- 
//-----------------------------------------------------------------------------
//
// $Id: wadread.h,v 1.3 1997/01/30 19:54:23 b1 Exp $
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
//
// $Log: wadread.h,v $
// Revision 1.3  1997/01/30 19:54:23  b1
// Final reformatting run. All the remains (ST, W, WI, Z).
//
// Revision 1.2  1997/01/21 19:00:10  b1
// First formatting run:
//  using Emacs cc-mode.el indentation for C++ now.
//
// Revision 1.1  1997/01/19 17:22:52  b1
// Initial check in DOOM sources as of Jan. 10th, 1997
//
//
// DESCRIPTION:
//	WAD and Lump I/O, the second.
//	This time for soundserver only.
//	Welcome to Department of Redundancy Department.
//	 (Yeah, I said that elsewhere already).
//	Note: makes up for a nice w_wad.h.
//
//-----------------------------------------------------------------------------

#ifndef __WADREAD_H__
#define __WADREAD_H__

//
//  Opens the wadfile specified.
// Must be called before any calls to  loadlump() or getsfx().
//

void openwad(char* wadname);

//
//  Gets a sound effect from the wad file.  The pointer points to the
//  start of the data.  Returns a 0 if the sfx was not
//  found.  Sfx names should be no longer than 6 characters.  All data is
//  rounded up in size to the nearest MIXBUFFERSIZE and is padded out with
//  0x80's.  Returns the data length in len.
//

void*
getsfx
( char*		sfxname,
  int*		len );

#endif
