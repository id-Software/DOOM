//! miniquad key codes to DOOM key codes, following linuxdoom's X translation `xlatekey`
//! (i_video.c:97-160): named keys to their `KEY_*` codes, both Shift, Ctrl and Alt keys to one
//! code each, printable keys to lower-case ASCII. A key `xlatekey` would leave as a keysym of
//! 256 or more (keypad digits, Insert, Home and so on) never reaches `gamekeydown`
//! (the `data1 < NUMKEYS` tests, g_game.c:566 and 571), so it is not forwarded.

use doom_game::input::keys::{
    KEY_BACKSPACE, KEY_DOWNARROW, KEY_ENTER, KEY_EQUALS, KEY_ESCAPE, KEY_F, KEY_LEFTARROW,
    KEY_MINUS, KEY_PAUSE, KEY_RALT, KEY_RCTRL, KEY_RIGHTARROW, KEY_RSHIFT, KEY_TAB, KEY_UPARROW,
};
use miniquad::KeyCode;

/// The DOOM key code of `key`, or `None` when linuxdoom's game path would never see it.
pub fn doom_key(key: KeyCode) -> Option<u8> {
    use KeyCode as K;
    Some(match key {
        K::Left => KEY_LEFTARROW,
        K::Right => KEY_RIGHTARROW,
        K::Down => KEY_DOWNARROW,
        K::Up => KEY_UPARROW,
        K::Escape => KEY_ESCAPE,
        K::Enter => KEY_ENTER,
        K::Tab => KEY_TAB,
        K::F1 => KEY_F[0],
        K::F2 => KEY_F[1],
        K::F3 => KEY_F[2],
        K::F4 => KEY_F[3],
        K::F5 => KEY_F[4],
        K::F6 => KEY_F[5],
        K::F7 => KEY_F[6],
        K::F8 => KEY_F[7],
        K::F9 => KEY_F[8],
        K::F10 => KEY_F[9],
        K::F11 => KEY_F[10],
        K::F12 => KEY_F[11],
        K::Backspace | K::Delete => KEY_BACKSPACE,
        K::Pause => KEY_PAUSE,
        K::KpEqual | K::Equal => KEY_EQUALS,
        K::KpSubtract | K::Minus => KEY_MINUS,
        K::LeftShift | K::RightShift => KEY_RSHIFT,
        K::LeftControl | K::RightControl => KEY_RCTRL,
        K::LeftAlt | K::RightAlt => KEY_RALT,
        // XK_space..XK_asciitilde: the ASCII code, letters lowered (i_video.c:153-156)
        K::Space
        | K::Apostrophe
        | K::Comma
        | K::Period
        | K::Slash
        | K::Key0
        | K::Key1
        | K::Key2
        | K::Key3
        | K::Key4
        | K::Key5
        | K::Key6
        | K::Key7
        | K::Key8
        | K::Key9
        | K::Semicolon
        | K::LeftBracket
        | K::Backslash
        | K::RightBracket
        | K::GraveAccent => key as u8,
        K::A
        | K::B
        | K::C
        | K::D
        | K::E
        | K::F
        | K::G
        | K::H
        | K::I
        | K::J
        | K::K
        | K::L
        | K::M
        | K::N
        | K::O
        | K::P
        | K::Q
        | K::R
        | K::S
        | K::T
        | K::U
        | K::V
        | K::W
        | K::X
        | K::Y
        | K::Z => (key as u8).to_ascii_lowercase(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_follows_xlatekey() {
        use KeyCode as K;
        let cases: [(KeyCode, Option<u8>); 34] = [
            (K::Up, Some(0xad)),
            (K::Down, Some(0xaf)),
            (K::Left, Some(0xac)),
            (K::Right, Some(0xae)),
            (K::Escape, Some(27)),
            (K::Enter, Some(13)),
            (K::Tab, Some(9)),
            (K::F1, Some(0xbb)),
            (K::F10, Some(0xc4)),
            (K::F11, Some(0xd7)),
            (K::F12, Some(0xd8)),
            (K::Backspace, Some(127)),
            (K::Delete, Some(127)),
            (K::Pause, Some(0xff)),
            (K::Equal, Some(0x3d)),
            (K::KpEqual, Some(0x3d)),
            (K::Minus, Some(0x2d)),
            (K::KpSubtract, Some(0x2d)),
            (K::LeftShift, Some(0xb6)),
            (K::RightShift, Some(0xb6)),
            (K::LeftControl, Some(0x9d)),
            (K::RightControl, Some(0x9d)),
            (K::LeftAlt, Some(0xb8)),
            (K::RightAlt, Some(0xb8)),
            (K::Space, Some(b' ')),
            (K::Comma, Some(b',')),
            (K::Period, Some(b'.')),
            (K::Key1, Some(b'1')),
            (K::A, Some(b'a')),
            (K::W, Some(b'w')),
            (K::Z, Some(b'z')),
            (K::KpEnter, None),
            (K::Kp8, None),
            (K::LeftSuper, None),
        ];
        for (k, want) in cases {
            assert_eq!(doom_key(k), want, "{k:?}");
        }
        for k in [K::Insert, K::Home, K::CapsLock, K::Menu, K::Kp0, K::Unknown] {
            assert_eq!(doom_key(k), None, "{k:?}");
        }
    }
}
