// Null sound implementation - no audio output
// Stubs out all i_sound.h interface functions.

#include "i_sound.h"
#include "sounds.h"
#include "doomdef.h"

// SNDSERV support: define the server filename (unused in null backend)
// This is declared extern in i_sound.h and referenced from m_misc.c when
// SNDSERV is defined in doomdef.h.
char* sndserver_filename = "sndserver";

void I_InitSound(void)    {}
void I_UpdateSound(void)  {}
void I_SubmitSound(void)  {}
void I_ShutdownSound(void){}
void I_SetChannels(void)  {}

int  I_GetSfxLumpNum(sfxinfo_t *sfxinfo)                          { (void)sfxinfo; return -1; }
int  I_StartSound(int id, int vol, int sep, int pitch, int pri)   { (void)id;(void)vol;(void)sep;(void)pitch;(void)pri; return -1; }
void I_StopSound(int handle)                                       { (void)handle; }
int  I_SoundIsPlaying(int handle)                                  { (void)handle; return 0; }
void I_UpdateSoundParams(int handle, int vol, int sep, int pitch)  { (void)handle;(void)vol;(void)sep;(void)pitch; }

void I_InitMusic(void)             {}
void I_ShutdownMusic(void)         {}
void I_SetMusicVolume(int volume)  { (void)volume; }
void I_PauseSong(int handle)       { (void)handle; }
void I_ResumeSong(int handle)      { (void)handle; }
int  I_RegisterSong(void *data)    { (void)data; return 0; }
void I_PlaySong(int handle, int looping) { (void)handle;(void)looping; }
void I_StopSong(int handle)        { (void)handle; }
void I_UnRegisterSong(int handle)  { (void)handle; }
