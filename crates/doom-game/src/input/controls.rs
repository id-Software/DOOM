//! The input configuration values of `defaults[]` (m_misc.c:234-299) and the two presets of
//! decision S7.

use super::keys;
use super::layer::LayerOptions;
use std::fmt;

/// A mouse-button binding (`mouseb_fire`, `mouseb_strafe`, `mouseb_forward`): an index into
/// `mousebuttons`, which points at `&mousearray[1]` so that −1 is legal (g_game.c:186-187).
/// `mousebuttons[-1]` is `mousearray[0]`, which nothing writes, so −1 binds no button.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MouseBinding(i8);

impl MouseBinding {
    /// No button: index −1.
    pub const NONE: Self = Self(-1);
    /// The left button: index 0, bit 0 of `ev_mouse`'s `data1`.
    pub const LEFT: Self = Self(0);
    /// The middle button: index 1, bit 1.
    pub const MIDDLE: Self = Self(1);
    /// The right button: index 2, bit 2.
    pub const RIGHT: Self = Self(2);

    /// The binding of configuration value `value`, which must be −1..=2: the indices
    /// `mousebuttons` can address (g_game.c:186-187, 576-578). linuxdoom does not validate
    /// (m_misc.c:388-393); out of range is undefined behaviour there and refused here.
    pub fn new(value: i32) -> Option<Self> {
        if (-1..=2).contains(&value) {
            Some(Self(value as i8))
        } else {
            None
        }
    }

    /// The configuration value, −1..=2.
    pub fn value(self) -> i32 {
        i32::from(self.0)
    }

    /// The index into `mousearray`, 0..=3.
    pub(crate) fn array_index(self) -> usize {
        (self.0 + 1) as usize
    }
}

/// A refused configuration value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlsError {
    /// Not one of the input names of `defaults[]` that [`Controls::set`] accepts.
    UnknownName(String),
    /// A key binding outside 0..=255, the range of `gamekeydown` (g_game.c:181-183).
    KeyOutOfRange {
        /// The configuration name.
        name: &'static str,
        /// The refused value.
        value: i32,
    },
    /// A key binding to a [`keys::WITHHELD`] key, which never reaches the game path.
    WithheldKey {
        /// The configuration name.
        name: &'static str,
        /// The refused key.
        value: u8,
    },
    /// A mouse-button binding outside −1..=2.
    MouseButtonOutOfRange {
        /// The configuration name.
        name: &'static str,
        /// The refused value.
        value: i32,
    },
    /// A `mouse_sensitivity` outside 0..=9, the range the menu keeps (m_menu.c:1113-1126).
    SensitivityOutOfRange(i32),
}

impl fmt::Display for ControlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownName(n) => write!(f, "unknown input setting {n:?}"),
            Self::KeyOutOfRange { name, value } => write!(f, "{name} {value} is not 0..=255"),
            Self::WithheldKey { name, value } => {
                write!(f, "{name} 0x{value:02x} is a key the app withholds")
            }
            Self::MouseButtonOutOfRange { name, value } => {
                write!(f, "{name} {value} is not -1..=2")
            }
            Self::SensitivityOutOfRange(v) => write!(f, "mouse_sensitivity {v} is not 0..=9"),
        }
    }
}

impl std::error::Error for ControlsError {}

const KEY_NAMES: [&str; 10] = [
    "key_right",
    "key_left",
    "key_up",
    "key_down",
    "key_strafeleft",
    "key_straferight",
    "key_fire",
    "key_use",
    "key_strafe",
    "key_speed",
];

const MOUSE_NAMES: [&str; 3] = ["mouseb_fire", "mouseb_strafe", "mouseb_forward"];

/// The input values `G_Responder` and `G_BuildTiccmd` read, under their `defaults[]` names
/// (m_misc.c:236, 243-253, 269-271). The joystick values are not kept: no joystick event is
/// ever posted (see [`super::Event`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Controls {
    /// `key_right`, `key_left`, `key_up`, `key_down`, `key_strafeleft`, `key_straferight`,
    /// `key_fire`, `key_use`, `key_strafe`, `key_speed`, in the order of `KEY_NAMES`.
    keys: [u8; 10],
    /// `mouseb_fire`, `mouseb_strafe`, `mouseb_forward`.
    mouse: [MouseBinding; 3],
    /// `mouse_sensitivity`.
    sensitivity: i32,
}

impl Default for Controls {
    fn default() -> Self {
        Self::vanilla()
    }
}

impl Controls {
    /// The genuine defaults (m_misc.c:236, 243-253, 269-271): arrows, `,` and `.` strafe,
    /// right Ctrl fire, space use, right Alt strafe, right Shift run; mouse left fire, middle
    /// strafe, right forward; sensitivity 5.
    pub const fn vanilla() -> Self {
        Self {
            keys: [
                keys::KEY_RIGHTARROW,
                keys::KEY_LEFTARROW,
                keys::KEY_UPARROW,
                keys::KEY_DOWNARROW,
                b',',
                b'.',
                keys::KEY_RCTRL,
                b' ',
                keys::KEY_RALT,
                keys::KEY_RSHIFT,
            ],
            mouse: [
                MouseBinding::LEFT,
                MouseBinding::MIDDLE,
                MouseBinding::RIGHT,
            ],
            sensitivity: 5,
        }
    }

