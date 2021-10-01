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
//	System interface for sound.
//
//-----------------------------------------------------------------------------

static const char
rcsid[] = "$Id: i_unix.c,v 1.5 1997/02/03 22:45:10 b1 Exp $";
#include "mus2mid.h"

#include <stdio.h>
#include <stdlib.h>
#include <stdarg.h>
#include <unistd.h>
#include <sys/stat.h>
#include <math.h>
#include "cvector.h"
#include <sys/time.h>
#include <sys/types.h>
#include <errno.h>
#ifndef LINUX
#endif

#include <fcntl.h>
#include <unistd.h>

// Timer stuff. Experimental.
#include <time.h>
#include <fluidsynth.h>
#include <signal.h>

#include "z_zone.h"

#include "i_system.h"
#include "i_sound.h"
#include "m_argv.h"
#include "m_misc.h"
#include "w_wad.h"

#include "doomdef.h"



// Update all 30 millisecs, approx. 30fps synchronized.
// Linux resolution is allegedly 10 millisecs,
//  scale is microseconds.
#define SOUND_INTERVAL     500

// Get the interrupt. Set duration in millisecs.
int I_SoundSetTimer( int duration_of_tick );
void I_SoundDelTimer( void );

#include <SFML/Audio.h>


// A quick hack to establish a protocol between
// synchronous mix buffer updates and asynchronous
// audio writes. Probably redundant with gametic.
static int flag = 0;

// The number of internal mixing channels,
//  the samples calculated for each mixing step,
//  the size of the 16bit, 2 hardware channel (stereo)
//  mixing buffer, and the samplerate of the raw data.


// Needed for calling the actual sound output.
#define SAMPLECOUNT		512
#define NUM_CHANNELS		8
// It is 2 for 16bit, and 2 for two channels.
#define BUFMUL                  4
#define MIXBUFFERSIZE		(SAMPLECOUNT*BUFMUL)

#define SAMPLERATE		11025	// Hz
#define MUSSAMPLERATE 22050
#define SAMPLESIZE		2   	// 16bit

// The actual lengths of all sound effects.
int 		lengths[NUMSFX];

// The actual output device.
int	audio_fd;

// The global mixing buffer.
// Basically, samples from all active internal channels
//  are modifed and added, and stored in the buffer
//  that is submitted to the audio device.
signed short	mixbuffer[MIXBUFFERSIZE];


// The channel step amount...
unsigned int	channelstep[NUM_CHANNELS];
// ... and a 0.16 bit remainder of last step.
unsigned int	channelstepremainder[NUM_CHANNELS];


// The channel data pointers, start and end.
unsigned char*	channels[NUM_CHANNELS];
unsigned char*	channelsend[NUM_CHANNELS];


// Time/gametic that the channel started playing,
//  used to determine oldest, which automatically
//  has lowest priority.
// In case number of active sounds exceeds
//  available channels.
int		channelstart[NUM_CHANNELS];

// The sound in channel handles,
//  determined on registration,
//  might be used to unregister/stop/modify,
//  currently unused.
int 		channelhandles[NUM_CHANNELS];

// SFX id of the playing sound effect.
// Used to catch duplicates (like chainsaw).
int		channelids[NUM_CHANNELS];			

// Pitch to stepping lookup, unused.
int		steptable[256];

// Volume lookups.
int		vol_lookup[128*256];

// Hardware left and right channel volume lookup.
int*		channelleftvol_lookup[NUM_CHANNELS];
int*		channelrightvol_lookup[NUM_CHANNELS];





