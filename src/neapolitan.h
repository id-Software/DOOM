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
// DESCRIPTION:  Neapolitan Doom specific vars
//
//-----------------------------------------------------------------------------
#define NEAPOLITAN_VERSION "0.02"
#define NEAPOLITAN_SAVEFILE "neapolitan.conf"

#ifndef __NEAPOLITAN__
#define __NEAPOLITAN__

#include <SFML/Window.h>
#include "doomtype.h"
void N_WriteConfig();
void N_LoadConfig(void);
void N_RebindKeys();
void N_MonsterHeightFix(int choice);
extern boolean fixInfiniteMonsterHeight;
extern boolean useMouse;
extern char bindnames[7][32];
extern int keybindCount;
extern char keynames[sfKeyCount][16];
extern int keybinds[];
extern int unbindablekeys[];
extern int unbindableKeyCount;
extern int snd_DoPitchShift;

#endif