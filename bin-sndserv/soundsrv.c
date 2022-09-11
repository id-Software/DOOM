// Emacs style mode select   -*- C++ -*- 
//-----------------------------------------------------------------------------
//
// $Id: soundsrv.c,v 1.3 1997/01/29 22:40:44 b1 Exp $
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
// $Log: soundsrv.c,v $
// Revision 1.3  1997/01/29 22:40:44  b1
// Reformatting, S (sound) module files.
//
// Revision 1.2  1997/01/21 19:00:07  b1
// First formatting run:
//  using Emacs cc-mode.el indentation for C++ now.
//
// Revision 1.1  1997/01/19 17:22:50  b1
// Initial check in DOOM sources as of Jan. 10th, 1997
//
//
// DESCRIPTION:
//	UNIX soundserver, run as a separate process,
//	 started by DOOM program.
//	Originally conceived fopr SGI Irix,
//	 mostly used with Linux voxware.
//
//-----------------------------------------------------------------------------


static const char rcsid[] = "$Id: soundsrv.c,v 1.3 1997/01/29 22:40:44 b1 Exp $";



#include <math.h>
#include <sys/types.h>
#include <stdio.h>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <unistd.h>
#include <stdlib.h>
#include <malloc.h>
#include <sys/stat.h>
#include <sys/time.h>

#include "sounds.h"
#include "soundsrv.h"
#include "wadread.h"



//
// Department of Redundancy Department.
//
typedef struct wadinfo_struct
{
    // should be IWAD
    char	identification[4];	
    int		numlumps;
    int		infotableofs;
    
} wadinfo_t;


typedef struct filelump_struct
{
    int		filepos;
    int		size;
    char	name[8];
    
} filelump_t;


// an internal time keeper
static int	mytime = 0;

// number of sound effects
int 		numsounds;

// longest sound effect
int 		longsound;

// lengths of all sound effects
int 		lengths[NUMSFX];

// mixing buffer
signed short	mixbuffer[MIXBUFFERSIZE];

// file descriptor of sfx device
int		sfxdevice;			

// file descriptor of music device
int 		musdevice;			

// the channel data pointers
unsigned char*	channels[8];

// the channel step amount
unsigned int	channelstep[8];

// 0.16 bit remainder of last step
unsigned int	channelstepremainder[8];

// the channel data end pointers
unsigned char*	channelsend[8];

// time that the channel started playing
int		channelstart[8];

// the channel handles
int 		channelhandles[8];

// the channel left volume lookup
int*		channelleftvol_lookup[8];

// the channel right volume lookup
int*		channelrightvol_lookup[8];

// sfx id of the playing sound effect
int		channelids[8];			

int		snd_verbose=1;

int		steptable[256];

int		vol_lookup[128*256];

static void derror(char* msg)
{
    fprintf(stderr, "error: %s\n", msg);
    exit(-1);
}

