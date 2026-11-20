/* M5c mixer driver (tools/reference/m5c_mixer_build.py): a declared FAKE <linux/soundcard.h>, so
 * that the genuine linuxdoom-1.10 i_sound.c compiles with no audio device and no kernel sound
 * header. Not the kernel header: only the seven names i_sound.c uses (i_sound.c:775-789), with the
 * values of the Linux UAPI OSS header (_IOWR('P', n, int) and friends; the Phase 1 prototype printed
 * the host header's values and they are these). The fake ioctl of m5c_mixer_fakesnd.c records the
 * requests and never reaches a device. */
#ifndef M5C_FAKE_LINUX_SOUNDCARD_H
#define M5C_FAKE_LINUX_SOUNDCARD_H
#define SNDCTL_DSP_RESET       0x00005000
#define SNDCTL_DSP_SPEED       0xc0045002
#define SNDCTL_DSP_STEREO      0xc0045003
#define SNDCTL_DSP_SETFMT      0xc0045005
#define SNDCTL_DSP_SETFRAGMENT 0xc004500a
#define SNDCTL_DSP_GETFMTS     0x8004500b
#define AFMT_S16_LE            0x00000010
#endif
