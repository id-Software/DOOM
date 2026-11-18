#ifndef FAKE_XUTIL_H
#define FAKE_XUTIL_H
#include <X11/Xlib.h>
typedef struct { Visual *visual; VisualID visualid; int screen; int depth; int class; unsigned long red_mask, green_mask, blue_mask; int colormap_size, bits_per_rgb; } XVisualInfo;
Status XMatchVisualInfo (Display *, int, int, int, XVisualInfo *);
#endif
