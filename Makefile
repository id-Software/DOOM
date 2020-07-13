################################################################
#
# $Id:$
#
# $Log:$
#

OSFLAG:=
ifeq ($(OS),Windows_NT)
  OSFLAG += -D WIN32
  ifeq ($(PROCESSOR_ARCHITECTURE),AMD64)
    OSFLAG += -D AMD64
  endif
  ifeq ($(PROCESSOR_ARCHITECTURE),x86)
    OSFLAG += -D IA32
  endif
else
  UNAME_S := $(shell uname -s)
  ifeq ($(UNAME_S),Linux)
    OSFLAG += -D LINUX
  endif
  ifeq ($(UNAME_S),Darwin)
    OSFLAG += -D OSX
  endif
    UNAME_P := $(shell uname -p)
  ifeq ($(UNAME_P),x86_64)
    OSFLAG += -D AMD64
  endif
    ifneq ($(filter %86,$(UNAME_P)),)
  OSFLAG += -D IA32
    endif
  ifneq ($(filter arm%,$(UNAME_P)),)
    OSFLAG += -D ARM
  endif
endif

CC=gcc # gcc or g++


RAYLIB_PATH:=
RAYLIB_INCLUDE:=
RAYLIB_LIB:=
ifeq ($(OS),Windows_NT)
  RAYLIB_PATH=C:/raylib
  RAYLIB_INCLUDE=$(RAYLIB_PATH)/raylib/src
  RAYLIB_LIB=$(RAYLIB_PATH)/MinGW/i686-w64-mingw32/lib
else
  ifeq ($(UNAME_S),Darwin)
    RAYLIB_PATH=/usr/local/var/homebrew/linked/raylib
    RAYLIB_INCLUDE=$(RAYLIB_PATH)/include
    RAYLIB_LIB=$(RAYLIB_PATH)/lib
  else
    $(error unknown raylib path)
  endif
endif

CFLAGS=-g $(OSFLAG) # -DNORMALUNIX
LDFLAGS=-I$(RAYLIB_INCLUDE) -L$(RAYLIB_LIB)
LIBS=-lraylib
ifeq ($(OS),Windows_NT)
  LIBS += -lopengl32 -lgdi32 -lwinmm
endif

# subdirectory for objects
SRC=src
O=build

# not too sophisticated dependency
OBJS=				\
		$(O)/doomdef.o		\
		$(O)/doomstat.o		\
		$(O)/dstrings.o		\
		$(O)/i_system.o		\
		$(O)/i_sound.o		\
		$(O)/i_video.o		\
		$(O)/i_net.o			\
		$(O)/tables.o			\
		$(O)/f_finale.o		\
		$(O)/f_wipe.o 		\
		$(O)/d_main.o			\
		$(O)/d_net.o			\
		$(O)/d_items.o		\
		$(O)/g_game.o			\
		$(O)/m_menu.o			\
		$(O)/m_misc.o			\
		$(O)/m_argv.o  		\
		$(O)/m_bbox.o			\
		$(O)/m_fixed.o		\
		$(O)/m_swap.o			\
		$(O)/m_cheat.o		\
		$(O)/m_random.o		\
		$(O)/am_map.o			\
		$(O)/p_ceilng.o		\
		$(O)/p_doors.o		\
		$(O)/p_enemy.o		\
		$(O)/p_floor.o		\
		$(O)/p_inter.o		\
		$(O)/p_lights.o		\
		$(O)/p_map.o			\
		$(O)/p_maputl.o		\
		$(O)/p_plats.o		\
		$(O)/p_pspr.o			\
		$(O)/p_setup.o		\
		$(O)/p_sight.o		\
		$(O)/p_spec.o			\
		$(O)/p_switch.o		\
		$(O)/p_mobj.o			\
		$(O)/p_telept.o		\
		$(O)/p_tick.o			\
		$(O)/p_saveg.o		\
		$(O)/p_user.o			\
		$(O)/r_bsp.o			\
		$(O)/r_data.o			\
		$(O)/r_draw.o			\
		$(O)/r_main.o			\
		$(O)/r_plane.o		\
		$(O)/r_segs.o			\
		$(O)/r_sky.o			\
		$(O)/r_things.o		\
		$(O)/w_wad.o			\
		$(O)/wi_stuff.o		\
		$(O)/v_video.o		\
		$(O)/st_lib.o			\
		$(O)/st_stuff.o		\
		$(O)/hu_stuff.o		\
		$(O)/hu_lib.o			\
		$(O)/s_sound.o		\
		$(O)/z_zone.o			\
		$(O)/info.o				\
		$(O)/sounds.o

all:	 $(O)/doom

clean:
  ifeq ($(OS),Windows_NT)
    del *.o *.exe /s
  else
    $(rm -f *.o)
    $(rm -f $(O)/*.o)
  endif

$(O)/doom:	$(OBJS) $(O)/i_main.o
	$(CC) $(CFLAGS) $(LDFLAGS) $(OBJS) $(O)/i_main.o \
	-o $(O)/doom $(LIBS)

$(O)/%.o:	$(SRC)/%.c
	$(CC) $(CFLAGS) -c $< -o $@

#############################################################
#
#############################################################
