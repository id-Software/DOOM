#include "mus2mid.h"
#include <string.h>
#include <stdio.h>
// reads a variable length integer
unsigned long ReadVarLen( char* buffer ) {
	unsigned long value;
	char c;

	if ((value = *buffer++) & 0x80) {
		value &= 0x7f;
		do  {
			value = (value << 7) + ((c = *buffer++) & 0x7f);
		}  while (c & 0x80);
	}
	return value;
}

// Writes a variable length integer to a buffer, and returns bytes written
int WriteVarLen( long value, char* out ) 
{
	long buffer, count = 0;

	buffer = value & 0x7f;
	while ((value >>= 7) > 0) {
		buffer <<= 8;
		buffer += 0x80;
		buffer += (value & 0x7f);
	}

	while (1) {
		++count;
		*out = (char)buffer;
		++out;
		if (buffer & 0x80)
			buffer >>= 8;
		else
			break;
	}
	return count;
}

// writes a byte, and returns the buffer
unsigned char* WriteByte(void* buf, char b)
{
	unsigned char* buffer = (unsigned char*)buf;
	*buffer++ = b;
	return buffer;
}

unsigned char* WriteString(void* buf, const char* str)
{
	unsigned char* buffer = (unsigned char*)buf;
    for(int i = 0; i < sizeof(str); i++)
    {
        *buffer++ = str[i];
    }
	return buffer;
}	

unsigned char* WriteShort(void* b, unsigned short s)
{
	unsigned char* buffer = (unsigned char*)b;
	*buffer++ = (s >> 8);
	*buffer++ = (s & 0x00FF);
	return buffer;
}

unsigned char* WriteInt(void* b, unsigned int i)
{
	unsigned char* buffer = (unsigned char*)b;
	*buffer++ = (i & 0xff000000) >> 24;
	*buffer++ = (i & 0x00ff0000) >> 16;
	*buffer++ = (i & 0x0000ff00) >> 8;
	*buffer++ = (i & 0x000000ff);
	return buffer;
}

// Format - 0(1 track only), 1(1 or more tracks, each play same time), 2(1 or more, each play seperatly)
void Midi_CreateHeader(MidiHeaderChunk_t* header, short format, short track_count,  short division)
{
	WriteString(header->name, "MThd");
	WriteInt(&header->length, 6);
	WriteShort(&header->format, format);
	WriteShort(&header->ntracks, track_count);
	WriteShort(&header->division, division);
}

unsigned char* Midi_WriteTempo(unsigned char* buffer, int tempo)
{
	buffer = WriteByte(buffer, 0x00);	// delta time
	buffer = WriteByte(buffer, 0xff);	// sys command
	buffer = WriteShort(buffer, 0x5103); // command - set tempo
	
	buffer = WriteByte(buffer, tempo & 0x000000ff);
	buffer = WriteByte(buffer, (tempo & 0x0000ff00) >> 8);
	buffer = WriteByte(buffer, (tempo & 0x00ff0000) >> 16);

	return buffer;
}

int Midi_UpdateBytesWritten(int* bytes_written, int to_add, int max)
{
	*bytes_written += to_add;
	if (max && *bytes_written > max)
	{
		return 0;
	}
	return 1;
}

unsigned char MidiMap[] = 
{
	0,				// prog change
	0,				// bank sel
	1,	//2			// mod pot
	0x07,	//3		// volume
	0x0A,	//4		// pan pot
	0x0B,	//5		// expression pot
	0x5B,	//6		// reverb depth
	0x5D,	//7		// chorus depth
	0x40,	//8		// sustain pedal
	0x43,	//9		// soft pedal
	0x78,	//10		// all sounds off
	0x7B,	//11		// all notes off
	0x7E,	//12		// mono(use numchannels + 1)
	0x7F,	//13		// poly
	0x79,	//14	// reset all controllers
};

// The MUS data is stored in little-endian.
unsigned short LittleToNative( const unsigned short value ) {
	return value;
}