    /// The configuration part of the S7 modern preset, exactly the values the genuine driver
    /// was given (`tools/reference/m5b_input_scripts.py`, `MODERN_CONFIG`): `key_up` `w`,
    /// `key_down` `s`, `key_strafeleft` `a`, `key_straferight` `d`, `key_use` `e`,
    /// `mouseb_strafe` and `mouseb_forward` −1. Every other value is the vanilla default.
    pub const fn modern() -> Self {
        let mut c = Self::vanilla();
        c.keys[2] = b'w';
        c.keys[3] = b's';
        c.keys[4] = b'a';
        c.keys[5] = b'd';
        c.keys[7] = b'e';
        c.mouse[1] = MouseBinding::NONE;
        c.mouse[2] = MouseBinding::NONE;
        c
    }

    /// Sets one value by its `defaults[]` name, as one row of a configuration file would
    /// (m_misc.c:384-393), but validated: keys 0..=255 and not [`keys::WITHHELD`], mouse
    /// buttons −1..=2, sensitivity 0..=9. A refusal changes nothing.
    pub fn set(&mut self, name: &str, value: i32) -> Result<(), ControlsError> {
        if let Some(i) = KEY_NAMES.iter().position(|n| *n == name) {
            let name = KEY_NAMES[i];
            let key =
                u8::try_from(value).map_err(|_| ControlsError::KeyOutOfRange { name, value })?;
            if keys::is_withheld(key) {
                return Err(ControlsError::WithheldKey { name, value: key });
            }
            self.keys[i] = key;
            Ok(())
        } else if let Some(i) = MOUSE_NAMES.iter().position(|n| *n == name) {
            let name = MOUSE_NAMES[i];
            self.mouse[i] = MouseBinding::new(value)
                .ok_or(ControlsError::MouseButtonOutOfRange { name, value })?;
            Ok(())
        } else if name == "mouse_sensitivity" {
            if !(0..=9).contains(&value) {
                return Err(ControlsError::SensitivityOutOfRange(value));
            }
            self.sensitivity = value;
            Ok(())
        } else {
            Err(ControlsError::UnknownName(name.to_owned()))
        }
    }

    /// The value of `name`, for the names [`Controls::set`] accepts.
    pub fn get(&self, name: &str) -> Option<i32> {
        if let Some(i) = KEY_NAMES.iter().position(|n| *n == name) {
            Some(i32::from(self.keys[i]))
        } else if let Some(i) = MOUSE_NAMES.iter().position(|n| *n == name) {
            Some(self.mouse[i].value())
        } else if name == "mouse_sensitivity" {
            Some(self.sensitivity)
        } else {
            None
        }
    }

    /// `key_right`.
    pub fn key_right(&self) -> u8 {
        self.keys[0]
    }
    /// `key_left`.
    pub fn key_left(&self) -> u8 {
        self.keys[1]
    }
    /// `key_up`.
    pub fn key_up(&self) -> u8 {
        self.keys[2]
    }
    /// `key_down`.
    pub fn key_down(&self) -> u8 {
        self.keys[3]
    }
    /// `key_strafeleft`.
    pub fn key_strafeleft(&self) -> u8 {
        self.keys[4]
    }
    /// `key_straferight`.
    pub fn key_straferight(&self) -> u8 {
        self.keys[5]
    }
    /// `key_fire`.
    pub fn key_fire(&self) -> u8 {
        self.keys[6]
    }
    /// `key_use`.
    pub fn key_use(&self) -> u8 {
        self.keys[7]
    }
    /// `key_strafe`.
    pub fn key_strafe(&self) -> u8 {
        self.keys[8]
    }
    /// `key_speed`.
    pub fn key_speed(&self) -> u8 {
        self.keys[9]
    }
    /// `mouseb_fire`.
    pub fn mouseb_fire(&self) -> MouseBinding {
        self.mouse[0]
    }
    /// `mouseb_strafe`.
    pub fn mouseb_strafe(&self) -> MouseBinding {
        self.mouse[1]
    }
    /// `mouseb_forward`.
    pub fn mouseb_forward(&self) -> MouseBinding {
        self.mouse[2]
    }
    /// `mouse_sensitivity`, 0..=9.
    pub fn mouse_sensitivity(&self) -> i32 {
        self.sensitivity
    }
}

/// The two binding presets of decision S7 (ii); vanilla is the default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Preset {
    /// The genuine defaults, with the layer forwarding the stream as it comes.
    #[default]
    Vanilla,
    /// The modern preset: [`Controls::modern`], always-run (the layer holds `key_speed`) and
    /// no forward movement from the mouse (the layer posts a vertical motion of zero).
    Modern,
}

impl Preset {
    /// The configuration values of the preset.
    pub const fn controls(self) -> Controls {
        match self {
            Self::Vanilla => Controls::vanilla(),
            Self::Modern => Controls::modern(),
        }
    }

    /// The event-layer options of the preset.
    pub const fn layer_options(self) -> LayerOptions {
        match self {
            Self::Vanilla => LayerOptions {
                always_run: false,
                vertical_mouse: true,
            },
            Self::Modern => LayerOptions {
                always_run: true,
                vertical_mouse: false,
            },
        }
    }
}
