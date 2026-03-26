// Single-player network stub for Windows port.
// Avoids POSIX socket dependencies; multiplayer is not supported.

#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#include "i_system.h"
#include "d_event.h"
#include "d_net.h"
#include "m_argv.h"
#include "doomstat.h"

#ifdef __GNUG__
#pragma implementation "i_net.h"
#endif
#include "i_net.h"

void (*netget)(void);
void (*netsend)(void);

//
// I_InitNetwork
// Sets up single-player mode only.
//
void I_InitNetwork(void)
{
    doomcom = malloc(sizeof(*doomcom));
    memset(doomcom, 0, sizeof(*doomcom));

    doomcom->ticdup    = 1;
    doomcom->extratics = 0;

    // Single player always
    netgame              = false;
    doomcom->id          = DOOMCOM_ID;
    doomcom->numplayers  = doomcom->numnodes = 1;
    doomcom->deathmatch  = false;
    doomcom->consoleplayer = 0;
}

void I_NetCmd(void)
{
    I_Error("I_NetCmd: networking not supported in Windows single-player build");
}
