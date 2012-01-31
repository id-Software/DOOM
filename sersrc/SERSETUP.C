// sersetup.c
#define DOOM2
#include "sersetup.h"
//#include "serstr.h"
#include "ser_frch.h"		// FRENCH VERSION
#include "DoomNet.h"

extern	que_t		inque, outque;

void jump_start( void );
extern int 	uart;

int			usemodem;
char		startup[256], shutdown[256], baudrate[256];

extern		int baudbits;

void ModemCommand (char *str);

int		myargc;
char	**myargv;

//======================================
//
// I_Error
//
//======================================
void I_Error(char *string)
{
	printf("%s\n",string);
	exit(1);
}

/*
================
=
= write_buffer
=
================
*/

void write_buffer( char *buffer, unsigned int count )
{
	int	i;

// if this would overrun the buffer, throw everything else out
	if (outque.head-outque.tail+count > QUESIZE)
		outque.tail = outque.head;

	while (count--)
		write_byte (*buffer++);

	if ( INPUT( uart + LINE_STATUS_REGISTER ) & 0x40)
		jump_start();
}


/*
=================
=
= Error
=
= For abnormal program terminations
=
=================
*/

void Error (char *error, ...)
{
	va_list argptr;

	if (usemodem)
	{
		printf ("\n");
		printf ("\n"STR_DROPDTR"\n");

		OUTPUT(uart+MODEM_CONTROL_REGISTER, INPUT(uart+MODEM_CONTROL_REGISTER)&~MCR_DTR);
		delay (1250);
		OUTPUT( uart + MODEM_CONTROL_REGISTER, INPUT( uart + MODEM_CONTROL_REGISTER ) | MCR_DTR );
		ModemCommand("+++");
		delay (1250);
		ModemCommand(shutdown);
		delay (1250);

	}

	ShutdownPort ();

	if (vectorishooked)
		setvect (doomcom.intnum,olddoomvect);

	if (error)
	{
		va_start (argptr,error);
		vprintf (error,argptr);
		va_end (argptr);
		printf ("\n");
		exit (1);
	}

	printf (STR_CLEANEXIT"\n");
	exit (0);
}


/*
================
=
= ReadPacket
=
================
*/

#define MAXPACKET	512
#define	FRAMECHAR	0x70

char	packet[MAXPACKET];
int		packetlen;
int		inescape;
int		newpacket;

boolean ReadPacket (void)
{
	int	c;

// if the buffer has overflowed, throw everything out

	if (inque.head-inque.tail > QUESIZE - 4)	// check for buffer overflow
	{
		inque.tail = inque.head;
		newpacket = true;
		return false;
	}

	if (newpacket)
	{
		packetlen = 0;
		newpacket = 0;
	}

	do
	{
		c = read_byte ();
		if (c < 0)
			return false;		// haven't read a complete packet
//printf ("%c",c);
		if (inescape)
		{
			inescape = false;
			if (c!=FRAMECHAR)
			{
				newpacket = 1;
				return true;	// got a good packet
			}
		}
		else if (c==FRAMECHAR)
		{
			inescape = true;
			continue;			// don't know yet if it is a terminator
		}						// or a literal FRAMECHAR

		if (packetlen >= MAXPACKET)
			continue;			// oversize packet
		packet[packetlen] = c;
		packetlen++;
	} while (1);

}


/*
=============
=
= WritePacket
=
=============
*/



void WritePacket (char *buffer, int len)
{
	int		b;
	char	static localbuffer[MAXPACKET*2+2];

	b = 0;
	if (len > MAXPACKET)
		return;

	while (len--)
	{
		if (*buffer == FRAMECHAR)
			localbuffer[b++] = FRAMECHAR;	// escape it for literal
		localbuffer[b++] = *buffer++;
	}

	localbuffer[b++] = FRAMECHAR;
	localbuffer[b++] = 0;

	write_buffer (localbuffer, b);
}


/*
=============
=
= NetISR
=
=============
*/

void interrupt NetISR (void)
{
	if (doomcom.command == CMD_SEND)
	{
//I_ColorBlack (0,0,63);
		WritePacket ((char *)&doomcom.data, doomcom.datalength);
	}
	else if (doomcom.command == CMD_GET)
	{
//I_ColorBlack (63,63,0);

		if (ReadPacket () && packetlen <= sizeof(doomcom.data) )
		{
			doomcom.remotenode = 1;
			doomcom.datalength = packetlen;
			memcpy (&doomcom.data, &packet, packetlen);
		}
		else
			doomcom.remotenode = -1;

	}
//I_ColorBlack (0,0,0);
}




/*
=================
=
= Connect
=
= Figures out who is player 0 and 1
=================
*/