//
// This function loads the sound data from the WAD lump,
//  for single sound.
//
void*
getsfx
( char*         sfxname,
  int*          len )
{
    unsigned char*      sfx;
    unsigned char*      paddedsfx;
    int                 i;
    int                 size;
    int                 paddedsize;
    char                name[20];
    int                 sfxlump;

    
    // Get the sound data from the WAD, allocate lump
    //  in zone memory.
    sprintf(name, "ds%s", sfxname);

    // Now, there is a severe problem with the
    //  sound handling, in it is not (yet/anymore)
    //  gamemode aware. That means, sounds from
    //  DOOM II will be requested even with DOOM
    //  shareware.
    // The sound list is wired into sounds.c,
    //  which sets the external variable.
    // I do not do runtime patches to that
    //  variable. Instead, we will use a
    //  default sound for replacement.
    if ( W_CheckNumForName(name) == -1 )
      sfxlump = W_GetNumForName("dspistol");
    else
      sfxlump = W_GetNumForName(name);
    
    size = W_LumpLength( sfxlump );

    // Debug.
    // fprintf( stderr, "." );
    //fprintf( stderr, " -loading  %s (lump %d, %d bytes)\n",
    //	     sfxname, sfxlump, size );
    //fflush( stderr );
    
    sfx = (unsigned char*)W_CacheLumpNum( sfxlump, PU_STATIC );

    // Pads the sound effect out to the mixing buffer size.
    // The original realloc would interfere with zone memory.
    paddedsize = ((size-8 + (SAMPLECOUNT-1)) / SAMPLECOUNT) * SAMPLECOUNT;

    // Allocate from zone memory.
    paddedsfx = (unsigned char*)malloc( paddedsize+8);
    // ddt: (unsigned char *) realloc(sfx, paddedsize+8);
    // This should interfere with zone memory handling,
    //  which does not kick in in the soundserver.

    // Now copy and pad.
    memcpy(  paddedsfx, sfx, size );
    for (i=size ; i<paddedsize+8 ; i++)
        paddedsfx[i] = 128;

    // Remove the cached lump.
  //  Z_Free( sfx );
    
    
    // Preserve padded length.
    *len = paddedsize;

    // Return allocated padded data.
    return (void *) (paddedsfx + 8);
}









 
void I_SetSfxVolume(int volume)
{
  snd_SfxVolume = volume;
}


#include <SFML/Audio.h>
sfSound* sounds[NUMSFX];
sfSound* music;

int cursong = -1;

void I_SetMusicVolume(int volume)
{
  snd_MusicVolume = volume;
  sfSound_setVolume(music, (100.0 / 15) * (float)volume);
}


//
// Retrieve the raw data lump index
//  for a given SFX name.
//
int I_GetSfxLumpNum(sfxinfo_t* sfx)
{
    char namebuf[9];
    sprintf(namebuf, "ds%s", sfx->name);
    return W_GetNumForName(namebuf);
}



int
I_StartSound
( int		id,
  int		vol,
  int		sep,
  int		pitch,
  int		priority )
{
  sfSound* sound = sounds[id];
  
  sfSound_setVolume(sound,  (100.0 / 15.0) * (float)snd_SfxVolume);
  if(snd_DoPitchShift)
  {
    sfSound_setPitch(sound, (1.0 / 128.0) * (float)pitch);
  }
  sfSound_play(sound);
}



void I_StopSound (int handle)
{
  // You need the handle returned by StartSound.
  // Would be looping all channels,
  //  tracking down the handle,
  //  an setting the channel to zero.
  
  // UNUSED.
  handle = 0;
}


int I_SoundIsPlaying(int handle)
{
    // Ouch.
    return gametic < handle;
}




void I_UpdateSound( void )
{
}



void
I_UpdateSoundParams
( int	handle,
  int	vol,
  int	sep,
  int	pitch)
{

  handle = vol = sep = pitch = 0;
}




void I_ShutdownSound(void)
{    

}





