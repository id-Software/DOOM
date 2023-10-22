# sndserv (Sound Server)

Linux DOOM plays sounds from a separate process called sndserv (sound server).

## Requirements

_Note_: Provided commands were tested on Linux Mint Victoria 21.2.

- Linux
- OSS Proxy Daemon (`sudo apt-get install osspd`)

## Build and Run

```bash
$ cd DOOM_fixed/sndserv # navigate to the `sndserv` directory
$ make # 64-bit build
$ make x86 # 32-bit build
$ cp ./linux/sndserver ../linuxdoom-1.10/linux/ # copy sndserver to the linuxdoom output directory
```

## Additional Info

- [How exactly did the linuxdoom sndserver used to work?](https://www.doomworld.com/forum/post/2544842) ([archive](https://web.archive.org/web/20231004233042/https://www.doomworld.com/forum/topic/131304-how-exactly-did-the-linuxdoom-sndserver-used-to-work/?tab=comments#comment-2544842))
- [Where is /dev/dsp or /dev/audio?](https://askubuntu.com/questions/220370/where-is-dev-dsp-or-dev-audio?rq=1) ([archive](https://web.archive.org/web/20231004233215/https://askubuntu.com/questions/220370/where-is-dev-dsp-or-dev-audio?rq=1))
- [Package: osspd (1.3.2-13.1)](https://packages.debian.org/sid/osspd) ([archive](https://web.archive.org/web/20231004234639/https://packages.debian.org/sid/osspd))
