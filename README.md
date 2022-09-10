# DOOM (my fork)

This is a bare bones 32-bit linux fork of the original DOOM source code. The point of this fork is to apply as minimal changes as possible to run the game on modern 32-bit linux (Debian 11). This git repository includes shareware DOOM1.WAD

![example screenshot](.docs/screenshot.png "Running inside 8-bit Xephyr window")  
*As you can see, even the colors are wrong at the moment*

## TODO

- Sound does not work
- Colors are incorrect
- Mouse does not work
- Music does not work

## Compiling the game

```bash
make
```

## Running the game

The original linux DOOM uses an 8-bit 320x200 screen, modern X11 does not support this out-of-box but a simulated window can be created with a different DISPLAY ID.

```bash
# start in the background
Xephyr :1 -ac screen 320x200x8
# run the game in DISPLAY 1
DISPLAY=:1 ./linuxxdoom
```