int mix(void)
{

    register int		dl;
    register int		dr;
    register unsigned int	sample;
    
    signed short*		leftout;
    signed short*		rightout;
    signed short*		leftend;
    
    int				step;

    leftout = mixbuffer;
    rightout = mixbuffer+1;
    step = 2;

    leftend = mixbuffer + SAMPLECOUNT*step;

    // mix into the mixing buffer
    while (leftout != leftend)
    {

	dl = 0;
	dr = 0;

	if (channels[0])
	{
	    sample = *channels[0];
	    dl += channelleftvol_lookup[0][sample];
	    dr += channelrightvol_lookup[0][sample];
	    channelstepremainder[0] += channelstep[0];
	    channels[0] += channelstepremainder[0] >> 16;
	    channelstepremainder[0] &= 65536-1;

	    if (channels[0] >= channelsend[0])
		channels[0] = 0;
	}

	if (channels[1])
	{
	    sample = *channels[1];
	    dl += channelleftvol_lookup[1][sample];
	    dr += channelrightvol_lookup[1][sample];
	    channelstepremainder[1] += channelstep[1];
	    channels[1] += channelstepremainder[1] >> 16;
	    channelstepremainder[1] &= 65536-1;

	    if (channels[1] >= channelsend[1])
		channels[1] = 0;
	}

	if (channels[2])
	{
	    sample = *channels[2];
	    dl += channelleftvol_lookup[2][sample];
	    dr += channelrightvol_lookup[2][sample];
	    channelstepremainder[2] += channelstep[2];
	    channels[2] += channelstepremainder[2] >> 16;
	    channelstepremainder[2] &= 65536-1;

	    if (channels[2] >= channelsend[2])
		channels[2] = 0;
	}
	
	if (channels[3])
	{
	    sample = *channels[3];
	    dl += channelleftvol_lookup[3][sample];
	    dr += channelrightvol_lookup[3][sample];
	    channelstepremainder[3] += channelstep[3];
	    channels[3] += channelstepremainder[3] >> 16;
	    channelstepremainder[3] &= 65536-1;

	    if (channels[3] >= channelsend[3])
		channels[3] = 0;
	}
	
	if (channels[4])
	{
	    sample = *channels[4];
	    dl += channelleftvol_lookup[4][sample];
	    dr += channelrightvol_lookup[4][sample];
	    channelstepremainder[4] += channelstep[4];
	    channels[4] += channelstepremainder[4] >> 16;
	    channelstepremainder[4] &= 65536-1;

	    if (channels[4] >= channelsend[4])
		channels[4] = 0;
	}
	
	if (channels[5])
	{
	    sample = *channels[5];
	    dl += channelleftvol_lookup[5][sample];
	    dr += channelrightvol_lookup[5][sample];
	    channelstepremainder[5] += channelstep[5];
	    channels[5] += channelstepremainder[5] >> 16;
	    channelstepremainder[5] &= 65536-1;

	    if (channels[5] >= channelsend[5])
		channels[5] = 0;
	}
	
	if (channels[6])
	{
	    sample = *channels[6];
	    dl += channelleftvol_lookup[6][sample];
	    dr += channelrightvol_lookup[6][sample];
	    channelstepremainder[6] += channelstep[6];
	    channels[6] += channelstepremainder[6] >> 16;
	    channelstepremainder[6] &= 65536-1;

	    if (channels[6] >= channelsend[6])
		channels[6] = 0;
	}
	if (channels[7])
	{
	    sample = *channels[7];
	    dl += channelleftvol_lookup[7][sample];
	    dr += channelrightvol_lookup[7][sample];
	    channelstepremainder[7] += channelstep[7];
	    channels[7] += channelstepremainder[7] >> 16;
	    channelstepremainder[7] &= 65536-1;

	    if (channels[7] >= channelsend[7])
		channels[7] = 0;
	}

	// Has been char instead of short.
	// if (dl > 127) *leftout = 127;
	// else if (dl < -128) *leftout = -128;
	// else *leftout = dl;

	// if (dr > 127) *rightout = 127;
	// else if (dr < -128) *rightout = -128;
	// else *rightout = dr;
	
	if (dl > 0x7fff)
	    *leftout = 0x7fff;
	else if (dl < -0x8000)
	    *leftout = -0x8000;
	else
	    *leftout = dl;

	if (dr > 0x7fff)
	    *rightout = 0x7fff;
	else if (dr < -0x8000)
	    *rightout = -0x8000;
	else
	    *rightout = dr;

	leftout += step;
	rightout += step;

    }
    return 1;
}



void
grabdata
( int		c,
  char**	v )
{
    int		i;
    char*	name;
    char*	doom1wad;
    char*	doomwad;
    char*	doomuwad;
    char*	doom2wad;
    char*	doom2fwad;
    // Now where are TNT and Plutonia. Yuck.
    
    //	char *home;
    char*	doomwaddir;

    doomwaddir = getenv("DOOMWADDIR");

    if (!doomwaddir)
	doomwaddir = ".";

    doom1wad = malloc(strlen(doomwaddir)+1+9+1);
    sprintf(doom1wad, "%s/doom1.wad", doomwaddir);

    doom2wad = malloc(strlen(doomwaddir)+1+9+1);
    sprintf(doom2wad, "%s/doom2.wad", doomwaddir);

    doom2fwad = malloc(strlen(doomwaddir)+1+10+1);
    sprintf(doom2fwad, "%s/doom2f.wad", doomwaddir);
    
    doomuwad = malloc(strlen(doomwaddir)+1+8+1);
    sprintf(doomuwad, "%s/doomu.wad", doomwaddir);
    
    doomwad = malloc(strlen(doomwaddir)+1+8+1);
    sprintf(doomwad, "%s/doom.wad", doomwaddir);

    //	home = getenv("HOME");
    //	if (!home)
    //	  derror("Please set $HOME to your home directory");
    //	sprintf(basedefault, "%s/.doomrc", home);


    for (i=1 ; i<c ; i++)
    {
	if (!strcmp(v[i], "-quiet"))
	{
	    snd_verbose = 0;
	}
    }

    numsounds = NUMSFX;
    longsound = 0;

    if (! access(doom2fwad, R_OK) )
	name = doom2fwad;
    else if (! access(doom2wad, R_OK) )
	name = doom2wad;
    else if (! access(doomuwad, R_OK) )
	name = doomuwad;
    else if (! access(doomwad, R_OK) )
	name = doomwad;
    else if (! access(doom1wad, R_OK) )
	name = doom1wad;
    // else if (! access(DEVDATA "doom2.wad", R_OK) )
    //   name = DEVDATA "doom2.wad";
    //   else if (! access(DEVDATA "doom.wad", R_OK) )
    //   name = DEVDATA "doom.wad";
    else
    {
	fprintf(stderr, "Could not find wadfile anywhere\n");
	exit(-1);
    }

    
    openwad(name);
    if (snd_verbose)
	fprintf(stderr, "loading from [%s]\n", name);

    for (i=1 ; i<NUMSFX ; i++)
    {
	if (!S_sfx[i].link)
	{
	    S_sfx[i].data = getsfx(S_sfx[i].name, &lengths[i]);
	    if (longsound < lengths[i]) longsound = lengths[i];
	} else {
	    S_sfx[i].data = S_sfx[i].link->data;
	    lengths[i] = lengths[(S_sfx[i].link - S_sfx)/sizeof(sfxinfo_t)];
	}
	// test only
	//  {
	//  int fd;
	//  char name[10];
	//  sprintf(name, "sfx%d", i);
	//  fd = open(name, O_WRONLY|O_CREAT, 0644);
	//  write(fd, S_sfx[i].data, lengths[i]);
	//  close(fd);
	//  }
    }

}

