//! The app's event layer: the stream of [`Event`]s the app posts to the genuine path, decided
//! by S4 (one accumulated mouse event per command build), S5 (withheld keys), S6 (repeats,
//! same-pump releases, release on pause and focus loss) and S7 (always-run, no mouse forward
//! movement). This is the app's choice of stream, not linuxdoom's X layer (i_video.c), whose
//! behaviour the genuine driver records as evidence only. Every stream it produces is one the
//! genuine `G_Responder` could have received.

use super::controls::Controls;
use super::queue::MAXEVENTS;
use super::{Event, NUMKEYS, keys};
use std::collections::VecDeque;

/// The clamp of the motion in one mouse event, each way: the most a 320-pixel-wide linuxdoom
/// window can report in one X motion event (decision S4 (d); M5b investigation §2.2.4: with a
/// 320×200 window `|dx| ≤ 319`, so `|data2| ≤ 1276`). doom-play's motion is in raw device
/// units, not pixels (see [`EventLayer::mouse_motion`]); the bound keeps the genuine scaling
/// far from `int` overflow either way.
pub const MOUSE_CLAMP_PIXELS: i32 = 319;

/// The most events one [`EventLayer::drain`] produces: one fewer than `MAXEVENTS`
/// (d_event.h:108), so the ring of [`super::EventQueue`] never wraps onto its tail and loses
/// the batch (d_main.c:150-154). Events beyond it wait for the next build, in order.
pub const MAX_EVENTS_PER_BUILD: usize = MAXEVENTS - 1;

/// The most raw inputs held between two builds. Past it the layer releases everything, as on
/// focus loss ([`EventLayer::release_all`]), so its memory is bounded and no key is left stuck.
const MAX_PENDING: usize = 4 * NUMKEYS;

/// Options of the layer that a preset chooses ([`super::Preset::layer_options`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayerOptions {
    /// Always-run (S7): the layer posts a keydown of `key_speed` at the start and inverts the
    /// physical `key_speed` key. A held `key_speed` is the only genuine form of always-run
    /// (g_game.c:258; M5b investigation §1.1.5).
    pub always_run: bool,
    /// Whether vertical mouse motion is posted. When false (the S7 modern preset), the vertical
    /// motion is posted as zero, which is what a mouse moved only sideways produces.
    pub vertical_mouse: bool,
}

impl Default for LayerOptions {
    fn default() -> Self {
        Self {
            always_run: false,
            vertical_mouse: true,
        }
    }
}

/// What the layer did with a key from the platform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyRoute {
    /// Queued for the game path.
    Game,
    /// A [`keys::WITHHELD`] key: never forwarded (S5). The app may act on it (Esc and Pause
    /// are the app's own pause).
    Withheld,
    /// The raw-input bound was reached: everything was released as by
    /// [`EventLayer::release_all`], and this key was dropped.
    Overflowed,
}

/// A physical mouse button, mapped to the `data1` bits of linuxdoom's button events: left 1,
/// middle 2, right 4 (i_video.c:221-223).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseButton {
    /// Button 1, bit 0.
    Left,
    /// Button 2, bit 1.
    Middle,
    /// Button 3, bit 2.
    Right,
}

