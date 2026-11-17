//! `G_Responder`'s game switch (g_game.c:558-597), `G_BuildTiccmd` (g_game.c:237-437) and the
//! level-load clear of `G_DoLoadLevel` (g_game.c:490-496), over an owned state.

use super::demo::RecordedTic;
use super::{Controls, Event, InputError, NUMKEYS, keys};
use doom_sim::{BT_ATTACK, BT_CHANGE, BT_USE, TicCommand};

/// `forwardmove[2]` (g_game.c:175): walk and run forward thrust. `-turbo` (d_main.c:884-898)
/// is not supported, so the table is never scaled.
pub const FORWARDMOVE: [i32; 2] = [0x19, 0x32];
/// `sidemove[2]` (g_game.c:176): walk and run sideways thrust.
pub const SIDEMOVE: [i32; 2] = [0x18, 0x28];
/// `angleturn[3]` (g_game.c:177): normal, fast and slow keyboard turn.
pub const ANGLETURN: [i32; 3] = [640, 1280, 320];
/// `MAXPLMOVE` = `forwardmove[1]` (g_game.c:171): the clamp of both `forward` and `side`
/// (g_game.c:413-420).
pub const MAXPLMOVE: i32 = FORWARDMOVE[1];
/// `SLOWTURNTICS` (g_game.c:179).
pub const SLOWTURNTICS: i32 = 6;
/// `ticdup`: 1 unless `-dup` (i_net.c:254-265), which the app never uses.
const TICDUP: i32 = 1;
/// `BT_WEAPONSHIFT` (d_event.h:88).
const BT_WEAPONSHIFT: u8 = 3;
/// `NUMWEAPONS` (doomdef.h:182-192).
const NUMWEAPONS: u8 = 9;

/// One double-click machine of `G_BuildTiccmd`: `dclicktime`, `dclickstate` and `dclicks` for
/// the forward button (g_game.c:193-195, 354-376), or `dclicktime2`, `dclickstate2` and
/// `dclicks2` for the strafe button (g_game.c:196-198, 378-403).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DoubleClick {
    /// `dclicktime`: tics since the last registered transition. It is read only through
    /// `> 1` and `> 20`, so it saturates at `i32::MAX` instead of reaching the C's undefined
    /// overflow after 2^31 tics, with the same observable behaviour.
    pub time: i32,
    /// `dclickstate`: the raw button value last registered (the forward machine stores the
    /// `mousebuttons` value itself, so 4 for the right button; the strafe machine stores 0/1).
    pub state: i32,
    /// `dclicks`: presses counted towards a double click, 0 or 1 between tics.
    pub clicks: i32,
}

impl DoubleClick {
    /// One tic of the machine with the current button value; true when it completes a double
    /// click, which ORs in `BT_USE`. The text, for the forward machine (g_game.c:355-376):
    ///
    /// ```c
    /// if (mousebuttons[mousebforward] != dclickstate && dclicktime > 1 ) {
    ///     dclickstate = mousebuttons[mousebforward];
    ///     if (dclickstate) dclicks++;
    ///     if (dclicks == 2) { cmd->buttons |= BT_USE; dclicks = 0; }
    ///     else dclicktime = 0;
    /// } else {
    ///     dclicktime += ticdup;
    ///     if (dclicktime > 20) { dclicks = 0; dclickstate = 0; }
    /// }
    /// ```
    fn step(&mut self, current: i32) -> bool {
        if current != self.state && self.time > 1 {
            self.state = current;
            if self.state != 0 {
                self.clicks += 1;
            }
            if self.clicks == 2 {
                self.clicks = 0;
                true
            } else {
                self.time = 0;
                false
            }
        } else {
            self.time = self.time.saturating_add(TICDUP);
            if self.time > 20 {
                self.clicks = 0;
                self.state = 0;
            }
            false
        }
    }
}

/// A command built by [`InputState::build_ticcmd`], before the demo round trip: `angle_turn`
/// carries the full 16 bits. Only `G_BuildTiccmd` constructs one, which is what guarantees
/// the wire-24 domain (see the [module documentation](super)).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuiltCommand(TicCommand);

