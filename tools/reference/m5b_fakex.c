/*
 * M5b input driver: the FAKE X server behind the genuine i_video.c.
 *
 * Models only what I_StartTic / I_GetEvent observe (i_video.c:194-340):
 *   - a FIFO of scripted XEvents, drained by XPending/XNextEvent;
 *   - XKeycodeToKeysym (display, keycode, 0): the fake keycode IS the keysym the script named
 *     (index 0: the unshifted keysym, as i_video.c:102 asks for);
 *   - the pointer: absolute window position and the held-button mask; ButtonPress and
 *     ButtonRelease carry the button state BEFORE the event in .state, MotionNotify the
 *     current state (X protocol semantics, which i_video.c:214-244 relies on);
 *   - XWarpPointer (i_video.c:323-328): counted; with fake_warp_model on, a warp that moves the
 *     pointer queues a MotionNotify at the destination ahead of the next tic's events (what an X
 *     server delivers), off by default.
 * Every other Xlib entry point is reachable only from I_InitGraphics / I_FinishUpdate /
 * I_ShutdownGraphics, which the driver never calls: they abort.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/extensions/XShm.h>

struct FakeDisplay { int dummy; };
struct FakeDisplay fake_display_object;

#define FQ 4096
static XEvent fq[FQ];
static int fq_head, fq_tail;
int fake_ptr_x = 160, fake_ptr_y = 100;  /* pointer position in the window */
unsigned int fake_buttons;               /* Button1Mask.. currently held */
int fake_warp_model;                      /* 1: XWarpPointer queues the resulting MotionNotify */
int fake_warps;                           /* XWarpPointer calls */
int fake_drained;                         /* XNextEvent calls */
static XEvent warp_pending; static int warp_has;

static void push (XEvent *e)
{
    if (((fq_head + 1) % FQ) == fq_tail) { fprintf (stderr, "fake X queue full\n"); exit (3); }
    fq[fq_head] = *e; fq_head = (fq_head + 1) % FQ;
}

/* called by the driver at the start of a tic, before its scripted events are queued */
void fake_begin_tic (void)
{
    if (warp_has) { push (&warp_pending); warp_has = 0; }
}

void fake_key (int press, unsigned long keysym)
{
    XEvent e; memset (&e, 0, sizeof e);
    e.type = press ? KeyPress : KeyRelease;
    e.xkey.keycode = (unsigned int) keysym;
    e.xkey.state = fake_buttons;
    push (&e);
}

static unsigned int bmask (int b) { return b == 1 ? Button1Mask : b == 2 ? Button2Mask : b == 3 ? Button3Mask : 0; }

void fake_button (int press, int button)
{
    XEvent e; memset (&e, 0, sizeof e);
    e.type = press ? ButtonPress : ButtonRelease;
    e.xbutton.button = button;
    e.xbutton.state = fake_buttons;     /* state before the event */
    e.xbutton.x = fake_ptr_x; e.xbutton.y = fake_ptr_y;
    push (&e);
    if (press) fake_buttons |= bmask (button); else fake_buttons &= ~bmask (button);
}

void fake_motion (int x, int y)
{
    XEvent e; memset (&e, 0, sizeof e);
    e.type = MotionNotify;
    e.xmotion.x = x; e.xmotion.y = y;
    e.xmotion.state = fake_buttons;
    fake_ptr_x = x; fake_ptr_y = y;
    push (&e);
}

void fake_other (int type)
{
    XEvent e; memset (&e, 0, sizeof e);
    e.type = type;
    push (&e);
}

