//! The heads-up message of hu_stuff.c: the font (`HU_Init`, hu_stuff.c:392-412), `HU_Start`
//! (hu_stuff.c:419-484), `HU_Ticker` (hu_stuff.c:505-578), `HU_Drawer` and `HU_Erase`
//! (hu_stuff.c:486-503), fed by the per-tic message observer of §1.6 item 5 (decision S17).
//!
//! Single player: no chat, no title line (drawn only with the automap, hu_stuff.c:491-492), no
//! net messages. Facts: M5b investigation §1.5.
use crate::hu_lib::{FONTSTART, TextLine};
use crate::st_stuff::patch_named;
use crate::v_video::{Canvas, DrawError, Patch};
use crate::{ResourceError, Resources};
use doom_sim::{ActorStateEvent, Event, MoverEvent, MoverFunction, PickupEvent, PickupMessage};

/// `HU_MSGTIMEOUT` (hu_stuff.h:44): `4*TICRATE`.
pub const MSGTIMEOUT: i32 = 140;

/// A message the game assigned to `player->message` in a tic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Message {
    /// Its code in the genuine runners' numbering (`PICKUP_MESSAGES` of
    /// `tools/reference/m4a_driver.c`, 1 + the index; 41 is the turbo message; the bespoke
    /// scenario's cheat messages are -1).
    pub code: i32,
    /// The text (d_englsh.h).
    pub text: &'static str,
}

/// The source text and genuine code of a simulation message (d_englsh.h:80-130; the turbo text
/// is `sprintf("%s is turbo!", player_names[0])` with `HUSTR_PLRGREEN` "Green: ", g_game.c:675-678,
/// d_englsh.h:312).
pub fn message_text(m: PickupMessage) -> Option<Message> {
    use PickupMessage as P;
    let (code, text) = match m {
        P::None => return None,
        P::Clip => (22, "Picked up a clip."),
        P::BulletBox => (23, "Picked up a box of bullets."),
        P::Shells => (28, "Picked up 4 shotgun shells."),
        P::ShellBox => (29, "Picked up a box of shotgun shells."),
        P::Stimpack => (5, "Picked up a stimpack."),
        P::Medikit => (7, "Picked up a medikit."),
        P::MedikitNeeded => (6, "Picked up a medikit that you REALLY need!"),
        P::Shotgun => (36, "You got the shotgun!"),
        P::Chaingun => (32, "You got the chaingun!"),
        P::GreenArmor => (1, "Picked up the armor."),
        P::BlueArmor => (2, "Picked up the MegaArmor!"),
        P::HealthBonus => (3, "Picked up a health bonus."),
        P::ArmorBonus => (4, "Picked up an armor bonus."),
        P::Soulsphere => (8, "Supercharge!"),
        P::BlueCard => (9, "Picked up a blue keycard."),
        P::YellowCard => (10, "Picked up a yellow keycard."),
        P::RedCard => (11, "Picked up a red keycard."),
        P::Invisibility => (17, "Partial Invisibility"),
        P::RadSuit => (18, "Radiation Shielding Suit"),
        P::ComputerMap => (19, "Computer Area Map"),
        P::LightAmp => (20, "Light Amplification Visor"),
        P::Rocket => (24, "Picked up a rocket."),
        P::RocketBox => (25, "Picked up a box of rockets."),
        P::Backpack => (30, "Picked up a backpack full of ammo!"),
        P::Chainsaw => (33, "A chainsaw!  Find some meat!"),
        P::Launcher => (34, "You got the rocket launcher!"),
        P::BlueKeyNeeded => (38, "You need a blue key to open this door"),
        P::YellowKeyNeeded => (39, "You need a yellow key to open this door"),
        P::RedKeyNeeded => (40, "You need a red key to open this door"),
        P::Turbo => (41, "Green:  is turbo!"),
    };
    Some(Message { code, text })
}