static struct timeval		last={0,0};
//static struct timeval		now;

static struct timezone		whocares;

void updatesounds(void)
{

    mix();
    I_SubmitOutputBuffer(mixbuffer, SAMPLECOUNT);

}

int
addsfx
( int		sfxid,
  int		volume,
  int		step,
  int		seperation )
{
    static unsigned short	handlenums = 0;
 
    int		i;
    int		rc = -1;
    
    int		oldest = mytime;
    int		oldestnum = 0;
    int		slot;
    int		rightvol;
    int		leftvol;

    // play these sound effects
    //  only one at a time
    if ( sfxid == sfx_sawup
	 || sfxid == sfx_sawidl
	 || sfxid == sfx_sawful
	 || sfxid == sfx_sawhit
	 || sfxid == sfx_stnmov
	 || sfxid == sfx_pistol )
    {
	for (i=0 ; i<8 ; i++)
	{
	    if (channels[i] && channelids[i] == sfxid)
	    {
		channels[i] = 0;
		break;
	    }
	}
    }

    for (i=0 ; i<8 && channels[i] ; i++)
    {
	if (channelstart[i] < oldest)
	{
	    oldestnum = i;
	    oldest = channelstart[i];
	}
    }

    if (i == 8)
	slot = oldestnum;
    else
	slot = i;

    channels[slot] = (unsigned char *) S_sfx[sfxid].data;
    channelsend[slot] = channels[slot] + lengths[sfxid];

    if (!handlenums)
	handlenums = 100;
    
    channelhandles[slot] = rc = handlenums++;
    channelstep[slot] = step;
    channelstepremainder[slot] = 0;
    channelstart[slot] = mytime;

    // (range: 1 - 256)
    seperation += 1;

    // (x^2 seperation)
    leftvol =
	volume - (volume*seperation*seperation)/(256*256);

    seperation = seperation - 257;

    // (x^2 seperation)
    rightvol =
	volume - (volume*seperation*seperation)/(256*256);	

    // sanity check
    if (rightvol < 0 || rightvol > 127)
	derror("rightvol out of bounds");
    
    if (leftvol < 0 || leftvol > 127)
	derror("leftvol out of bounds");
    
    // get the proper lookup table piece
    //  for this volume level
    channelleftvol_lookup[slot] = &vol_lookup[leftvol*256];
    channelrightvol_lookup[slot] = &vol_lookup[rightvol*256];

    channelids[slot] = sfxid;

    return rc;

}


void outputushort(int num)
{

    static unsigned char	buff[5] = { 0, 0, 0, 0, '\n' };
    static char*		badbuff = "xxxx\n";

    // outputs a 16-bit # in hex or "xxxx" if -1.
    if (num < 0)
    {
	write(1, badbuff, 5);
    }
    else
    {
	buff[0] = num>>12;
	buff[0] += buff[0] > 9 ? 'a'-10 : '0';
	buff[1] = (num>>8) & 0xf;
	buff[1] += buff[1] > 9 ? 'a'-10 : '0';
	buff[2] = (num>>4) & 0xf;
	buff[2] += buff[2] > 9 ? 'a'-10 : '0';
	buff[3] = num & 0xf;
	buff[3] += buff[3] > 9 ? 'a'-10 : '0';
	write(1, buff, 5);
    }
}

