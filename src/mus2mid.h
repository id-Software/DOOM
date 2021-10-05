//I don't for the life of me understand MIDI so I borrowed some SFML-DOOM code and patched it to work with C
//https://github.com/JonnyPtn/SFML-DOOM/blob/master/src/mus2midi.cpp


#define MUSEVENT_KEYOFF	0
#define MUSEVENT_KEYON	1
#define MUSEVENT_PITCHWHEEL	2
#define MUSEVENT_CHANNELMODE	3
#define MUSEVENT_CONTROLLERCHANGE	4
#define MUSEVENT_END	6
#define MIDI_MAXCHANNELS	16
#define MIDIHEADERSIZE 14
#include <stdint.h>

typedef struct tagMUSheader_t {
    char    ID[4];          // identifier "MUS" 0x1A
    unsigned short    scoreLen;
    unsigned short    scoreStart;
    unsigned short    channels;	// count of primary channels
    unsigned short    sec_channels;	// count of secondary channels
    unsigned short    instrCnt;
    unsigned short    dummy;
    //// variable-length part starts here
} MUSheader_t;
typedef struct tagMidiHeaderChunk_t {
    char name[4];
    int  length;

    short format;			// make 0
    short ntracks;			// make 1
    short division;			// 0xe250??
} MidiHeaderChunk_t;
typedef struct tagMidiTrackChunk_t {
    char name[4];
    int	length;
} MidiTrackChunk_t;


unsigned char* WriteByte(void* buf, char b);
unsigned char* WriteInt(void* b, unsigned int i);
void Midi_CreateHeader(MidiHeaderChunk_t* header, short format, short track_count,  short division);
unsigned char* Midi_WriteTempo(unsigned char* buffer, int tempo);
int Midi_UpdateBytesWritten(int* bytes_written, int to_add, int max);
int Mus2Midi(unsigned char* bytes, unsigned char* out, int* len);