//#define DOOM2

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <process.h>
#include <conio.h>
#include <dos.h>

#include "doomnet.h"
//#include "ipxstr.h"
#include "ipx_frch.h"		// FRENCH VERSION

doomcom_t doomcom;
int            vectorishooked;
void interrupt (*olddoomvect) (void);



/*
=============
=
= LaunchDOOM
=
These fields in doomcom should be filled in before calling:

     short     numnodes;      // console is allways node 0
     short     ticdup;             // 1 = no duplication, 2-5 = dup for 
slow nets
     short     extratics;          // 1 = send a backup tic in every 
packet

	 short     consoleplayer; // 0-3 = player number
	 short     numplayers;         // 1-4
	 short     angleoffset;   // 1 = left, 0 = center, -1 = right
	 short     drone;              // 1 = drone
=============
*/

void LaunchDOOM (void)
{
	 char *newargs[99];
	 char adrstring[10];
	 long      flatadr;

// prepare for DOOM
	 doomcom.id = DOOMCOM_ID;

// hook the interrupt vector
	 olddoomvect = getvect (doomcom.intnum);
     setvect (doomcom.intnum,(void interrupt (*)(void))MK_FP(_CS, 
(int)NetISR));
     vectorishooked = 1;

// build the argument list for DOOM, adding a -net &doomcom
     memcpy (newargs, _argv, (_argc+1)*2);
	 newargs[_argc] = "-net";
	 flatadr = (long)_DS*16 + (unsigned)&doomcom;
	 sprintf (adrstring,"%lu",flatadr);
	 newargs[_argc+1] = adrstring;
	 newargs[_argc+2] = NULL;

	 if (!access("doom2.exe",0))
		spawnv  (P_WAIT, "doom2", newargs);
	 else
		spawnv  (P_WAIT, "doom", newargs);

	 #ifdef DOOM2
	 printf (STR_RETURNED"\n");
	 #else
	 printf ("Returned from DOOM\n");
	 #endif
}
