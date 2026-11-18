//! linuxdoom-1.10's input path, from posted events to the command the game executes, and the
//! app's event layer in front of it.
//!
//! The pipeline of one command build, in the order of the `-singletics` branch of
//! `D_DoomLoop` (d_main.c:376-386) and of `G_Ticker`'s copy and demo write (g_game.c:662-669):
//!
//! 1. [`EventLayer`] (the app's choices S4–S7, not linuxdoom's X layer) turns platform input,
//!    already in DOOM key codes, into at most [`MAX_EVENTS_PER_BUILD`] [`Event`]s;
//! 2. [`EventQueue`] is `D_PostEvent`/`D_ProcessEvents`' 64-entry ring (d_main.c:141-177);
//! 3. [`InputState::respond`] is `G_Responder`'s game switch (g_game.c:558-597);
//! 4. [`InputState::build_ticcmd`] is `G_BuildTiccmd` (g_game.c:237-437);
//! 5. [`BuiltCommand::record`] is `G_WriteDemoTiccmd`'s write and re-read (g_game.c:1506-1523),
//!    which gives the command the game executes while recording (decision S3).
//!
//! [`Input`] bundles steps 2–5 with a [`Controls`] value.
//!
//! # Which commands can come out (wire 24)
//!
//! Every [`BuiltCommand`], and therefore every [`RecordedTic::executed`] (only
//! [`BuiltCommand::record`] makes a [`RecordedTic`]), is admitted by the
//! simulation's wire 24 (`PlayerMonstersEpisode1V2`): `forward` and `side` are clamped to
//! ±`MAXPLMOVE` = ±50 (g_game.c:171, 413-420) before the narrowing to `char`, so neither is
//! ever −128 or outside −127..127; the buttons are only ever ORed from `BT_ATTACK`, `BT_USE`
//! and `BT_CHANGE | i<<3` with `i` ≤ 7 (g_game.c:330-347, the weapon bits only together with
//! `BT_CHANGE`); and `BT_SPECIAL` cannot be produced, because the only two writers of it
//! (`sendpause` and `sendsave`, g_game.c:426-436) have no counterpart here: the keydown that
//! sets `sendpause` (`KEY_PAUSE`, g_game.c:561-565) is refused by [`InputState::respond`] as
//! [`InputError::WithheldKey`], and `sendsave` is set only from the menu (g_game.c:1267), which
//! is not ported. The demo round trip keeps `forward`, `side` and `buttons` unchanged
//! (g_game.c:1499-1502). `consistancy` and `chatchar` are 0 in single player
//! (g_game.c:252-253 with g_game.c:681-693; hu_stuff.c:600-615) and the simulation's
//! [`TicCommand`] has no field for them.

mod controls;
mod demo;
mod layer;
mod queue;
mod ticcmd;

pub use controls::{Controls, ControlsError, MouseBinding, Preset};
pub use demo::{RecordedTic, read_ticcmd_bytes, write_ticcmd_bytes};
pub use layer::{
    EventLayer, KeyRoute, LayerOptions, MAX_EVENTS_PER_BUILD, MOUSE_CLAMP_PIXELS, MouseButton,
};
pub use queue::{EventQueue, MAXEVENTS};
pub use ticcmd::{
    ANGLETURN, BuiltCommand, DoubleClick, FORWARDMOVE, InputState, MAXPLMOVE, SIDEMOVE,
    SLOWTURNTICS,
};

pub use doom_sim::TicCommand;

use std::fmt;

/// `NUMKEYS` (g_game.c:181): the size of `gamekeydown`, one entry per 8-bit DOOM key code.
pub const NUMKEYS: usize = 256;