int Mus2Midi(unsigned char* bytes, unsigned char* out, int* len)
{


	// mus header and instruments
	MUSheader_t header;
	
	// current position in read buffer
	unsigned char* cur = bytes,* end;

	// Midi header(format 0)
	MidiHeaderChunk_t midiHeader;
	// Midi track header, only 1 needed(format 0)
	MidiTrackChunk_t midiTrackHeader;
	// Stores the position of the midi track header(to change the size)
	unsigned char* midiTrackHeaderOut;
	
	// Delta time for midi event
	int delta_time = 0;
	int temp;
	int channel_volume[MIDI_MAXCHANNELS] = {0};
	int bytes_written = 0;
	int channelMap[MIDI_MAXCHANNELS], currentChannel = 0;

	// read the mus header
	memcpy(&header, cur, sizeof(header));
	cur += sizeof(header);

	header.scoreLen = LittleToNative( header.scoreLen );
	header.scoreStart = LittleToNative( header.scoreStart );
	header.channels = LittleToNative( header.channels );
	header.sec_channels = LittleToNative( header.sec_channels );
	header.instrCnt = LittleToNative( header.instrCnt );
	header.dummy = LittleToNative( header.dummy );
	
	// only 15 supported
	if (header.channels > MIDI_MAXCHANNELS - 1)
		return 0;

	// Map channel 15 to 9(percussions)
	for (temp = 0; temp < MIDI_MAXCHANNELS; ++temp) {
		channelMap[temp] = -1;
		channel_volume[temp] = 0x40;
	}
	channelMap[15] = 9;

	// Get current position, and end of position
	cur = bytes + header.scoreStart;
	end = cur + header.scoreLen;

	// Write out midi header
	Midi_CreateHeader(&midiHeader, 0, 1, 0x0059);
	Midi_UpdateBytesWritten(&bytes_written, MIDIHEADERSIZE, *len);
	memcpy(out, &midiHeader, MIDIHEADERSIZE);	// cannot use sizeof(packs it to 16 bytes)
	out += MIDIHEADERSIZE;
	 
	// Store this position, for later filling in the midiTrackHeader
	Midi_UpdateBytesWritten(&bytes_written, sizeof(midiTrackHeader), *len);
	midiTrackHeaderOut = out;
	out += sizeof(midiTrackHeader);
	

	// microseconds per quarter note(yikes)
	Midi_UpdateBytesWritten(&bytes_written, 7, *len);
	out = Midi_WriteTempo(out, 0x001aa309);
	
	// Percussions channel starts out at full volume
	Midi_UpdateBytesWritten(&bytes_written, 4, *len);
	out = WriteByte(out, 0x00);
	out = WriteByte(out, 0xB9);
	out = WriteByte(out, 0x07);
	out = WriteByte(out, 127);
	
	// Main Loop
	while (cur < end) {
		uint8_t channel; 
		char event;
		unsigned char temp_buffer[32];	// temp buffer for current iterator
		unsigned char *out_local = temp_buffer;
		char status, bit1, bit2, bitc = 2;
		
		// Read in current bit
		event		= *cur++;
		channel		= (event & 15);		// current channel
		
		// Write variable length delta time
		out_local += WriteVarLen(delta_time, (char*)out_local);
		
		if (channelMap[channel] < 0) {
			// Set all channels to 127 volume
			out_local = WriteByte(out_local, 0xB0 + currentChannel);
			out_local = WriteByte(out_local, 0x07);
			out_local = WriteByte(out_local, 127);
			out_local = WriteByte(out_local, 0x00);

			channelMap[channel] = currentChannel++;
			if (currentChannel == 9)
				++currentChannel;
		}

		status = channelMap[channel];

		// Handle ::g->events
		switch ((event & 122) >> 4)
		{
		default:
			break;
		case MUSEVENT_KEYOFF:
			status |=  0x80;
			bit1 = *cur++;
			bit2 = 0x40;
			break;
		case MUSEVENT_KEYON:
			status |= 0x90;
			bit1 = *cur & 127;
			if (*cur++ & 128)	// volume bit?
				channel_volume[channelMap[channel]] = *cur++;
			bit2 = channel_volume[channelMap[channel]];
			break;
		case MUSEVENT_PITCHWHEEL:
			status |= 0xE0;
			bit1 = (*cur & 1) >> 6;
			bit2 = (*cur++ >> 1) & 127;
			break;
		case MUSEVENT_CHANNELMODE:
			status |= 0xB0;
			bit1 = MidiMap[*cur++];
			bit2 = (*cur++ == 12) ? header.channels + 1 : 0x00;
			break;
		case MUSEVENT_CONTROLLERCHANGE:
			if (*cur == 0) {
				cur++;
				status |= 0xC0;
				bit1 = *cur++;
				bitc = 1;
			} else {
				status |= 0xB0;
				bit1 = MidiMap[*cur++];
				bit2 = *cur++;
			}
			break;
		case 5:	// Unknown
			break;
		case MUSEVENT_END:	// End
			status = 0xff;
			bit1 = 0x2f;
			bit2 = 0x00;
			break;
		case 7:	// Unknown
			break;
		}

		// Write it out
		out_local = WriteByte(out_local, status);
		out_local = WriteByte(out_local, bit1);
		if (bitc == 2) 
			out_local = WriteByte(out_local, bit2);
			

		// Write out temp stuff
		if (out_local != temp_buffer)
		{
			Midi_UpdateBytesWritten(&bytes_written, out_local - temp_buffer, *len);
			memcpy(out, temp_buffer, out_local - temp_buffer);
			out += out_local - temp_buffer;
		}

		if (event & 128) {
			delta_time = 0;
			do {
				delta_time = delta_time * 128 + (*cur & 127);
			} while ((*cur++ & 128));
		} else {
			delta_time = 0;
		}
	}

	// Write out track header
	WriteString(midiTrackHeader.name, "MTrk");
	WriteInt(&midiTrackHeader.length, out - midiTrackHeaderOut - sizeof(midiTrackHeader));
	memcpy(midiTrackHeaderOut, &midiTrackHeader, sizeof(midiTrackHeader));
	
	// Store length written
	*len = bytes_written;
	/*{
		FILE* file = f o pen("d:\\test.midi", "wb");
		fwrite(midiTrackHeaderOut - sizeof(MidiHeaderChunk_t), bytes_written, 1, file);
		fclose(file);
	}*/
	return 1;
}