void
I_InitSound()
{ 
  I_InitMusic();
  for (int i=1 ; i<NUMSFX ; i++)
  { 
    if (!S_sfx[i].link)
    {
      // Load data from WAD file.
      S_sfx[i].data = getsfx( S_sfx[i].name, &lengths[i] );

      sfUint16 data[lengths[i]];


      byte* raw = (byte*)S_sfx[i].data;

      for(int k = 0; k < lengths[i]; k++)
      {
        data[k] = (short)((raw[k] - 128) << 8);
      }


      //convert to sfaudio
      sfSoundBuffer* soundbuffer = sfSoundBuffer_createFromSamples(data, lengths[i], 1, SAMPLERATE);
      sfSound* sound = sfSound_create();
      sfSound_setBuffer(sound, soundbuffer);

      sounds[i] = sound;
    }	
  }

}




//fluidsynth stuff
fluid_settings_t *settings;
fluid_synth_t *synth;
fluid_audio_driver_t *adriver;

void I_LoadSoundFont(char* filename)
{
  int sfont_id = fluid_synth_sfload(synth, filename, 1);
}

void I_InitMusic(void)
{
  music = sfSound_create();
  settings = new_fluid_settings();
  int i, key;
  settings = new_fluid_settings();


  // since this is a non-realtime scenario, there is no need to pin the sample data


  synth = new_fluid_synth(settings);
  fluid_synth_set_sample_rate(synth, MUSSAMPLERATE);

  fluid_settings_setstr(settings, "audio.driver", "alsa");
  fluid_settings_setint(settings, "audio.period-size", 0);


  


}

void I_ShutdownMusic(void)	{ }

static int	looping=0;
static int	musicdies=-1;


int songid = 0;
void I_PlaySong(int handle, int looping)
{
  
  printf("playing song %d\n", handle);
  musicdies = gametic + TICRATE*30;
  cursong = handle;
  sfSound_setLoop(music, looping);
  sfSound_play(music);
  
}

void I_PauseSong (int handle)
{
  sfSound_pause(music);
}



void I_ResumeSong (int handle)
{
  sfSound_play(music);
}

void I_StopSong(int handle)
{
  sfSound_stop(music);
}

void I_UnRegisterSong(int handle)
{
  // UNUSED.
  handle = 0;
}



void MidiShit(sfInt16* vec)
{ 
  sfSoundBuffer* sbuffer = sfSoundBuffer_createFromSamples((void*)vec, cvector_size(vec), 2, MUSSAMPLERATE);
  sfSound_setBuffer(music, sbuffer);
  I_SetMusicVolume(snd_MusicVolume);
  cvector_free(vec);
}

#define SFMIDI_LOADERFRAMES 1024
int I_RegisterSong(void* data)
{
  int length;
  char* midi = malloc(1024 * 1024);


   Mus2Midi(data, midi, &length);

  sfInt16* vec = NULL;
  
  int dataSize;
  int read;
  int outputsize = SFMIDI_LOADERFRAMES*2;


  for(int i = 0; i < outputsize; i++)
  {
    cvector_push_back(vec, 0);
  }

  fluid_player_t *player = new_fluid_player(synth);
  fluid_player_add_mem(player, midi, length);
  fluid_player_play(player);
  int fuck;
  while(true)
  {
    read = fluid_synth_write_s16(synth, SFMIDI_LOADERFRAMES, &vec[dataSize], 0, 2, &vec[dataSize], 1, 2);
  
    if (read == FLUID_FAILED ||
        fluid_player_get_status(player) != FLUID_PLAYER_PLAYING)
      break;

    dataSize += SFMIDI_LOADERFRAMES * 2;

    if(outputsize - dataSize < SFMIDI_LOADERFRAMES * 2)
    {
      outputsize += SFMIDI_LOADERFRAMES * 2;

      for(int i = cvector_size(vec); i < outputsize; i++)
      {
        cvector_push_back(vec, 0);
      }
    }
  }
  delete_fluid_player(player);

  MidiShit(vec);


  free(midi);

  songid++;
  return songid - 1;
}

// Is the song playing?
int I_QrySongPlaying(int handle)
{
  // UNUSED.
  handle = 0;
  return looping || musicdies > gametic;
}