KeySym XKeycodeToKeysym (Display *d, unsigned int keycode, int index)
{
    (void) d; if (index != 0) { fprintf (stderr, "XKeycodeToKeysym index %d\n", index); abort (); }
    return (KeySym) keycode;
}
int XPending (Display *d) { (void) d; return (fq_head - fq_tail + FQ) % FQ; }
int XNextEvent (Display *d, XEvent *e)
{
    (void) d;
    if (fq_head == fq_tail) { fprintf (stderr, "XNextEvent on an empty queue (would block)\n"); abort (); }
    *e = fq[fq_tail]; fq_tail = (fq_tail + 1) % FQ; fake_drained++;
    return 0;
}
int XWarpPointer (Display *d, Window s, Window w, int sx, int sy, unsigned int sw, unsigned int sh, int dx, int dy)
{
    (void) d; (void) s; (void) w; (void) sx; (void) sy; (void) sw; (void) sh;
    fake_warps++;
    if (fake_warp_model && (dx != fake_ptr_x || dy != fake_ptr_y))
    {
        memset (&warp_pending, 0, sizeof warp_pending);
        warp_pending.type = MotionNotify;
        warp_pending.xmotion.x = dx; warp_pending.xmotion.y = dy; warp_pending.xmotion.state = fake_buttons;
        warp_has = 1;
    }
    fake_ptr_x = dx; fake_ptr_y = dy;
    return 0;
}

#define NOPE(name) { fprintf (stderr, "fake Xlib: %s reached (not modelled)\n", name); abort (); }
Display *XOpenDisplay (const char *n) { (void) n; NOPE ("XOpenDisplay"); }
int XPutImage (Display *a, Drawable b, GC c, XImage *d, int e, int f, int g, int h, unsigned int i, unsigned int j) NOPE ("XPutImage")
int XSync (Display *a, Bool b) NOPE ("XSync")
int XStoreColors (Display *a, Colormap b, XColor *c, int d) NOPE ("XStoreColors")
Colormap XCreateColormap (Display *a, Window b, Visual *c, int d) NOPE ("XCreateColormap")
Window XCreateWindow (Display *a, Window b, int c, int d, unsigned int e, unsigned int f, unsigned int g, int h, unsigned int i, Visual *j, unsigned long k, XSetWindowAttributes *l) NOPE ("XCreateWindow")
int XDefineCursor (Display *a, Window b, Cursor c) NOPE ("XDefineCursor")
GC XCreateGC (Display *a, Drawable b, unsigned long c, XGCValues *d) NOPE ("XCreateGC")
int XMapWindow (Display *a, Window b) NOPE ("XMapWindow")
int XGrabPointer (Display *a, Window b, Bool c, unsigned int d, int e, int f, Window g, Cursor h, unsigned long i) NOPE ("XGrabPointer")
XImage *XCreateImage (Display *a, Visual *b, unsigned int c, int d, int e, char *f, unsigned int g, unsigned int h, int i, int j) NOPE ("XCreateImage")
Pixmap XCreatePixmap (Display *a, Drawable b, unsigned int c, unsigned int d, unsigned int e) NOPE ("XCreatePixmap")
int XFillRectangle (Display *a, Drawable b, GC c, int d, int e, unsigned int f, unsigned int g) NOPE ("XFillRectangle")
Cursor XCreatePixmapCursor (Display *a, Pixmap b, Pixmap c, XColor *d, XColor *e, unsigned int f, unsigned int g) NOPE ("XCreatePixmapCursor")
int XFreePixmap (Display *a, Pixmap b) NOPE ("XFreePixmap")
int XFreeGC (Display *a, GC b) NOPE ("XFreeGC")
Status XMatchVisualInfo (Display *a, int b, int c, int d, XVisualInfo *e) NOPE ("XMatchVisualInfo")
Bool XShmQueryExtension (Display *a) NOPE ("XShmQueryExtension")
Bool XShmAttach (Display *a, XShmSegmentInfo *b) NOPE ("XShmAttach")
Bool XShmDetach (Display *a, XShmSegmentInfo *b) NOPE ("XShmDetach")
XImage *XShmCreateImage (Display *a, Visual *b, unsigned int c, int d, char *e, XShmSegmentInfo *f, unsigned int g, unsigned int h) NOPE ("XShmCreateImage")
Bool XShmPutImage (Display *a, Drawable b, GC c, XImage *d, int e, int f, int g, int h, unsigned int i, unsigned int j, Bool k) NOPE ("XShmPutImage")
int XShmGetEventBase (Display *a) NOPE ("XShmGetEventBase")
