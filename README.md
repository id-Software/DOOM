# DOOM (Fixed)

I was excited to discover that the original DOOM source code was open on GitHub. Unfortunately, when I tried to clone and build it for myself, it did not work OOTB. This fork/repo contains a collection of fixes that I had to apply to get it working on my computer. The goal is that others will stumble across this repo and be able to build the source code for themselves without error.

## Requirements

_Note_: Provided commands were tested on Linux Mint Victoria 21.2.

- Linux
- Git (`sudo apt install git`)
- xserver-xephyr (`sudo apt-get install xserver-xephyr`)

### 64-bit

- libc6-dev (`sudo apt-get install libc6-dev`)
- libx11-dev (`sudo apt-get install libx11-dev`)
- libxext-dev (`sudo apt-get install libxext-dev`)
- libnsl-dev (`sudo apt-get install libnsl-dev`)

### 32-bit

- libc6-dev:i386 (`sudo apt-get install libc6-dev:i386`)
- libx11-dev:i386 (`sudo apt-get install libx11-dev:i386`)
- libxext-dev:i386 (`sudo apt-get install libxext-dev:i386`)
- libnsl-dev (`sudo apt-get install libnsl-dev:i386`)

## Build and Run

```bash
$ git clone https://github.com/lunkums/DOOM_fixed.git # clone the repo
$ cd DOOM_fixed/linuxdoom-1.10 # navigate to the linuxdoom-1.10 folder
$ make # 64-bit build
$ make x86 # 32-bit build
$ cd linux/ # navigate to the output directory
$ curl -O https://distro.ibiblio.org/slitaz/sources/packages/d/doom1.wad # download the shareware wad
...
# in a separate terminal, run:
$ Xephyr :2 -ac -screen 320x200x8 # start an X server with Xephyr
...
# back in the first terminal, run:
$ DISPLAY=:2 # set the `DISPLAY` environment variable to be `:2`
$ ./linuxxdoom # LGTM
```

### Sound

See the [sndserv README](./sndserv/README.md) for instructions on setting up the game's sound.

### Music

Unfortunately, Linux DOOM does not support music due to a licensing issue with the [DMX](https://doomwiki.org/wiki/DMX) sound library used in the original release. Reimplementing it is out of the scope of this fork.

## Additional Info

- [Linux DOOM FAQ](https://hexadecimal.uoregon.edu/~stevev/Linux-DOOM-FAQ.html)
- [How To Compile Vanilla Doom (Linux Doom)](https://www.youtube.com/watch?v=9JgQfQHHhTw)
