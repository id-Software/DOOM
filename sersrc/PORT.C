// port.c

#include "doomnet.h"
#include "sersetup.h"
//#include "serstr.h"
#include "ser_frch.h"		// FRENCH VERSION


void jump_start( void );

void interrupt isr_8250 (void);
void interrupt isr_16550 (void);

union	REGS	regs;
struct	SREGS	sregs;

que_t		inque, outque;


int			uart;			// io address
enum {UART_8250, UART_16550} uart_type;
int			irq;

int			modem_status = -1;
int			line_status = -1;

void interrupt (*oldirqvect) (void);
int			irqintnum;

int	   		comport;

int			baudbits;


/*
==============
=
= GetUart
=
==============
*/

void GetUart (void)
{
	char   far *system_data;
	static int ISA_uarts[] = {0x3f8,0x2f8,0x3e8,0x2e8};
	static int ISA_IRQs[] = {4,3,4,3};
	static int MCA_uarts[] = {0x03f8,0x02f8,0x3220,0x3228};
	static int MCA_IRQs[] = {4,3,3,3};
	int		p;

	if (CheckParm ("-com2"))
		comport = 2;
	else if (CheckParm ("-com3"))
		comport = 3;
	else if (CheckParm ("-com4"))
		comport = 4;
	else
		comport = 1;

	regs.h.ah = 0xc0;
	int86x( 0x15, &regs, &regs, &sregs );
	if ( regs.x.cflag )
	{
		irq = ISA_IRQs[ comport-1 ];
		uart = ISA_uarts[ comport-1 ];
		return;
	}
	system_data = ( char far *) ( ( (long) sregs.es << 16 ) + regs.x.bx );
	if ( system_data[ 5 ] & 0x02 )
	{
		irq = MCA_IRQs[ comport-1 ];
		uart = MCA_uarts[ comport-1 ];
	}
	else
	{
		irq = ISA_IRQs[ comport-1 ];
		uart = ISA_uarts[ comport-1 ];
	}

	p = CheckParm ("-port");
	if (p)
		sscanf (_argv[p+1],"0x%x",&uart);
	p = CheckParm ("-irq");
	if (p)
		sscanf (_argv[p+1],"%i",&irq);


	printf (STR_PORTLOOK" 0x%x, irq %i\n",uart,irq);
}




/*
===============
=
= InitPort
=
===============
*/

void InitPort (void)
{
	int mcr;
	int	temp;
	int	u;

//
// find the irq and io address of the port
//
	GetUart ();

//
// disable all uart interrupts
//
	OUTPUT( uart + INTERRUPT_ENABLE_REGISTER, 0 );

//
// init com port settings
//

	printf (STR_PORTSET"\n",115200l/baudbits);

// set baud rate
	OUTPUT(uart + LINE_CONTROL_REGISTER, 0x83);
	OUTPUT(uart, baudbits);
	OUTPUT(uart + 1, 0);

// set line control register (N81)
	OUTPUT(uart + LINE_CONTROL_REGISTER, 0x03);

// set modem control register (OUT2+RTS+DTR)
	OUTPUT( uart + MODEM_CONTROL_REGISTER,8+2+1);

//
// check for a 16550
//
	if (CheckParm("-8250"))
	// allow a forced 8250
	{
		uart_type = UART_8250;
		printf (STR_UART8250"\n\n");
	}
	else
	{	
		OUTPUT ( uart + FIFO_CONTROL_REGISTER,
			FCR_FIFO_ENABLE + FCR_TRIGGER_04 );
		temp = INPUT( uart + INTERRUPT_ID_REGISTER );
		if ( ( temp & 0xf8 ) == 0xc0 )
		{
			uart_type = UART_16550;
		}
		else
		{
			uart_type = UART_8250;
			OUTPUT( uart + FIFO_CONTROL_REGISTER, 0 );
		}
	}

//
// clear out any pending uart events
//
	printf (STR_CLEARPEND);
	for (u=0 ; u<16 ; u++)		// clear an entire 16550 silo
		INPUT( uart + RECEIVE_BUFFER_REGISTER );

	do
	{
		switch( u = INPUT( uart + INTERRUPT_ID_REGISTER ) & 7 )
		{
		case IIR_MODEM_STATUS_INTERRUPT :
			modem_status = INPUT( uart + MODEM_STATUS_REGISTER );
			break;

		case IIR_LINE_STATUS_INTERRUPT :
			line_status = INPUT( uart + LINE_STATUS_REGISTER );
			break;

		case IIR_TX_HOLDING_REGISTER_INTERRUPT :
			break;

		case IIR_RX_DATA_READY_INTERRUPT :
			INPUT( uart + RECEIVE_BUFFER_REGISTER );
			break;
		}
	} while (! (u&1) );


//
// hook the irq vector
//
	irqintnum = irq + 8;

	oldirqvect = getvect (irqintnum);
	if (uart_type == UART_16550)
	{
		setvect (irqintnum,isr_16550);
		printf (STR_UART16550"\n\n");
	}
	else
	{
		setvect (irqintnum,isr_8250);
		printf (STR_UART8250"\n\n");

	}


	OUTPUT( 0x20 + 1, INPUT( 0x20 + 1 ) & ~(1<<irq) );

	CLI();

// enable RX and TX interrupts at the uart

	OUTPUT( uart + INTERRUPT_ENABLE_REGISTER,IER_RX_DATA_READY + IER_TX_HOLDING_REGISTER_EMPTY);

// enable interrupts through the interrupt controller

	OUTPUT( 0x20, 0xc2 );

	STI();
}