void Connect (void)
{
	struct time	time;
	int			oldsec;
	int			localstage, remotestage;
	char		str[20];
	char		idstr[7];
	char		remoteidstr[7];
	unsigned long		idnum;
	int			i;
	
//
// wait for a good packet
//
	printf (STR_ATTEMPT"\n");

//
// build a (hopefully) unique id string by hashing up the current milliseconds
// and the interrupt table
//
	if (CheckParm ("-player1"))
		idnum = 0;
	else if (CheckParm ("-player2"))
		idnum = 999999;
	else
	{
		gettime (&time);
		idnum = time.ti_sec*100+time.ti_hund;
		for (i=0 ; i<512 ; i++)
			idnum += ((unsigned far *)0)[i];
		idnum %= 1000000;
	}
	
	idstr[0] = '0' + idnum/ 100000l;
	idnum -= (idstr[0]-'0')*100000l;
	idstr[1] = '0' + idnum/ 10000l;
	idnum -= (idstr[1]-'0')*10000l;
	idstr[2] = '0' + idnum/ 1000l;
	idnum -= (idstr[2]-'0')*1000l;
	idstr[3] = '0' + idnum/ 100l;
	idnum -= (idstr[3]-'0')*100l;
	idstr[4] = '0' + idnum/ 10l;
	idnum -= (idstr[4]-'0')*10l;
	idstr[5] = '0' + idnum;
	idstr[6] = 0;
	
//
// sit in a loop until things are worked out
//
// the packet is:  ID000000_0
// the first field is the idnum, the second is the acknowledge stage
// ack stage starts out 0, is bumped to 1 after the other computer's id
// is known, and is bumped to 2 after the other computer has raised to 1
//
	oldsec = -1;
	localstage = remotestage = 0;

	do
	{
		while ( bioskey(1) )
		{
			if ( (bioskey (0) & 0xff) == 27)
				Error ("\n\n"STR_NETABORT);
		}

		if (ReadPacket ())
		{
			packet[packetlen] = 0;
			printf ("read : %s\n",packet);
			if (packetlen != 10)
				continue;
			if (strncmp(packet,"ID",2) )
				continue;
			if (!strncmp (packet+2,idstr,6))
				Error ("\n\n"STR_DUPLICATE);
			strncpy (remoteidstr,packet+2,6);
				
			remotestage = packet[9] - '0';
			localstage = remotestage+1;
			oldsec = -1;
		}

		gettime (&time);
		if (time.ti_sec != oldsec)
		{
			oldsec = time.ti_sec;
			sprintf (str,"ID%s_%i",idstr,localstage);
			WritePacket (str,strlen(str));
			printf ("wrote: %s\n",str);
		}

	} while (localstage < 2);

//
// decide who is who
//
	if (strcmp(remoteidstr,idstr) > 0)
		doomcom.consoleplayer = 0;
	else
		doomcom.consoleplayer = 1;
	

//
// flush out any extras
//
	while (ReadPacket ())
	;
}



/*
==============
=
= ModemCommand
=
==============
*/

void ModemCommand (char *str)
{
	int		i,l;
	
	printf (STR_MODEMCMD,str);
	l = strlen(str);
	for (i=0 ; i<l ; i++)
	{
		write_buffer (str+i,1);
		printf ("%c",str[i]);
		delay (100);
	}

	write_buffer ("\r",1);
	printf ("\n");
}


/*
==============
=
= ModemResponse
=
= Waits for OK, RING, CONNECT, etc
==============
*/

char	response[80];

void ModemResponse (char *resp)
{
	int		c;
	int		respptr;

	do
	{
		printf (STR_MODEMRESP);
		respptr=0;
		do
		{
			while ( bioskey(1) )
			{
				if ( (bioskey (0) & 0xff) == 27)
					Error ("\n"STR_RESPABORT);
			}
			c = read_byte ();
			if (c==-1)
				continue;
			if (c=='\n' || respptr == 79)
			{
				response[respptr] = 0;
				printf ("%s\n",response);
				break;
			}
			if (c>=' ')
			{
				response[respptr] = c;
				respptr++;
			}
		} while (1);

	} while (strncmp(response,resp,strlen(resp)));
}


/*
=============
=
= ReadLine
=
=============
*/

void ReadLine (FILE *f, char *dest)
{
	int	c;

	do
	{
		c = fgetc (f);
		if (c == EOF || c == '\r' || c == '\n')
			break;
		*dest++ = c;
	} while (1);
	*dest = 0;
}


/*
=============
=
= ReadModemCfg
=
=============
*/

void ReadModemCfg (void)
{
	int		mcr;
	FILE	*f;
	unsigned	baud;

	f = fopen ("modem.cfg","r");
	if (!f)
		Error (STR_CANTREAD);
	ReadLine (f, startup);
	ReadLine (f, shutdown);
	ReadLine (f, baudrate);
	fclose (f);

	baud = atol(baudrate);
	if (baud)
		baudbits = 115200l/baud;

	usemodem = true;
}


