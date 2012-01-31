// Emacs style mode select   -*- C++ -*- 
//-----------------------------------------------------------------------------
//
// $Id: soundst.h,v 1.3 1997/01/29 22:40:45 b1 Exp $
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
// $Log: soundst.h,v $
// Revision 1.3  1997/01/29 22:40:45  b1
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
//	Sound (utility) related. Hang on.
//	See gensounds.h and gensounds.c for what soundst.h is made of.
//
//-----------------------------------------------------------------------------

#ifndef __SOUNDSTH__
#define __SOUNDSTH__

#define S_MAX_VOLUME		127

// when to clip out sounds
// Doesn't fit the large outdoor areas.
#define S_CLIPPING_DIST		(1200*0x10000)

// when sounds should be max'd out
#define S_CLOSE_DIST		(200*0x10000)


#define S_ATTENUATOR		((S_CLIPPING_DIST-S_CLOSE_DIST)>>FRACBITS)

#define NORM_PITCH     		128
#define NORM_PRIORITY		64
#define NORM_VOLUME    		snd_MaxVolume

#define S_PITCH_PERTURB		1
#define NORM_SEP			128
#define S_STEREO_SWING		(96*0x10000)

// % attenuation from front to back
#define S_IFRACVOL			30

#define NA				0
#define S_NUMCHANNELS		2




//
// MusicInfo struct.
//
typedef struct
{
    // up to 6-character name
    char*	name;

    // lump number of music
    int		lumpnum;
    
    // music data
    void*	data;

    // music handle once registered
    int handle;
    
} musicinfo_t;



//
// SoundFX struct.
//
typedef struct sfxinfo_struct	sfxinfo_t;

struct sfxinfo_struct
{
    // up to 6-character name
    char*	name;

    // Sfx singularity (only one at a time)
    int		singularity;

    // Sfx priority
    int		priority;

    // referenced sound if a link
    sfxinfo_t*	link;

    // pitch if a link
    int		pitch;

    // volume if a link
    int		volume;

    // sound data
    void*	data;

    // this is checked every second to see if sound
    // can be thrown out (if 0, then decrement, if -1,
    // then throw out, if > 0, then it's in use)
    int		usefulness;

    // lump number of sfx
    int		lumpnum;		
};



typedef struct
{
    // sound information (if null, channel avail.)
    sfxinfo_t*	sfxinfo;

    // origin of sound
    void*	origin;

    // handle of the sound being played
    int		handle;
    
} channel_t;



enum
{
    Music,
    Sfx,
    SfxLink
};

enum
{
    PC=1,
    Adlib=2,
    SB=4,
    Midi=8
}; // cards available

enum
{
    sfxThrowOut=-1,
    sfxNotUsed=0
};


//
// Initialize the sound code at start of level
//
void S_Start(void);

//
// Start sound for thing at <origin>
//  using <sound_id> from sounds.h
//
extern void
S_StartSound
( void*		origin,
  int		sound_id );



// Will start a sound at a given volume.
extern void
S_StartSoundAtVolume
( void*		origin,
  int		sound_id,
  int		volume );


// Stop sound for thing at <origin>
extern void S_StopSound(void* origin);

// Start music using <music_id> from sounds.h
extern void S_StartMusic(int music_id);

// Start music using <music_id> from sounds.h,
//  and set whether looping
extern void
S_ChangeMusic
( int		music_id,
  int		looping );


// Stops the music
extern void S_StopMusic(void);

void S_PauseSound(void);
void S_ResumeSound(void);


//
// Updates music & sounds
//
extern void S_UpdateSounds(void* listener);

void S_SetMusicVolume(int volume);
void S_SetSfxVolume(int volume);

//
// Initializes sound stuff, including volume
//
void
S_Init
( int 	,
  int    );



//
// SOUND IO
//
#define FREQ_LOW		0x40
#define FREQ_NORM		0x80
#define FREQ_HIGH		0xff


void I_SetMusicVolume(int volume);
void I_SetSfxVolume(int volume);

//
//  MUSIC I/O
//
void I_PauseSong(int handle);
void I_ResumeSong(int handle);

//
// Called by anything that wishes to start music.
//  plays a song, and when the song is done,
//  starts playing it again in an endless loop.
// Horrible thing to do, considering.
void
I_PlaySong
( int		handle,
  int		looping );


// stops a song over 3 seconds.
void I_StopSong(int handle);

// registers a song handle to song data
int I_RegisterSong(void *data);

// see above then think backwards
void I_UnRegisterSong(int handle);

// is the song playing?
int I_QrySongPlaying(int handle);


//
//  SFX I/O
//
void I_SetChannels(int channels);

int I_GetSfxLumpNum (sfxinfo_t*);


// Starts a sound in a particular sound channel.
int
I_StartSound
( int		id,
  void*		data,
  int		vol,
  int		sep,
  int		pitch,
  int		priority );


// Updates the volume, separation,
//  and pitch of a sound channel.
void
I_UpdateSoundParams
( int		handle,
  int		vol,
  int		sep,
  int		pitch );


// Stops a sound channel.
void I_StopSound(int handle);

// Called by S_*()'s to see if a channel is still playing.
// Returns 0 if no longer playing, 1 if playing.
int I_SoundIsPlaying(int handle);


// the complete set of sound effects
extern sfxinfo_t	S_sfx[];

// the complete set of music
extern musicinfo_t	S_music[];

#endif