/// DOOM key codes (doomdef.h:250-280). Printable keys are their lower-case ASCII code; the
/// linuxdoom X layer lowers `A`..`Z` (i_video.c:152-157).
pub mod keys {
    /// `KEY_RIGHTARROW` (doomdef.h:250).
    pub const KEY_RIGHTARROW: u8 = 0xae;
    /// `KEY_LEFTARROW` (doomdef.h:251).
    pub const KEY_LEFTARROW: u8 = 0xac;
    /// `KEY_UPARROW` (doomdef.h:252).
    pub const KEY_UPARROW: u8 = 0xad;
    /// `KEY_DOWNARROW` (doomdef.h:253).
    pub const KEY_DOWNARROW: u8 = 0xaf;
    /// `KEY_ESCAPE` (doomdef.h:254).
    pub const KEY_ESCAPE: u8 = 27;
    /// `KEY_ENTER` (doomdef.h:255).
    pub const KEY_ENTER: u8 = 13;
    /// `KEY_TAB` (doomdef.h:256).
    pub const KEY_TAB: u8 = 9;
    /// `KEY_F1` .. `KEY_F12` (doomdef.h:257-268), indexed from `KEY_F[0]` = F1.
    pub const KEY_F: [u8; 12] = [
        0x80 + 0x3b,
        0x80 + 0x3c,
        0x80 + 0x3d,
        0x80 + 0x3e,
        0x80 + 0x3f,
        0x80 + 0x40,
        0x80 + 0x41,
        0x80 + 0x42,
        0x80 + 0x43,
        0x80 + 0x44,
        0x80 + 0x57,
        0x80 + 0x58,
    ];
    /// `KEY_BACKSPACE` (doomdef.h:270).
    pub const KEY_BACKSPACE: u8 = 127;
    /// `KEY_PAUSE` (doomdef.h:271).
    pub const KEY_PAUSE: u8 = 0xff;
    /// `KEY_EQUALS` (doomdef.h:273).
    pub const KEY_EQUALS: u8 = 0x3d;
    /// `KEY_MINUS` (doomdef.h:274).
    pub const KEY_MINUS: u8 = 0x2d;
    /// `KEY_RSHIFT` (doomdef.h:276); the X layer maps both Shift keys to it (i_video.c:135-138).
    pub const KEY_RSHIFT: u8 = 0x80 + 0x36;
    /// `KEY_RCTRL` (doomdef.h:277); both Ctrl keys (i_video.c:140-143).
    pub const KEY_RCTRL: u8 = 0x80 + 0x1d;
    /// `KEY_RALT` (doomdef.h:278), equal to `KEY_LALT` (doomdef.h:280); both Alt and Meta keys
    /// (i_video.c:145-150).
    pub const KEY_RALT: u8 = 0x80 + 0x38;

    /// The keys the app never forwards to the game path (decision S5 (i)), and which
    /// [`super::InputState::respond`] refuses as a keydown. In a level with the menu, automap
    /// and chat closed, linuxdoom acts on each of them before or instead of `gamekeydown`
    /// (M5b investigation §1.1.2 fact 15 and §1.2.2 fact 1):
    ///
    /// - Esc, `-`, `=` and F1–F11: `M_Responder` eats them (m_menu.c:1519-1617);
    /// - F12: `G_Responder`'s spy branch eats it (g_game.c:507-518);
    /// - Enter: `HU_Responder`'s message refresh eats it (hu_stuff.c:659-664);
    /// - Tab: `AM_Responder` opens the automap and eats it (am_map.c:625-631);
    /// - Pause: `G_Responder` sets `sendpause` (g_game.c:561-565), whose command carries
    ///   `BT_SPECIAL` (g_game.c:426-430), which wire 24 refuses;
    /// - `q`: while recording, `G_WriteDemoTiccmd` ends the demo and exits when
    ///   `gamekeydown['q']` is set (g_game.c:1508-1509), and every app session records (S3).
    pub const WITHHELD: [u8; 19] = [
        KEY_ESCAPE, KEY_ENTER, KEY_TAB, KEY_PAUSE, KEY_MINUS, KEY_EQUALS, b'q', KEY_F[0], KEY_F[1],
        KEY_F[2], KEY_F[3], KEY_F[4], KEY_F[5], KEY_F[6], KEY_F[7], KEY_F[8], KEY_F[9], KEY_F[10],
        KEY_F[11],
    ];

    /// Whether `key` is one of [`WITHHELD`].
    pub fn is_withheld(key: u8) -> bool {
        WITHHELD.contains(&key)
    }
}

