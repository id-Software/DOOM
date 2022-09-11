// ipxnet.h


typedef struct
{
     char private[512];
} doomdata_t;


#include "DoomNet.h"

//===========================================================================

#define NUMPACKETS      10              // max outstanding packets before loss

// setupdata_t is used as doomdata_t during setup
typedef struct
{
     short     gameid;                       // so multiple games can setup at once
     short     drone;
     short     nodesfound;
     short     nodeswanted;
} setupdata_t;



typedef unsigned char BYTE;
typedef unsigned short WORD;
typedef unsigned long LONG;

typedef struct IPXPacketStructure
{
     WORD    PacketCheckSum;         /* high-low */
     WORD    PacketLength;           /* high-low */
     BYTE    PacketTransportControl;
     BYTE    PacketType;

     BYTE    dNetwork[4];            /* high-low */
     BYTE    dNode[6];               /* high-low */
     BYTE    dSocket[2];             /* high-low */

     BYTE    sNetwork[4];            /* high-low */
     BYTE    sNode[6];               /* high-low */
     BYTE    sSocket[2];             /* high-low */
} IPXPacket;


typedef struct
{
     BYTE    network[4];             /* high-low */
     BYTE    node[6];                /* high-low */
} localadr_t;

typedef struct
{
     BYTE    node[6];                /* high-low */
} nodeadr_t;

typedef struct ECBStructure
{
     WORD    Link[2];                /* offset-segment */
     WORD    ESRAddress[2];          /* offset-segment */
     BYTE    InUseFlag;
     BYTE    CompletionCode;
     WORD    ECBSocket;              /* high-low */
     BYTE    IPXWorkspace[4];        /* N/A */
     BYTE    DriverWorkspace[12];    /* N/A */
     BYTE    ImmediateAddress[6];    /* high-low */
     WORD    FragmentCount;          /* low-high */

     WORD    fAddress[2];            /* offset-segment */
     WORD    fSize;                  /* low-high */

     WORD    f2Address[2];            /* offset-segment */
     WORD    f2Size;                  /* low-high */
} ECB;


// time is used by the communication driver to sequence packets returned
// to DOOM when more than one is waiting

typedef struct
{
     ECB             ecb;
     IPXPacket       ipx;

     long           time;
     doomdata_t          data;
} packet_t;


extern    doomcom_t doomcom;
extern    int            gameid;

extern    nodeadr_t nodeadr[MAXNETNODES+1];
extern    int            localnodenum;

extern    long           localtime;          // for time stamp in packets
extern    long      remotetime;         // timestamp of last packet gotten

extern    nodeadr_t remoteadr;

extern	  int	myargc;

extern	  char  **myargv;

void Error (char *error, ...);


void InitNetwork (void);
void ShutdownNetwork (void);
void SendPacket (int destination);
int GetPacket (void);
int CheckParm (char *check);

void PrintAddress (nodeadr_t *adr, char *str);

