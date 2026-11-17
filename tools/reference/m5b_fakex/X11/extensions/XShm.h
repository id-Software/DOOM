#ifndef FAKE_XSHM_H
#define FAKE_XSHM_H
#include <X11/Xlib.h>
typedef unsigned long ShmSeg;
typedef struct { ShmSeg shmseg; int shmid; char *shmaddr; Bool readOnly; } XShmSegmentInfo;
#define ShmCompletion 0
Bool XShmQueryExtension (Display *);
Bool XShmAttach (Display *, XShmSegmentInfo *);
Bool XShmDetach (Display *, XShmSegmentInfo *);
XImage *XShmCreateImage (Display *, Visual *, unsigned int, int, char *, XShmSegmentInfo *, unsigned int, unsigned int);
Bool XShmPutImage (Display *, Drawable, GC, XImage *, int, int, int, int, unsigned int, unsigned int, Bool);
#endif