/// Every simulation message, in code order (the display state decoder's table).
pub const ALL_MESSAGES: [PickupMessage; 31] = {
    use PickupMessage as P;
    [
        P::None,
        P::Clip,
        P::BulletBox,
        P::Shells,
        P::ShellBox,
        P::Stimpack,
        P::Medikit,
        P::MedikitNeeded,
        P::Shotgun,
        P::Chaingun,
        P::GreenArmor,
        P::BlueArmor,
        P::HealthBonus,
        P::ArmorBonus,
        P::Soulsphere,
        P::BlueCard,
        P::YellowCard,
        P::RedCard,
        P::Invisibility,
        P::RadSuit,
        P::ComputerMap,
        P::LightAmp,
        P::Rocket,
        P::RocketBox,
        P::Backpack,
        P::Chainsaw,
        P::Launcher,
        P::BlueKeyNeeded,
        P::YellowKeyNeeded,
        P::RedKeyNeeded,
        P::Turbo,
    ]
};

/// `sounds.h` `sfx_oof` (34), the sound of a locked door (p_doors.c:379, 393, 406).
const SFX_OOF: u16 = 34;

/// The last `player->message` assignment of a tic, derived from the tic's events and the command
/// as fed (§1.6 item 5): the turbo test first (`G_Ticker`, g_game.c:671-679, before `P_Ticker`),
/// then in event order every pickup message and every locked-door pair (`sfx_oof` without an
/// origin followed by the `EV_VerticalDoor` call that failed, p_doors.c:369-410). `gametic` is
/// the tic number before the tic. Cross-checked against the stored code: a derived assignment
/// must equal the stored code after the tic, and no assignment must leave it unchanged; any
/// mismatch is `Err` (a missed writer), never a guess.
pub fn derive_message(
    events: &[Event],
    gametic: u64,
    fed_forward: i8,
    stored_before: PickupMessage,
    stored_after: PickupMessage,
) -> Result<Option<PickupMessage>, PickupMessage> {
    let mut last = None;
    if fed_forward > 50 && gametic & 31 == 0 && (gametic >> 5) & 3 == 0 {
        last = Some(PickupMessage::Turbo);
    }
    let mut oof = false;
    for e in events {
        match e {
            Event::Pickup(PickupEvent::Message(m)) => last = Some(*m),
            Event::Mover(MoverEvent::Call {
                function: MoverFunction::VerticalDoor,
                argument,
                result: -1,
                created: 0,
                ..
            }) if oof => {
                last = match argument {
                    26 | 32 => Some(PickupMessage::BlueKeyNeeded),
                    27 | 34 => Some(PickupMessage::YellowKeyNeeded),
                    28 | 33 => Some(PickupMessage::RedKeyNeeded),
                    _ => return Err(stored_after),
                };
            }
            _ => {}
        }
        oof = matches!(
            e,
            Event::ActorState(ActorStateEvent::Sound {
                actor: None,
                sound: SFX_OOF
            })
        );
    }
    match last {
        Some(m) if m == stored_after => Ok(Some(m)),
        None if stored_before == stored_after => Ok(None),
        _ => Err(stored_after),
    }
}

/// The HUD's history (§1.5.11), saved in the display state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HudState {
    /// `message_on` (hu_stuff.c:98).
    pub message_on: bool,
    /// `message_counter` (hu_stuff.c:103; not reset by `HU_Start`).
    pub message_counter: i32,
    /// `message_nottobefuckedwith` (hu_stuff.c:100): the message on screen is protected.
    pub protected: bool,
    /// `message_dontfuckwithme` (hu_stuff.c:99): the pending message must be shown. Only the
    /// menu's message toggle sets it (m_menu.c:989), which the app does not offer.
    pub must_show: bool,
    /// `showMessages` (m_menu.c:79; default 1, m_misc.c:239), tested for nonzero.
    pub show_messages: i32,
    /// `w_message.l[0]`.
    pub line: TextLine,
    /// `w_message.laston` (hu_lib.h:73).
    pub laston: bool,
    /// The simulation's stored message code after the last observed tic (the cross-check of
    /// [`derive_message`]).
    pub stored: PickupMessage,
}

