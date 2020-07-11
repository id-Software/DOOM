# raylib-doom

raylib-doom is a port of DOOM to [raylib](https://www.raylib.com/) from the linuxdoom-1.10 open source release.

![doom1](https://raw.githubusercontent.com/sbuggay/raylib-doom/master/screenshots/doom1.png)

There are very little changes from the original source, mostly just the `I_` interface files.
Theres no reason to use this over something much more mature like Chocolate Doom. 

### Controls
|Key|Action|
|---|------|
|Control or Mouse1|Fire|
|W or Up|Forward|
|S or Down|Backward|
|A|Strafe Left|
|D|Strafe Right|
|Left|Turn Left|
|Right|Turn Right|
|E/Space|Use|
|Shift|Sprint|
|Alt|Strafe (for arrow keys)|

### Changes

- Add 64bit support.
- Resizable window that maintains internal aspect ratio.
- Default game screen size to maximum.
- Load menu demos from version 1.19 WADs.

### Roadmap

- Sound
- Music
- Configs
- Mouselook up/down