/// An input event, the analogue of `event_t` (d_event.h:35-50) for the event kinds the app
/// posts. `ev_joystick` is not represented: nothing in linuxdoom posts it (only i_video.c posts
/// events, i_video.c:206, 212, 225, 241, 260), so `joyxmove`, `joyymove` and `joybuttons` are
/// always zero and every joystick term of `G_BuildTiccmd` is false.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    /// `ev_keydown` with `data1` = a DOOM key code ([`keys`]).
    KeyDown(u8),
    /// `ev_keyup` with `data1` = a DOOM key code.
    KeyUp(u8),
    /// `ev_mouse`. `buttons` is `data1`, of which `G_Responder` reads bits 0, 1 and 2 (left,
    /// middle and right, g_game.c:576-578; i_video.c:219-223); `dx` and `dy` are `data2` and
    /// `data3`, with `dy` positive for motion away from the player (i_video.c:250-251).
    Mouse {
        /// `data1`: the button bits.
        buttons: u8,
        /// `data2`: horizontal motion, positive to the right.
        dx: i32,
        /// `data3`: vertical motion, positive forward.
        dy: i32,
    },
}

/// A refused input. The refused event itself changes nothing in the responder and
/// command-building state, and [`Input`] drops it from its ring; the events after it stay
/// queued and no command is built ([`Input::tic`]). The events before it in the same call were
/// already responded to and keep their effect: posting `KeyDown(b'w')` and then a withheld
/// `KeyDown(keys::KEY_F[0])` (F1) and calling [`Input::tic`] sets `gamekeydown['w']` and returns
/// [`InputError::WithheldKey`]. (In the app a refusal ends the session.)
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputError {
    /// A keydown of a [`keys::WITHHELD`] key: linuxdoom would act on it outside the command
    /// stream or write `BT_SPECIAL`, neither of which the app models.
    WithheldKey(u8),
    /// A mouse event whose `data2*(mouseSensitivity+5)` or `data3*(...)` overflows `int`
    /// (g_game.c:579-580): undefined behaviour in C, where the strict build wraps and the
    /// overflow build stops (M5b investigation §2.2.5 item 4). The app's clamp keeps
    /// `|data2|, |data3| ≤ 1276`, far inside.
    MouseOverflow,
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WithheldKey(k) => write!(f, "key 0x{k:02x} is withheld from the game path"),
            Self::MouseOverflow => write!(f, "mouse motion overflows the genuine int scaling"),
        }
    }
}

impl std::error::Error for InputError {}

/// The genuine input path of one console player: the event ring, `G_Responder`'s game part and
/// `G_BuildTiccmd`'s state, over one [`Controls`] value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Input {
    controls: Controls,
    state: InputState,
    queue: EventQueue,
}

impl Input {
    /// The state at process start (every variable zero, g_game.c:183-198; an empty ring,
    /// d_main.c:141-143) with `controls`.
    pub fn new(controls: Controls) -> Self {
        Self {
            controls,
            state: InputState::new(),
            queue: EventQueue::new(),
        }
    }

    /// The controls in use.
    pub fn controls(&self) -> &Controls {
        &self.controls
    }

    /// The responder and command-building state.
    pub fn state(&self) -> &InputState {
        &self.state
    }

    /// `D_PostEvent` (d_main.c:150-154).
    pub fn post(&mut self, event: Event) {
        self.queue.post(event);
    }

    /// `D_ProcessEvents` (d_main.c:161-177) with only `G_Responder`'s game part behind it,
    /// calling `delivered` for each event after it is responded to. On an error the responder
    /// state is unchanged by the refused event, which is dropped from the ring; the events
    /// after it stay queued, so the next call goes on with them.
    pub fn process_events_with(
        &mut self,
        mut delivered: impl FnMut(&Event),
    ) -> Result<(), InputError> {
        let Self {
            controls,
            state,
            queue,
        } = self;
        queue.process(|event| {
            state.respond(controls, event)?;
            delivered(&event);
            Ok(())
        })
    }

    /// [`Input::process_events_with`] without an observer.
    pub fn process_events(&mut self) -> Result<(), InputError> {
        self.process_events_with(|_| {})
    }

    /// `G_BuildTiccmd` (g_game.c:237-437).
    pub fn build_ticcmd(&mut self) -> BuiltCommand {
        self.state.build_ticcmd(&self.controls)
    }

    /// Processes the posted events, builds the command and records it: one tic of the input
    /// path. Returns the built, demo and executed values.
    pub fn tic(&mut self) -> Result<RecordedTic, InputError> {
        self.process_events()?;
        Ok(self.build_ticcmd().record())
    }

    /// `G_DoLoadLevel`'s clear of the command-building state (g_game.c:490-496); see
    /// [`InputState::load_level`]. The ring is not touched.
    pub fn load_level(&mut self) {
        self.state.load_level();
    }
}