impl HudState {
    /// A fresh process after `HU_Start` (hu_stuff.c:425-482), with the stored code at load.
    pub fn level_start(stored: PickupMessage) -> Self {
        Self {
            message_on: false,
            message_counter: 0,
            protected: false,
            must_show: false,
            show_messages: 1,
            line: TextLine {
                text: Vec::new(),
                needsupdate: 1,
            },
            laston: true,
            stored,
        }
    }

    /// `HU_Ticker` (hu_stuff.c:505-533) with the tic's message (`plr->message`). Returns
    /// whether the message was taken (the genuine code then clears `plr->message`; a message
    /// not taken would stay pending, which the observer cannot carry: refused by the caller).
    pub fn tick(&mut self, message: Option<Message>) -> bool {
        if self.message_counter != 0 {
            self.message_counter -= 1;
            if self.message_counter == 0 {
                self.message_on = false;
                self.protected = false;
            }
        }
        let Some(m) = message else {
            return false;
        };
        if (self.show_messages != 0 || self.must_show) && (!self.protected || self.must_show) {
            self.line.set_message(m.text.as_bytes());
            self.message_on = true;
            self.message_counter = MSGTIMEOUT;
            self.protected = self.must_show;
            self.must_show = false;
            return true;
        }
        false
    }

    /// `HU_Erase` (hu_stuff.c:496-503) through `HUlib_eraseSText` (hu_lib.c:254-267).
    pub fn erase(&mut self) {
        if self.laston && !self.message_on {
            self.line.needsupdate = 4;
        }
        self.line.erase();
        self.laston = self.message_on;
    }

    /// `HU_Drawer` (hu_stuff.c:486-494): the message line at (`HU_MSGX`, `HU_MSGY`) = (0, 0)
    /// when `message_on` (`HUlib_drawSText`, hu_lib.c:231-252).
    pub(crate) fn draw(&self, c: &mut Canvas<'_>, font: &[Patch]) -> Result<(), DrawError> {
        if self.message_on {
            self.line.draw(c, [0, 0], font)?;
        }
        Ok(())
    }
}

