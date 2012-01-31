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
//
//-----------------------------------------------------------------------------

static const char
rcsid[] = "$Id:$";

// We are referring to sprite numbers.
#include "info.h"

#ifdef __GNUG__
#pragma implementation "d_items.h"
#endif
#include "d_items.h"


//
// PSPRITE ACTIONS for waepons.
// This struct controls the weapon animations.
//
// Each entry is:
//   ammo/amunition type
//  upstate
//  downstate
// readystate
// atkstate, i.e. attack/fire/hit frame
// flashstate, muzzle flash
//
weaponinfo_t	weaponinfo[NUMWEAPONS] =
{
    {
	// fist
	am_noammo,
	S_PUNCHUP,
	S_PUNCHDOWN,
	S_PUNCH,
	S_PUNCH1,
	S_NULL
    },	
    {
	// pistol
	am_clip,
	S_PISTOLUP,
	S_PISTOLDOWN,
	S_PISTOL,
	S_PISTOL1,
	S_PISTOLFLASH
    },	
    {
	// shotgun
	am_shell,
	S_SGUNUP,
	S_SGUNDOWN,
	S_SGUN,
	S_SGUN1,
	S_SGUNFLASH1
    },
    {
	// chaingun
	am_clip,
	S_CHAINUP,
	S_CHAINDOWN,
	S_CHAIN,
	S_CHAIN1,
	S_CHAINFLASH1
    },
    {
	// missile launcher
	am_misl,
	S_MISSILEUP,
	S_MISSILEDOWN,
	S_MISSILE,
	S_MISSILE1,
	S_MISSILEFLASH1
    },
    {
	// plasma rifle
	am_cell,
	S_PLASMAUP,
	S_PLASMADOWN,
	S_PLASMA,
	S_PLASMA1,
	S_PLASMAFLASH1
    },
    {
	// bfg 9000
	am_cell,
	S_BFGUP,
	S_BFGDOWN,
	S_BFG,
	S_BFG1,
	S_BFGFLASH1
    },
    {
	// chainsaw
	am_noammo,
	S_SAWUP,
	S_SAWDOWN,
	S_SAW,
	S_SAW1,
	S_NULL
    },
    {
	// super shotgun
	am_shell,
	S_DSGUNUP,
	S_DSGUNDOWN,
	S_DSGUN,
	S_DSGUN1,
	S_DSGUNFLASH1
    },	
};