/*
=============
=
= Dial
=
=============
*/

void Dial (void)
{
	char	cmd[80];
	int		p;

	ModemCommand(startup);
	ModemResponse ("OK");

	printf ("\n"STR_DIALING"\n\n");
	p = CheckParm ("-dial");
	sprintf (cmd,"ATDT%s",myargv[p+1]);

	ModemCommand(cmd);
	ModemResponse (STR_CONNECT);
	doomcom.consoleplayer = 1;
}


/*
=============
=
= Answer
=
=============
*/

void Answer (void)
{
	ModemCommand(startup);
	ModemResponse ("OK");
	printf ("\n"STR_WAITRING"\n\n");

	ModemResponse (STR_RING);
	ModemCommand ("ATA");
	ModemResponse (STR_CONNECT);

	doomcom.consoleplayer = 0;
}

//========================================================
//
//	Find a Response File
//
//========================================================
void FindResponseFile (void)
{
	int		i;
	#define	MAXARGVS	100

	for (i = 1;i < myargc;i++)
		if (myargv[i][0] == '@')
		{
			FILE *		handle;
			int		size;
			int		k;
			int		index;
			int		indexinfile;
			char	*infile;
			char	*file;
			char	*moreargs[20];
			char	*firstargv;

			// READ THE RESPONSE FILE INTO MEMORY
			handle = fopen (&myargv[i][1],"rb");
			if (!handle)
				I_Error (STR_NORESP);
			printf("Found response file \"%s\"!\n",strupr(&myargv[i][1]));
			fseek (handle,0,SEEK_END);
			size = ftell(handle);
			fseek (handle,0,SEEK_SET);
			file = malloc (size);
			fread (file,size,1,handle);
			fclose (handle);

			// KEEP ALL CMDLINE ARGS FOLLOWING @RESPONSEFILE ARG
			for (index = 0,k = i+1; k < myargc; k++)
				moreargs[index++] = myargv[k];

			firstargv = myargv[0];
			myargv = malloc(sizeof(char *)*MAXARGVS);
			memset(myargv,0,sizeof(char *)*MAXARGVS);
			myargv[0] = firstargv;

			infile = file;
			indexinfile = k = 0;
			indexinfile++;	// SKIP PAST ARGV[0] (KEEP IT)
			do
			{
				myargv[indexinfile++] = infile+k;
				while(k < size &&
					((*(infile+k)>= ' '+1) && (*(infile+k)<='z')))
					k++;
				*(infile+k) = 0;
				while(k < size &&
					((*(infile+k)<= ' ') || (*(infile+k)>'z')))
					k++;
			} while(k < size);

			for (k = 0;k < index;k++)
				myargv[indexinfile++] = moreargs[k];
			myargc = indexinfile;

			// DISPLAY ARGS
//			printf("%d command-line args:\n",myargc);
//			for (k=1;k<myargc;k++)
//				printf("%s\n",myargv[k]);

			break;
		}
}



/*
=================
=
= main
=
=================
*/

void main(void)
{
	int				p;

//
// set network characteristics
//
	doomcom.ticdup = 1;
	doomcom.extratics = 0;
	doomcom.numnodes = 2;
	doomcom.numplayers = 2;
	doomcom.drone = 0;

	printf("\n"
		   "---------------------------------\n"
		   #ifdef DOOM2
		   STR_DOOMSERIAL"\n"
		   #else
		   "DOOM SERIAL DEVICE DRIVER v1.4\n"
		   #endif
		   "---------------------------------\n");
	myargc = _argc;
	myargv = _argv;
	FindResponseFile();

//
// allow override of automatic player ordering to allow a slower computer
// to be set as player 1 allways
//

//
// establish communications
//

	baudbits = 0x08;		// default to 9600 if not specified on cmd line
							// or in modem.cfg

	if (CheckParm ("-dial") || CheckParm ("-answer") )
		ReadModemCfg ();		// may set baudbits

//
// allow command-line override of modem.cfg baud rate
//
	if (CheckParm ("-9600")) baudbits = 0x0c;
	else if (CheckParm ("-14400")) baudbits = 0x08;
	else if (CheckParm ("-19200")) baudbits = 0x06;
	else if (CheckParm ("-38400")) baudbits = 0x03;
	else if (CheckParm ("-57600")) baudbits = 0x02;
	else if (CheckParm ("-115200")) baudbits = 0x01;

	InitPort ();

	if (CheckParm ("-dial"))
		Dial ();
	else if (CheckParm ("-answer"))
		Answer ();

	Connect ();

//
// launch DOOM
//
	LaunchDOOM ();

	Error (NULL);
}