impl BuiltCommand {
    /// The command at full resolution (what an unrecorded session would execute).
    pub fn command(self) -> TicCommand {
        self.0
    }

    /// `G_WriteDemoTiccmd`'s write and re-read (g_game.c:1510-1522): the demo bytes and the
    /// command the game executes while recording.
    pub fn record(self) -> RecordedTic {
        RecordedTic::of(self.0)
    }
}

/// The variables of g_game.c that `G_Responder` writes and `G_BuildTiccmd` reads
/// (g_game.c:183-198), at process start all zero. The joystick variables (g_game.c:201-204)
/// are omitted because they are always zero (see [`super::Event`]); `sendpause` and
/// `sendsave` (g_game.c:426-436) are omitted because nothing here sets them (see
/// [`keys::WITHHELD`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputState {
    gamekeydown: [bool; NUMKEYS],
    /// `mousearray[4]` (g_game.c:186) holding the raw stored values: `mousebuttons[i]` is
    /// `mousearray[i + 1]`, and `G_Responder` stores `data1 & 1`, `& 2` and `& 4`
    /// (g_game.c:576-578), so the values are 0/1, 0/2 and 0/4. `mousearray[0]` is never written.
    mousearray: [i32; 4],
    mousex: i32,
    mousey: i32,
    turnheld: i32,
    dclick: DoubleClick,
    dclick2: DoubleClick,
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

/// What a successful build changes, computed before anything is written.
struct Next {
    cmd: TicCommand,
    turnheld: i32,
    dclick: DoubleClick,
    dclick2: DoubleClick,
}

impl InputState {
    /// The state at process start: every variable is a zero-initialised global
    /// (g_game.c:183-198).
    pub const fn new() -> Self {
        Self {
            gamekeydown: [false; NUMKEYS],
            mousearray: [0; 4],
            mousex: 0,
            mousey: 0,
            turnheld: 0,
            dclick: DoubleClick {
                time: 0,
                state: 0,
                clicks: 0,
            },
            dclick2: DoubleClick {
                time: 0,
                state: 0,
                clicks: 0,
            },
        }
    }

    /// `gamekeydown[key]`.
    pub fn key_down(&self, key: u8) -> bool {
        self.gamekeydown[usize::from(key)]
    }

