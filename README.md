# DOOM (Fixed)

I was excited to discover that the original DOOM source code was open on GitHub. Unfortunately, when I tried to clone and build it for myself, it did not work OOTB. This fork/repo contains a collection of fixes that I had to apply to get it working on my computer. The goal is that others will stumble across this repo and be able to build the source code for themselves without error.

For a full list of changes made to the repo after forking it, see the [changelog](./CHANGELOG.md).

## Requirements

_Note_: Provided commands were tested on Linux Mint Victoria 21.2.

- Linux
- Git (`sudo apt install git`)
- libc6-dev (`sudo apt-get install libc6-dev`)
- libx11-dev (`sudo apt-get install libx11-dev`)
- libxext-dev (`sudo apt-get install libxext-dev`)
- xserver-xephyr (`sudo apt-get install xserver-xephyr`)

## Build and Run

0. Open up a terminal
1. `git clone https://github.com/lunkums/DOOM_fixed.git` (if not already done)
2. `cd DOOM_fixed/linuxdoom-1.10`
3. `make`
4. `cd linux/`
5. `curl -O https://distro.ibiblio.org/slitaz/sources/packages/d/doom1.wad` (download doom1.wad)
6. In a separate terminal, run `Xephyr :2 -ac -screen 640x400x8`
7. Back in the first terminal, run `DISPLAY=:2` then `./linuxxdoom -2` (-2 means the game is scaled by a factor of 2)

## Additional Info

https://hexadecimal.uoregon.edu/~stevev/Linux-DOOM-FAQ.html
https://www.youtube.com/watch?v=9JgQfQHHhTw