/*
=============
=
= ShutdownPort
=
=============
*/

void ShutdownPort ( void )
{
	int		u;
	
	OUTPUT( uart + INTERRUPT_ENABLE_REGISTER, 0 );
	OUTPUT( uart + MODEM_CONTROL_REGISTER, 0 );

	for (u=0 ; u<16 ; u++)		// clear an entire 16550 silo
		INPUT( uart + RECEIVE_BUFFER_REGISTER );

	OUTPUT( 0x20 + 1, INPUT( 0x20 + 1 ) | (1<<irq) );

	setvect (irqintnum,oldirqvect);

//
// init com port settings to defaults
//
	regs.x.ax = 0xf3;		//f3= 9600 n 8 1
	regs.x.dx = comport - 1;
	int86 (0x14, &regs, &regs);
}


int read_byte( void )
{
	int	c;

	if (inque.tail >= inque.head)
		return -1;
	c = inque.data[inque.tail&(QUESIZE-1)];
	inque.tail++;
	return c;
}


void write_byte( int c )
{
	outque.data[outque.head&(QUESIZE-1)] = c;
	outque.head++;
}



//==========================================================================


/*
==============
=
= isr_8250
=
==============
*/

void interrupt isr_8250(void)
{
	int c;

	while (1)
	{
		switch( INPUT( uart + INTERRUPT_ID_REGISTER ) & 7 )
		{
// not enabled
		case IIR_MODEM_STATUS_INTERRUPT :
			modem_status = INPUT( uart + MODEM_STATUS_REGISTER );
			break;

// not enabled
		case IIR_LINE_STATUS_INTERRUPT :
			line_status = INPUT( uart + LINE_STATUS_REGISTER );
			break;

//
// transmit
//
		case IIR_TX_HOLDING_REGISTER_INTERRUPT :
//I_ColorBlack (63,0,0);
			if (outque.tail < outque.head)
			{
				c = outque.data[outque.tail&(QUESIZE-1)];
				outque.tail++;
				OUTPUT( uart + TRANSMIT_HOLDING_REGISTER, c );
			}
			break;

//
// receive
//
		case IIR_RX_DATA_READY_INTERRUPT :
//I_ColorBlack (0,63,0);
			c = INPUT( uart + RECEIVE_BUFFER_REGISTER );
			inque.data[inque.head&(QUESIZE-1)] = c;
			inque.head++;
			break;

//
// done
//
		default :
//I_ColorBlack (0,0,0);
			OUTPUT( 0x20, 0x20 );
			return;
		}
	}
}


/*
==============
=
= isr_16550
=
==============
*/

void interrupt isr_16550(void)
{
	int c;
	int	count;

	while (1)
	{
		switch( INPUT( uart + INTERRUPT_ID_REGISTER ) & 7 )
		{
// not enabled
		case IIR_MODEM_STATUS_INTERRUPT :
			modem_status = INPUT( uart + MODEM_STATUS_REGISTER );
			break;

// not enabled
		case IIR_LINE_STATUS_INTERRUPT :
			line_status = INPUT( uart + LINE_STATUS_REGISTER );
			break;

//
// transmit
//
		case IIR_TX_HOLDING_REGISTER_INTERRUPT :
//I_ColorBlack (63,0,0);
			count = 16;
			while (outque.tail < outque.head && count--)
			{
				c = outque.data[outque.tail&(QUESIZE-1)];
				outque.tail++;
				OUTPUT( uart + TRANSMIT_HOLDING_REGISTER, c );
			}
			break;

//
// receive
//
		case IIR_RX_DATA_READY_INTERRUPT :
//I_ColorBlack (0,63,0);
			do
			{
				c = INPUT( uart + RECEIVE_BUFFER_REGISTER );
				inque.data[inque.head&(QUESIZE-1)] = c;
				inque.head++;
			} while (INPUT( uart + LINE_STATUS_REGISTER ) & LSR_DATA_READY );

			break;

//
// done
//
		default :
//I_ColorBlack (0,0,0);
			OUTPUT( 0x20, 0x20 );
			return;
		}
	}
}


/*
===============
=
= jump_start
=
= Start up the transmition interrupts by sending the first char
===============
*/

void jump_start( void )
{
	int c;

	if (outque.tail < outque.head)
	{
		c = outque.data [outque.tail&(QUESIZE-1)];
		outque.tail++;
		OUTPUT( uart, c );
	}
}