    /// The keys `gamekeydown` holds, ascending.
    pub fn keys_down(&self) -> impl Iterator<Item = u8> + '_ {
        (0..=u8::MAX).filter(|k| self.gamekeydown[usize::from(*k)])
    }

    /// `mousearray[0..4]` with the raw stored values (index 0 is `mousebuttons[-1]`).
    pub fn mouse_array(&self) -> [i32; 4] {
        self.mousearray
    }

    /// `mousex` and `mousey`: the scaled motion of the last mouse event since the last build.
    pub fn mouse_motion(&self) -> (i32, i32) {
        (self.mousex, self.mousey)
    }

    /// `turnheld`. It is read only through `< SLOWTURNTICS` (g_game.c:272), so it saturates
    /// at `i32::MAX` instead of reaching the C's undefined overflow after 2^31 tics.
    pub fn turnheld(&self) -> i32 {
        self.turnheld
    }

    /// The forward-button double-click machine.
    pub fn double_click(&self) -> DoubleClick {
        self.dclick
    }

    /// The strafe-button double-click machine.
    pub fn strafe_double_click(&self) -> DoubleClick {
        self.dclick2
    }

    fn mouse_button(&self, binding: super::MouseBinding) -> i32 {
        self.mousearray[binding.array_index()]
    }

    /// `G_Responder`'s game switch (g_game.c:558-597), reached by every event the app posts
    /// (with the [`keys::WITHHELD`] keys withheld, `M_Responder`, the spy branch,
    /// `HU_Responder`, `ST_Responder` and `AM_Responder` pass every such event through
    /// unchanged; M5b investigation §2.2.4 "Never forwarding these keys"). Returns whether the
    /// event was eaten, as the C does:
    ///
    /// - keydown: `gamekeydown[data1] = true`, eaten (g_game.c:566-568);
    /// - keyup: `gamekeydown[data1] = false`, not eaten (g_game.c:570-573);
    /// - mouse: `mousebuttons[0..3] = data1 & 1, & 2, & 4` and
    ///   `mousex = data2*(mouseSensitivity+5)/10`, `mousey` likewise, eaten (g_game.c:575-581).
    ///   The motion is assigned, not added; the division truncates toward zero, as gcc's does.
    ///
    /// Refused, with nothing changed: a keydown of a [`keys::WITHHELD`] key (including
    /// `KEY_PAUSE`, whose `sendpause` branch, g_game.c:561-565, would write `BT_SPECIAL`), and a
    /// mouse event whose scaling overflows `int` ([`InputError::MouseOverflow`]).
    ///
    /// Because of that refusal `|mousex|` and `|mousey|` never exceed
    /// ⌊(2^31 − 1) / 10⌋ = 214,748,364, which keeps every later `int` operation of
    /// `G_BuildTiccmd` in range: `mousex*0x8` ≤ 1,717,986,912, and the `short`, `forward` and
    /// `side` terms it meets are at most 32,768, 100 and 80 in magnitude (g_game.c:298-325,
    /// 351-352, 405-409).
    pub fn respond(&mut self, controls: &Controls, event: Event) -> Result<bool, InputError> {
        match event {
            Event::KeyDown(k) => {
                if keys::is_withheld(k) {
                    return Err(InputError::WithheldKey(k));
                }
                self.gamekeydown[usize::from(k)] = true;
                Ok(true)
            }
            Event::KeyUp(k) => {
                self.gamekeydown[usize::from(k)] = false;
                Ok(false)
            }
            Event::Mouse { buttons, dx, dy } => {
                let factor = controls.mouse_sensitivity() + 5;
                let scale = |d: i32| {
                    d.checked_mul(factor)
                        .map(|v| v / 10)
                        .ok_or(InputError::MouseOverflow)
                };
                let (mx, my) = (scale(dx)?, scale(dy)?);
                self.mousearray[1] = i32::from(buttons & 1);
                self.mousearray[2] = i32::from(buttons & 2);
                self.mousearray[3] = i32::from(buttons & 4);
                self.mousex = mx;
                self.mousey = my;
                Ok(true)
            }
        }
    }

    /// `G_BuildTiccmd` (g_game.c:237-437), line by line, with `ticdup` 1 and the joystick terms
    /// false. The command starts as the zeroed `emptycmd` (g_game.c:249-250, i_system.c:64-68).
    ///
    /// No `int` operation here can overflow (see [`InputState::respond`]). The narrowings of
    /// `cmd->angleturn` to `short` are implementation-defined in C and wrap modulo 2^16 in gcc;
    /// they are reproduced.
    pub fn build_ticcmd(&mut self, controls: &Controls) -> BuiltCommand {
        let next = self.compute(controls);
        self.turnheld = next.turnheld;
        self.dclick = next.dclick;
        self.dclick2 = next.dclick2;
        // mousex = mousey = 0 (g_game.c:411)
        self.mousex = 0;
        self.mousey = 0;
        BuiltCommand(next.cmd)
    }

    fn compute(&self, c: &Controls) -> Next {
        let key = |k: u8| self.gamekeydown[usize::from(k)];
        let mut buttons: u8 = 0;
        let mut angleturn: i16 = 0;
        // g_game.c:256-258
        let strafe = key(c.key_strafe()) || self.mouse_button(c.mouseb_strafe()) != 0;
        let speed = usize::from(key(c.key_speed()));
        let (mut forward, mut side) = (0i32, 0i32);
        // two-stage turning (g_game.c:264-275)
        let turnheld = if key(c.key_right()) || key(c.key_left()) {
            self.turnheld.saturating_add(TICDUP)
        } else {
            0
        };
        let tspeed = if turnheld < SLOWTURNTICS { 2 } else { speed };
        // g_game.c:278-306; each `cmd->angleturn -= ...` narrows to short
        if strafe {
            if key(c.key_right()) {
                side += SIDEMOVE[speed];
            }
            if key(c.key_left()) {
                side -= SIDEMOVE[speed];
            }
        } else {
            if key(c.key_right()) {
                angleturn = (i32::from(angleturn) - ANGLETURN[tspeed]) as i16;
            }
            if key(c.key_left()) {
                angleturn = (i32::from(angleturn) + ANGLETURN[tspeed]) as i16;
            }
        }
        // g_game.c:308-325
        if key(c.key_up()) {
            forward += FORWARDMOVE[speed];
        }
        if key(c.key_down()) {
            forward -= FORWARDMOVE[speed];
        }
        if key(c.key_straferight()) {
            side += SIDEMOVE[speed];
        }
        if key(c.key_strafeleft()) {
            side -= SIDEMOVE[speed];
        }
        // buttons (g_game.c:330-339); chatchar is 0 (g_game.c:328, hu_stuff.c:600-615)
        if key(c.key_fire()) || self.mouse_button(c.mouseb_fire()) != 0 {
            buttons |= BT_ATTACK;
        }
        let mut dclick = self.dclick;
        let mut dclick2 = self.dclick2;
        if key(c.key_use()) {
            buttons |= BT_USE;
            dclick.clicks = 0;
        }
        // weapon keys '1'..'8', the lowest wins (g_game.c:342-348)
        for i in 0..NUMWEAPONS - 1 {
            if key(b'1' + i) {
                buttons |= BT_CHANGE;
                buttons |= i << BT_WEAPONSHIFT;
                break;
            }
        }
        // mouse forward and the double clicks (g_game.c:351-403)
        let forward_button = self.mouse_button(c.mouseb_forward());
        if forward_button != 0 {
            forward += FORWARDMOVE[speed];
        }
        if dclick.step(forward_button) {
            buttons |= BT_USE;
        }
        let bstrafe = i32::from(self.mouse_button(c.mouseb_strafe()) != 0);
        if dclick2.step(bstrafe) {
            buttons |= BT_USE;
        }
        // mouse motion (g_game.c:405-409), in range by the bound of `respond`
        forward += self.mousey;
        if strafe {
            side += self.mousex * 2;
        } else {
            angleturn = (i32::from(angleturn) - self.mousex * 0x8) as i16;
        }
        // clamps (g_game.c:413-420): both by MAXPLMOVE
        let forward = forward.clamp(-MAXPLMOVE, MAXPLMOVE);
        let side = side.clamp(-MAXPLMOVE, MAXPLMOVE);
        // cmd->forwardmove += forward; cmd->sidemove += side (g_game.c:422-423): in range
        Next {
            cmd: TicCommand {
                forward: forward as i8,
                side: side as i8,
                angle_turn: angleturn,
                buttons,
            },
            turnheld,
            dclick,
            dclick2,
        }
    }

    /// `G_DoLoadLevel`'s clear of the command-building state (g_game.c:490-496): `gamekeydown`,
    /// `mousex` and `mousey` are zeroed, and `mousebuttons[0]` (the left button) only.
    ///
    /// The genuine line is `memset (mousebuttons, 0, sizeof(mousebuttons))` (g_game.c:495),
    /// whose size is that of the pointer `boolean*` (g_game.c:187): 4 bytes on the 32-bit
    /// original, which clears `mousebuttons[0]`, and 8 under LP64, which also clears
    /// `mousebuttons[1]`. The genuine input driver applies the declared ILP32-faithful adapter
    /// `sizeof(boolean)` (`tools/reference/m5b_input_build.py`, `GAME_ABI_ADAPTERS`; M5b
    /// investigation §2.2.5 item 1), and its goldens are reproduced here. `turnheld` and the
    /// double-click machines are not cleared (decision S11 makes a restart a fresh
    /// [`InputState::new`]; this clear is for a level load within a session).
    pub fn load_level(&mut self) {
        self.gamekeydown = [false; NUMKEYS];
        self.mousex = 0;
        self.mousey = 0;
        self.mousearray[1] = 0;
    }
}
