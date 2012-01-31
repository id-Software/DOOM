// Emacs style mode select   -*- C++ -*- 
//-----------------------------------------------------------------------------
//
// $Id: linux.c,v 1.3 1997/01/26 07:45:01 b1 Exp $
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
// $Log: linux.c,v $
// Revision 1.3  1997/01/26 07:45:01  b1
// 2nd formatting run, fixed a few warnings as well.
//
// Revision 1.2  1997/01/21 19:00:01  b1
// First formatting run:
//  using Emacs cc-mode.el indentation for C++ now.
//
// Revision 1.1  1997/01/19 17:22:45  b1
// Initial check in DOOM sources as of Jan. 10th, 1997
//
//
// DESCRIPTION:
//	UNIX, soundserver for Linux i386.
//
//-----------------------------------------------------------------------------

static const char rcsid[] = "$Id: linux.c,v 1.3 1997/01/26 07:45:01 b1 Exp $";


#include <stdlib.h>
#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>

#include <linux/soundcard.h>

#include "soundsrv.h"

int	audio_fd;

void
myioctl
( int	fd,
  int	command,
  int*	arg )
{   
    int		rc;
    extern int	errno;
    
    rc = ioctl(fd, command, arg);  
    if (rc < 0)
    {
	fprintf(stderr, "ioctl(dsp,%d,arg) failed\n", command);
	fprintf(stderr, "errno=%d\n", errno);
	exit(-1);
    }
}

void I_InitMusic(void)
{
}

void
I_InitSound
( int	samplerate,
  int	samplesize )
{

    int i;
                
    audio_fd = open("/dev/dsp", O_WRONLY);
    if (audio_fd<0)
        fprintf(stderr, "Could not open /dev/dsp\n");
         
                     
    i = 11 | (2<<16);                                           
    myioctl(audio_fd, SNDCTL_DSP_SETFRAGMENT, &i);
                    
    myioctl(audio_fd, SNDCTL_DSP_RESET, 0);
    i=11025;
    myioctl(audio_fd, SNDCTL_DSP_SPEED, &i);
    i=1;    
    myioctl(audio_fd, SNDCTL_DSP_STEREO, &i);
            
    myioctl(audio_fd, SNDCTL_DSP_GETFMTS, &i);
    if (i&=AFMT_S16_LE)    
        myioctl(audio_fd, SNDCTL_DSP_SETFMT, &i);
    else
        fprintf(stderr, "Could not play signed 16 data\n");

}

void
I_SubmitOutputBuffer
( void*	samples,
  int	samplecount )
{
    write(audio_fd, samples, samplecount*4);
}

void I_ShutdownSound(void)
{

    close(audio_fd);

}

void I_ShutdownMusic(void)
{
}
