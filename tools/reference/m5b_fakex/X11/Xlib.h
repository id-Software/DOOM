/* M5b input driver (tools/reference/m5b_input_build.py): a declared FAKE Xlib for compiling the genuine linuxdoom-1.10 i_video.c
 * headlessly. Not the X11 headers: only the declarations i_video.c uses. Protocol constants
 * (event types, masks, button numbers) carry the X11 protocol values. The definitions live in
 * fakex.c; every call i_video.c can make outside I_StartTic/I_GetEvent aborts. */
#ifndef FAKE_XLIB_H
#define FAKE_XLIB_H
typedef unsigned long XID, Window, Colormap, Pixmap, Cursor, Drawable, KeySym, VisualID;
typedef unsigned char KeyCode;
typedef int Bool, Status;
typedef struct FakeDisplay Display;
typedef struct { VisualID visualid; } Visual;
typedef struct FakeGC *GC;
#define True 1
#define False 0
#define None 0L
#define CurrentTime 0L
/* event types (X.h) */
#define KeyPress 2
#define KeyRelease 3
#define ButtonPress 4
#define ButtonRelease 5
#define MotionNotify 6
#define Expose 12
#define ConfigureNotify 22
/* masks (X.h) */
#define KeyPressMask (1L<<0)
#define KeyReleaseMask (1L<<1)
#define ButtonPressMask (1L<<2)
#define ButtonReleaseMask (1L<<3)
#define PointerMotionMask (1L<<6)
#define ExposureMask (1L<<15)
#define Button1Mask (1<<8)
#define Button2Mask (1<<9)
#define Button3Mask (1<<10)
#define Button1 1
#define Button2 2
#define Button3 3
#define InputOutput 1
#define CWBorderPixel (1L<<3)
#define CWEventMask (1L<<11)
#define CWColormap (1L<<13)
#define AllocAll 1
#define PseudoColor 3
#define ZPixmap 2
#define DoRed (1<<0)
#define DoGreen (1<<1)
#define DoBlue (1<<2)
#define GrabModeAsync 1
#define GXclear 0x0
#define GCFunction (1L<<0)
#define GCGraphicsExposures (1L<<16)
typedef struct { int type; unsigned long serial; Bool send_event; Display *display; Window window, root, subwindow;
                 unsigned long time; int x, y, x_root, y_root; unsigned int state; unsigned int keycode; Bool same_screen; } XKeyEvent;
typedef struct { int type; unsigned long serial; Bool send_event; Display *display; Window window, root, subwindow;
                 unsigned long time; int x, y, x_root, y_root; unsigned int state; unsigned int button; Bool same_screen; } XButtonEvent;
typedef struct { int type; unsigned long serial; Bool send_event; Display *display; Window window, root, subwindow;
                 unsigned long time; int x, y, x_root, y_root; unsigned int state; char is_hint; Bool same_screen; } XMotionEvent;
typedef struct { int type; unsigned long serial; Bool send_event; Display *display; Window window; int x, y, width, height, count; } XExposeEvent;
typedef union { int type; XKeyEvent xkey; XButtonEvent xbutton; XMotionEvent xmotion; XExposeEvent xexpose; long pad[24]; } XEvent;
typedef struct { unsigned long pixel; unsigned short red, green, blue; char flags, pad; } XColor;
typedef struct { int width, height, xoffset, format; char *data; int byte_order, bitmap_unit, bitmap_bit_order, bitmap_pad, depth, bytes_per_line, bits_per_pixel; } XImage;
typedef struct { Pixmap background_pixmap; unsigned long background_pixel; Pixmap border_pixmap; unsigned long border_pixel;
                 int bit_gravity, win_gravity, backing_store; unsigned long backing_planes, backing_pixel; Bool save_under;
                 long event_mask, do_not_propagate_mask; Bool override_redirect; Colormap colormap; Cursor cursor; } XSetWindowAttributes;
typedef struct { int function; unsigned long plane_mask, foreground, background; int line_width; Bool graphics_exposures; } XGCValues;
#define DefaultScreen(d) 0
#define RootWindow(d, s) ((Window)1)
Display *XOpenDisplay (const char *);
KeySym XKeycodeToKeysym (Display *, unsigned int, int);
int XNextEvent (Display *, XEvent *);
int XPending (Display *);
int XWarpPointer (Display *, Window, Window, int, int, unsigned int, unsigned int, int, int);
int XPutImage (Display *, Drawable, GC, XImage *, int, int, int, int, unsigned int, unsigned int);
int XSync (Display *, Bool);
int XStoreColors (Display *, Colormap, XColor *, int);
Colormap XCreateColormap (Display *, Window, Visual *, int);
Window XCreateWindow (Display *, Window, int, int, unsigned int, unsigned int, unsigned int, int, unsigned int, Visual *, unsigned long, XSetWindowAttributes *);
int XDefineCursor (Display *, Window, Cursor);
GC XCreateGC (Display *, Drawable, unsigned long, XGCValues *);
int XMapWindow (Display *, Window);
int XGrabPointer (Display *, Window, Bool, unsigned int, int, int, Window, Cursor, unsigned long);
XImage *XCreateImage (Display *, Visual *, unsigned int, int, int, char *, unsigned int, unsigned int, int, int);
Pixmap XCreatePixmap (Display *, Drawable, unsigned int, unsigned int, unsigned int);
int XFillRectangle (Display *, Drawable, GC, int, int, unsigned int, unsigned int);
Cursor XCreatePixmapCursor (Display *, Pixmap, Pixmap, XColor *, XColor *, unsigned int, unsigned int);
int XFreePixmap (Display *, Pixmap);
int XFreeGC (Display *, GC);
#endif
