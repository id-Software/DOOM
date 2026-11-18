/*
 * M5c mixer driver: the declared FAKE OSS device behind the genuine i_sound.c
 * (contract DOOM-M5C-MIXER-v1, field fake_device).
 *
 * Linked with -Wl,--wrap=open,close,ioctl,write,sigaction,setitimer, so every call of these from
 * the genuine objects (i_sound.c, w_wad.c) reaches a __wrap_ function here:
 *   - open ("/dev/dsp", O_WRONLY) (i_sound.c:770) returns FAKE_DSP_FD and touches no device; any
 *     other "/dev/" path aborts; every other path (the WAD files of W_AddFile, w_wad.c:163) goes
 *     to the real open, so the lumps are read by the genuine w_wad.c;
 *   - ioctl is accepted only on FAKE_DSP_FD (myioctl, i_sound.c:159-175, called at i_sound.c:776,
 *     777, 781, 784, 786, 789): the request and its argument are recorded; SPEED leaves 11025 in
 *     place, GETFMTS answers AFMT_S16_LE so the genuine code takes its SETFMT branch
 *     (i_sound.c:788-789); any other request aborts; it never fails, so myioctl's error path
 *     (i_sound.c:168-174) is not reached;
 *   - write is accepted only on FAKE_DSP_FD (I_SubmitSound, i_sound.c:669): the bytes go to the
 *     driver's fake_capture; any other descriptor aborts;
 *   - close on FAKE_DSP_FD (I_ShutdownSound, i_sound.c:725) is counted; other descriptors (the
 *     WAD files, w_wad.c:225) go to the real close;
 *   - sigaction and setitimer are referenced only by I_SoundSetTimer and I_SoundDelTimer
 *     (i_sound.c:941-985), compiled but called only with SNDINTR (i_sound.c:720-722, 762-765),
 *     which stays undefined (doomdef.h:85): they abort.
 * Nothing here opens, configures or writes an audio device, and no sound is ever played.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <fcntl.h>
#include <sys/types.h>
#include "m5c_fakesnd/linux/soundcard.h"

#define FAKE_DSP_FD 1000
int fake_dsp_opens, fake_dsp_closes, fake_dsp_writes, fake_ioctls;
int fake_speed = -1, fake_stereo = -1, fake_fmt = -1, fake_fragment = -1;

void fake_capture (const void *buf, size_t n);   /* m5c_mixer_driver.c */

int __real_open (const char *path, int flags, ...);
int __real_close (int fd);

int __wrap_open (const char *path, int flags, ...)
{
    int mode = 0;
    if (flags & O_CREAT) { va_list ap; va_start (ap, flags); mode = va_arg (ap, int); va_end (ap); }
    if (strcmp (path, "/dev/dsp") == 0) {
        if (flags != O_WRONLY) { fprintf (stderr, "fake dsp: unexpected open flags %d\n", flags); abort (); }
        fake_dsp_opens++;
        return FAKE_DSP_FD;
    }
    if (strncmp (path, "/dev/", 5) == 0) { fprintf (stderr, "fake dsp: refusing to open %s\n", path); abort (); }
    return __real_open (path, flags, mode);
}

int __wrap_close (int fd)
{
    if (fd == FAKE_DSP_FD) { fake_dsp_closes++; return 0; }
    return __real_close (fd);
}

int __wrap_ioctl (int fd, unsigned long req, ...)
{
    va_list ap; int *arg;
    unsigned int r = (unsigned int) req;   /* myioctl passes an int command (i_sound.c:162) */
    if (fd != FAKE_DSP_FD) { fprintf (stderr, "fake dsp: ioctl on fd %d\n", fd); abort (); }
    va_start (ap, req); arg = va_arg (ap, int *); va_end (ap);
    fake_ioctls++;
    switch (r) {
    case SNDCTL_DSP_SETFRAGMENT: fake_fragment = *arg; break;
    case SNDCTL_DSP_RESET: break;                                /* i_sound.c:777 passes a null argument */
    case SNDCTL_DSP_SPEED: fake_speed = *arg; break;             /* the fake device accepts 11025 as asked */
    case SNDCTL_DSP_STEREO: fake_stereo = *arg; break;
    case SNDCTL_DSP_GETFMTS: *arg = AFMT_S16_LE; break;
    case SNDCTL_DSP_SETFMT: fake_fmt = *arg; break;
    default: fprintf (stderr, "fake dsp: unknown ioctl 0x%x\n", r); abort ();
    }
    return 0;
}

ssize_t __wrap_write (int fd, const void *buf, size_t n)
{
    if (fd != FAKE_DSP_FD) { fprintf (stderr, "fake dsp: write to fd %d\n", fd); abort (); }
    fake_dsp_writes++;
    fake_capture (buf, n);
    return (ssize_t) n;
}

int __wrap_sigaction (int sig, const void *act, void *oact)
{ (void) sig; (void) act; (void) oact; fprintf (stderr, "fake: sigaction reached (SNDINTR only)\n"); abort (); }
int __wrap_setitimer (int which, const void *v, void *ov)
{ (void) which; (void) v; (void) ov; fprintf (stderr, "fake: setitimer reached (SNDINTR only)\n"); abort (); }