/// The HUD font `STCFN033`..`STCFN095` (hu_stuff.c:405-410), `hu_font[c - '!']`.
pub fn load_font(res: &Resources) -> Result<Vec<Patch>, ResourceError> {
    (FONTSTART..=b'_')
        .map(|c| patch_named(res, &format!("STCFN{c:03}")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(text: &'static str) -> Option<Message> {
        Some(Message { code: 1, text })
    }

    /// hu_stuff.c:512-531: decrement first, then take; 140 tics; a repeat restarts the count; a
    /// message on the tic the count reaches zero is shown that tic.
    #[test]
    fn timeout_restart_and_same_tic_take() {
        let mut h = HudState::level_start(PickupMessage::None);
        assert!(h.tick(msg("A")));
        assert_eq!((h.message_on, h.message_counter), (true, 140));
        for _ in 0..139 {
            assert!(!h.tick(None));
            assert!(h.message_on);
        }
        assert!(!h.tick(None));
        assert_eq!((h.message_on, h.message_counter), (false, 0));
        assert!(!h.tick(None));
        assert_eq!(h.message_counter, 0);
        h.tick(msg("A"));
        for _ in 0..50 {
            h.tick(None);
        }
        assert!(h.tick(msg("A")));
        assert_eq!(h.message_counter, 140);
        for _ in 0..139 {
            h.tick(None);
        }
        assert!(h.tick(msg("B")));
        assert_eq!((h.message_on, h.line.text.as_slice()), (true, &b"B"[..]));
    }

    /// hu_stuff.c:518-523: messages off, a protected message and the toggle's privilege.
    #[test]
    fn show_messages_and_protection() {
        let mut h = HudState::level_start(PickupMessage::None);
        h.show_messages = 0;
        assert!(!h.tick(msg("A")));
        h.show_messages = -1;
        assert!(h.tick(msg("A")));
        h.must_show = true;
        assert!(h.tick(msg("B")));
        assert!(h.protected && !h.must_show);
        assert!(!h.tick(msg("C")));
        h.must_show = true;
        assert!(h.tick(msg("D")));
    }

    /// hu_lib.c:68-84: at most 80 characters; hu_lib.c:211-212 and 254-267: `needsupdate`.
    #[test]
    fn line_length_and_erase_bookkeeping() {
        let mut l = TextLine::default();
        l.set_message(&[b'x'; 100]);
        assert_eq!((l.text.len(), l.needsupdate), (80, 4));
        let mut h = HudState::level_start(PickupMessage::None);
        assert_eq!((h.line.needsupdate, h.laston), (1, true));
        h.erase();
        assert_eq!((h.line.needsupdate, h.laston), (3, false));
        h.tick(msg("A"));
        h.erase();
        assert_eq!((h.line.needsupdate, h.laston), (3, true));
    }

    /// Every simulation message has its genuine code and text; GOTMEDINEED (code 6) is
    /// unreachable (p_inter.c:476-483) but named.
    #[test]
    fn message_codes() {
        assert_eq!(message_text(PickupMessage::None), None);
        let all: Vec<i32> = (1..=30u8)
            .map(|c| {
                let m = ALL_MESSAGES[usize::from(c)];
                assert_eq!(m as u8, c);
                message_text(m).expect("text").code
            })
            .collect();
        let mut sorted = all.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 30, "{all:?}");
        assert_eq!(
            message_text(PickupMessage::Turbo).expect("t").text.len(),
            17
        );
    }

    /// The derivation: turbo only on gametic % 128 == 0 with forward > 50; a later pickup wins;
    /// the locked-door pair; the cross-check refuses a missed writer.
    #[test]
    fn derivation_and_cross_check() {
        use PickupMessage as P;
        assert_eq!(
            derive_message(&[], 128, 51, P::None, P::Turbo),
            Ok(Some(P::Turbo))
        );
        assert_eq!(derive_message(&[], 128, 50, P::Turbo, P::Turbo), Ok(None));
        assert_eq!(derive_message(&[], 32, 51, P::Clip, P::Clip), Ok(None));
        let pick = Event::Pickup(PickupEvent::Message(P::Clip));
        assert_eq!(
            derive_message(std::slice::from_ref(&pick), 0, 51, P::None, P::Clip),
            Ok(Some(P::Clip))
        );
        assert_eq!(derive_message(&[], 5, 0, P::None, P::Clip), Err(P::Clip));
        assert_eq!(
            derive_message(&[pick], 5, 0, P::None, P::Shells),
            Err(P::Shells)
        );
        let oof = Event::ActorState(ActorStateEvent::Sound {
            actor: None,
            sound: 34,
        });
        let call = |argument| {
            Event::Mover(MoverEvent::Call {
                function: MoverFunction::VerticalDoor,
                line: 3,
                argument,
                result: -1,
                created: 0,
            })
        };
        for (s, m) in [
            (26, P::BlueKeyNeeded),
            (34, P::YellowKeyNeeded),
            (33, P::RedKeyNeeded),
        ] {
            let ev = [oof.clone(), call(s)];
            assert_eq!(derive_message(&ev, 1, 0, P::None, m), Ok(Some(m)));
        }
        // A monster at a locked door: the call without the sound assigns nothing.
        assert_eq!(
            derive_message(&[call(26)], 1, 0, P::Clip, P::Clip),
            Ok(None)
        );
    }
}