impl MouseButton {
    fn bit(self) -> u8 {
        match self {
            Self::Left => 1,
            Self::Middle => 2,
            Self::Right => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Raw {
    Key { code: u8, down: bool },
    Button { bit: u8, down: bool },
}

/// The app's event layer. Feed it platform input already translated to DOOM key codes with
/// [`EventLayer::key`], [`EventLayer::mouse_motion`] and [`EventLayer::mouse_button`], call
/// [`EventLayer::drain`] once before each command build and post what it produces, in order.
///
/// Per build (one "pump"):
///
/// - keys: a keydown of a key already down is a repeat and is dropped; a keyup of a key not down
///   is dropped; a keyup whose keydown was posted in the same drain is held back to the next
///   drain, together with every later input of that key, so a tap shorter than a tic still
///   lasts one tic (S6). With always-run the physical `key_speed` is inverted, so its tap is a
///   keyup then a keydown: that keydown is held back by the same rule, and a tap of the run
///   key shorter than a tic still gives one walking tic. The same rules apply to the mouse
///   buttons.
/// - mouse: one [`Event::Mouse`] at most, after the keys, carrying the current buttons and the
///   motion accumulated since the previous drain, each axis clamped to
///   ±[`MOUSE_CLAMP_PIXELS`] and scaled `<< 2` as linuxdoom's X layer does, with vertical motion
///   away from the player positive (i_video.c:250-251). It is posted only when there is motion
///   or the buttons changed, as an X server posts nothing for a still mouse.
/// - at most [`MAX_EVENTS_PER_BUILD`] events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventLayer {
    options: LayerOptions,
    speed_key: u8,
    ready: VecDeque<Event>,
    pending: VecDeque<Raw>,
    held: [bool; NUMKEYS],
    buttons: u8,
    posted_buttons: u8,
    motion: (i32, i32),
    force_mouse: bool,
}

impl EventLayer {
    /// A layer for `controls` (whose `key_speed` always-run holds) with `options`. With
    /// always-run, the first drain posts the keydown of `key_speed`.
    pub fn new(controls: &Controls, options: LayerOptions) -> Self {
        let speed_key = controls.key_speed();
        let mut layer = Self {
            options,
            speed_key,
            ready: VecDeque::new(),
            pending: VecDeque::new(),
            held: [false; NUMKEYS],
            buttons: 0,
            posted_buttons: 0,
            motion: (0, 0),
            force_mouse: false,
        };
        if options.always_run {
            layer.ready.push_back(Event::KeyDown(speed_key));
            layer.held[usize::from(speed_key)] = true;
        }
        layer
    }

    /// The options in use.
    pub fn options(&self) -> LayerOptions {
        self.options
    }

    /// Whether the game path has been told (or will be told by the next drain) that `key` is
    /// down.
    pub fn key_held(&self, key: u8) -> bool {
        self.held[usize::from(key)]
    }

    /// A physical key went down (`down`) or up. With always-run, `key_speed` is inverted.
    pub fn key(&mut self, code: u8, down: bool) -> KeyRoute {
        if keys::is_withheld(code) {
            return KeyRoute::Withheld;
        }
        let down = if self.options.always_run && code == self.speed_key {
            !down
        } else {
            down
        };
        if self.push(Raw::Key { code, down }) {
            KeyRoute::Game
        } else {
            KeyRoute::Overflowed
        }
    }

    /// A physical mouse button went down or up. Returns false when the raw-input bound was
    /// reached (see [`KeyRoute::Overflowed`]).
    pub fn mouse_button(&mut self, button: MouseButton, down: bool) -> bool {
        self.push(Raw::Button {
            bit: button.bit(),
            down,
        })
    }

    /// Relative mouse motion in the platform's units, `dy` positive downward as on screen. It is
    /// added up (saturating) until the next drain. linuxdoom's X layer reports window pixels;
    /// doom-play passes miniquad's raw mouse motion, which is in device units, not pixels, so
    /// the [`MOUSE_CLAMP_PIXELS`] bound applies to those units.
    pub fn mouse_motion(&mut self, dx: i32, dy: i32) {
        self.motion = (
            self.motion.0.saturating_add(dx),
            self.motion.1.saturating_add(dy),
        );
    }

    fn push(&mut self, raw: Raw) -> bool {
        if self.pending.len() >= MAX_PENDING {
            self.release_all();
            return false;
        }
        self.pending.push_back(raw);
        true
    }

    /// On the app's pause, minimise or focus loss (S6): forgets the inputs not yet posted and
    /// the accumulated motion, and makes the next drain post a keyup for every key the game
    /// path holds (ascending) and a mouse event with no buttons and no motion. Every physical
    /// key is then taken as up, so with always-run `key_speed` stays (or goes) down.
    pub fn release_all(&mut self) {
        self.pending.clear();
        for code in 0..=u8::MAX {
            let target = self.options.always_run && code == self.speed_key;
            let k = usize::from(code);
            if self.held[k] != target {
                self.held[k] = target;
                self.ready.push_back(if target {
                    Event::KeyDown(code)
                } else {
                    Event::KeyUp(code)
                });
            }
        }
        self.buttons = 0;
        self.motion = (0, 0);
        self.force_mouse = true;
    }

    /// The events to post before the next command build, appended to `out` in order.
    pub fn drain(&mut self, out: &mut Vec<Event>) {
        // one slot stays free for the mouse event
        let mut budget = MAX_EVENTS_PER_BUILD - 1;
        while budget > 0 {
            let Some(event) = self.ready.pop_front() else {
                break;
            };
            out.push(event);
            budget -= 1;
        }
        if self.ready.is_empty() {
            self.drain_pending(out, budget);
        }
        let clamp = |v: i32| v.clamp(-MOUSE_CLAMP_PIXELS, MOUSE_CLAMP_PIXELS);
        let dx = clamp(self.motion.0);
        let dy = if self.options.vertical_mouse {
            clamp(self.motion.1)
        } else {
            0
        };
        self.motion = (0, 0);
        if dx != 0 || dy != 0 || self.buttons != self.posted_buttons || self.force_mouse {
            out.push(Event::Mouse {
                buttons: self.buttons,
                dx: dx << 2,
                dy: -(dy << 2),
            });
            self.posted_buttons = self.buttons;
            self.force_mouse = false;
        }
    }

    fn drain_pending(&mut self, out: &mut Vec<Event>, mut budget: usize) {
        let mut down_now = [false; NUMKEYS];
        let mut up_now = [false; NUMKEYS];
        let inverted = |k: usize| self.options.always_run && k == usize::from(self.speed_key);
        let mut blocked = [false; NUMKEYS];
        let (mut buttons_down_now, mut buttons_blocked) = (0u8, 0u8);
        let mut carry = VecDeque::new();
        let mut full = false;
        while let Some(raw) = self.pending.pop_front() {
            if full {
                carry.push_back(raw);
                continue;
            }
            match raw {
                Raw::Key { code, down } => {
                    let k = usize::from(code);
                    if blocked[k] {
                        carry.push_back(raw);
                    } else if down {
                        if self.held[k] {
                            continue; // a repeat
                        }
                        if up_now[k] && inverted(k) {
                            blocked[k] = true; // the release of an always-run tap
                            carry.push_back(raw);
                            continue;
                        }
                        if budget == 0 {
                            full = true;
                            carry.push_back(raw);
                            continue;
                        }
                        out.push(Event::KeyDown(code));
                        budget -= 1;
                        self.held[k] = true;
                        down_now[k] = true;
                    } else {
                        if !self.held[k] {
                            continue; // not down
                        }
                        if down_now[k] {
                            blocked[k] = true;
                            carry.push_back(raw);
                            continue;
                        }
                        if budget == 0 {
                            full = true;
                            carry.push_back(raw);
                            continue;
                        }
                        out.push(Event::KeyUp(code));
                        budget -= 1;
                        self.held[k] = false;
                        up_now[k] = true;
                    }
                }
                Raw::Button { bit, down } => {
                    if buttons_blocked & bit != 0 {
                        carry.push_back(raw);
                    } else if down {
                        if self.buttons & bit == 0 {
                            self.buttons |= bit;
                            buttons_down_now |= bit;
                        }
                    } else if self.buttons & bit != 0 {
                        if buttons_down_now & bit != 0 {
                            buttons_blocked |= bit;
                            carry.push_back(raw);
                        } else {
                            self.buttons &= !bit;
                        }
                    }
                }
            }
        }
        self.pending = carry;
    }
}