void initdata(void)
{

    int		i;
    int		j;
    
    int*	steptablemid = steptable + 128;

    for (i=0 ;
	 i<sizeof(channels)/sizeof(unsigned char *) ;
	 i++)
    {
	channels[i] = 0;
    }
    
    gettimeofday(&last, &whocares);

    for (i=-128 ; i<128 ; i++)
	steptablemid[i] = pow(2.0, (i/64.0))*65536.0;

    // generates volume lookup tables
    //  which also turn the unsigned samples
    //  into signed samples
    // for (i=0 ; i<128 ; i++)
    // for (j=0 ; j<256 ; j++)
    // vol_lookup[i*256+j] = (i*(j-128))/127;
    
    for (i=0 ; i<128 ; i++)
	for (j=0 ; j<256 ; j++)
	    vol_lookup[i*256+j] = (i*(j-128)*256)/127;

}




void quit(void)
{
    I_ShutdownMusic();
    I_ShutdownSound();
    exit(0);
}



fd_set		fdset;
fd_set		scratchset;



int
main
( int		c,
  char**	v )
{

    int		done = 0;
    int		rc;
    int		nrc;
    int		sndnum;
    int		handle = 0;
    
    unsigned char	commandbuf[10];
    struct timeval	zerowait = { 0, 0 };

    
    int 	step;
    int 	vol;
    int		sep;
    
    int		i;
    int		waitingtofinish=0;

    // get sound data
    grabdata(c, v);

    // init any data
    initdata();		

    I_InitSound(11025, 16);

    I_InitMusic();

    if (snd_verbose)
	fprintf(stderr, "ready\n");
    
    // parse commands and play sounds until done
    FD_ZERO(&fdset);
    FD_SET(0, &fdset);

    while (!done)
    {
	mytime++;

	if (!waitingtofinish)
	{
	    do {
		scratchset = fdset;
		rc = select(FD_SETSIZE, &scratchset, 0, 0, &zerowait);

		if (rc > 0)
		{
		    //	fprintf(stderr, "select is true\n");
		    // got a command
		    nrc = read(0, commandbuf, 1);

		    if (!nrc)
		    {
			done = 1;
			rc = 0;
		    }
		    else
		    {
			if (snd_verbose)
			    fprintf(stderr, "cmd: %c", commandbuf[0]);

			switch (commandbuf[0])
			{
			  case 'p':
			    // play a new sound effect
			    read(0, commandbuf, 9);

			    if (snd_verbose)
			    {
				commandbuf[9]=0;
				fprintf(stderr, "%s\n", commandbuf);
			    }

			    commandbuf[0] -=
				commandbuf[0]>='a' ? 'a'-10 : '0';
			    commandbuf[1] -=
				commandbuf[1]>='a' ? 'a'-10 : '0';
			    commandbuf[2] -=
				commandbuf[2]>='a' ? 'a'-10 : '0';
			    commandbuf[3] -=
				commandbuf[3]>='a' ? 'a'-10 : '0';
			    commandbuf[4] -=
				commandbuf[4]>='a' ? 'a'-10 : '0';
			    commandbuf[5] -=
				commandbuf[5]>='a' ? 'a'-10 : '0';
			    commandbuf[6] -=
				commandbuf[6]>='a' ? 'a'-10 : '0';
			    commandbuf[7] -=
				commandbuf[7]>='a' ? 'a'-10 : '0';

			    //	p<snd#><step><vol><sep>
			    sndnum = (commandbuf[0]<<4) + commandbuf[1];
			    step = (commandbuf[2]<<4) + commandbuf[3];
			    step = steptable[step];
			    vol = (commandbuf[4]<<4) + commandbuf[5];
			    sep = (commandbuf[6]<<4) + commandbuf[7];

			    handle = addsfx(sndnum, vol, step, sep);
			    // returns the handle
			    //	outputushort(handle);
			    break;
			    
			  case 'q':
			    read(0, commandbuf, 1);
			    waitingtofinish = 1; rc = 0;
			    break;
			    
			  case 's':
			  {
			      int fd;
			      read(0, commandbuf, 3);
			      commandbuf[2] = 0;
			      fd = open((char*)commandbuf, O_CREAT|O_WRONLY, 0644);
			      commandbuf[0] -= commandbuf[0]>='a' ? 'a'-10 : '0';
			      commandbuf[1] -= commandbuf[1]>='a' ? 'a'-10 : '0';
			      sndnum = (commandbuf[0]<<4) + commandbuf[1];
			      write(fd, S_sfx[sndnum].data, lengths[sndnum]);
			      close(fd);
			  }
			  break;
			  
			  default:
			    fprintf(stderr, "Did not recognize command\n");
			    break;
			}
		    }
		}
		else if (rc < 0)
		{
		    quit();
		}
	    } while (rc > 0);
	}

	updatesounds();

	if (waitingtofinish)
	{
	    for(i=0 ; i<8 && !channels[i] ; i++);
	    
	    if (i==8)
		done=1;
	}

    }

    quit();
    return 0;
}
