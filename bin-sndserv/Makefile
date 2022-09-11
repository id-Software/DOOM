##########################################################
#
# $Id:$
#
# $Log:$
#
#

CC=gcc
CFLAGS=-m32 -std=gnu99 -O -DNORMALUNIX -DLINUX
LDFLAGS=-m32
LIBS=-lm

O=linux

all:	 $(O)/sndserver $(O)/doom1.wad

clean:
	rm -f *.o *~ *.flc
	rm -f linux/*

# Target
$(O)/sndserver: \
	$(O)/soundsrv.o \
	$(O)/sounds.o \
	$(O)/wadread.o \
	$(O)/linux.o
	$(CC) $(CFLAGS) $(LDFLAGS) \
	$(O)/soundsrv.o \
	$(O)/sounds.o \
	$(O)/wadread.o \
	$(O)/linux.o -o $(O)/sndserver $(LIBS)
	echo make complete.

# Rule
$(O)/%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

$(O)/doom1.wad: ../iwad/doom1.wad
	cp -rf $< $@


