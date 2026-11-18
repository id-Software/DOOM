//! Rules of `doom_sound::session::Sound` and its schedule, each from a cited fact (Rust-only
//! safety and order tests; the genuine comparison of whole sessions is tests/session_gate.rs).
//!
//! - A fresh session is a fresh process (decision T15): tic 0, no channel, handle counter 0.
//! - Per tic: the records in order with `gametic == t`, then `S_UpdateSounds` with
//!   `gametic == t + 1` (decision T5), then buffer k after tic floor(512k/315).
//! - A refused tic (a record or the update) leaves the whole session equal to a clone and hands
//!   back no index: the caller's `M_Random` index is unchanged.

mod common;

use common::sound_set;
use doom_sim::ActorId;
use doom_sound::logic::{
    Listener, OriginPositions, RecordEcho, SoundOrigin, SoundPlacement, SoundRecord, SoundRefusal,
    StartOutcome, StopReason,
};
use doom_sound::schedule::Schedule;
use doom_sound::session::{RefusedAt, SessionRefusal, Sound, SoundConfig};

const PISTOL: u16 = 1;
const ITEMUP: u16 = 32;
const DOROPN: u16 = 20;

struct Positions {
    actor: Option<[i32; 2]>,
}
impl OriginPositions for Positions {
    fn actor_xy(&self, _: ActorId) -> Option<[i32; 2]> {
        self.actor
    }
    fn sector_xy(&self, _: u16) -> Option<[i32; 2]> {
        Some([0, 0])
    }
    fn button_xy(&self, _: u8) -> Option<[i32; 2]> {
        Some([0, 0])
    }
}

fn player() -> ActorId {
    ActorId::from_parts(0, 0)
}

fn monster() -> ActorId {
    ActorId::from_parts(5, 1)
}

fn listener() -> Listener {
    Listener {
        actor: player(),
        xy: [0, 0],
        angle: 0,
    }
}

fn session() -> Sound {
    Sound::new(sound_set("m5c_sfx.wad"), SoundConfig::fresh(1, 0)).expect("session")
}

/// A monster's start 64 units east of the listener: audible (s_sound.c:796-815).
fn monster_start(sfx: u16) -> SoundRecord {
    SoundRecord::Start {
        sfx,
        origin: SoundOrigin::Actor(monster()),
        at: Some(SoundPlacement {
            origin: [64 << 16, 0],
            listener: [0, 0],
            listener_angle: 0,
        }),
    }
}

#[test]
fn a_fresh_session_is_a_fresh_process() {
    let s = session();
    assert_eq!(s.schedule(), Schedule::new());
    assert!(s.logic().channels().iter().all(Option::is_none));
    assert_eq!(s.logic().channels().len(), 3);
    assert_eq!(s.mixer().handle_counter(), 0);
    assert_eq!(s.mixer().mask(), 0);
}

#[test]
fn records_run_at_gametic_t_and_the_update_at_t_plus_one() {
    let mut s = session();
    let at = Positions {
        actor: Some([64 << 16, 0]),
    };
    // Tic 0: a NULL-origin pistol (one draw, s_sound.c:337-346) and an itemup (no draw).
    let records = [
        SoundRecord::Start {
            sfx: PISTOL,
            origin: SoundOrigin::None,
            at: None,
        },
        monster_start(ITEMUP),
    ];
    let t = s.tic(&records, 7, &listener(), &at).expect("tic 0");
    assert_eq!((t.tic, t.draws, t.rndindex), (0, 1, 8));
    let RecordEcho::Start(first) = t.records[0] else {
        panic!("start")
    };
    assert_eq!(
        (first.outcome, first.handle),
        (StartOutcome::Started, Some(100))
    );
    // Buffer 0 is mixed after tic 0.
    assert!(t.buffer.is_some());
    assert_eq!(s.schedule().tic(), 1);
    assert_eq!(s.schedule().buffers(), 1);
    // handle 100 and 101: still playing at gametic 1 (gametic < handle, i_sound.c:517-521).
    assert!(t.update.stops.is_empty());
    // The monster's channel was examined by the update (I_UpdateSoundParams, s_sound.c:597).
    assert_eq!(t.update.params.len(), 1);
    // Tic 1: buffer 1 is due (512 * 1 < 315 * 2); tic 2: none (512 * 2 >= 315 * 3).
    let t1 = s.tic(&[], 8, &listener(), &at).expect("tic 1");
    assert!(t1.buffer.is_some());
    let t2 = s.tic(&[], 8, &listener(), &at).expect("tic 2");
    assert!(t2.buffer.is_none());
    assert_eq!(s.schedule().buffers(), 2);
}

#[test]
fn a_removal_stop_frees_the_channel_within_the_tic() {
    let mut s = session();
    let at = Positions {
        actor: Some([64 << 16, 0]),
    };
    let records = [
        monster_start(DOROPN),
        SoundRecord::Stop { actor: monster() },
    ];
    let t = s.tic(&records, 0, &listener(), &at).expect("tic");
    let RecordEcho::Stop(Some(stop)) = t.records[1] else {
        panic!("stop")
    };
    assert_eq!(stop.reason, StopReason::Remove);
    assert!(stop.playing);
    assert!(s.logic().channels().iter().all(Option::is_none));
    // The mixer still plays it in the buffer after the tic (I_StopSound is empty,
    // i_sound.c:505-514; decision T13 item 2).
    let buffer = t.buffer.expect("buffer 0 after tic 0");
    assert!(buffer.iter().any(|f| *f != [0, 0]));
}

#[test]
fn a_refused_record_leaves_the_session_unchanged() {
    let mut s = session();
    let at = Positions {
        actor: Some([64 << 16, 0]),
    };
    s.tic(&[monster_start(PISTOL)], 3, &listener(), &at)
        .expect("tic 0");
    let before = s.clone();
    // A start that draws and takes a channel, then sound number 0 (s_sound.c:277-278).
    let records = [
        SoundRecord::Start {
            sfx: DOROPN,
            origin: SoundOrigin::None,
            at: None,
        },
        SoundRecord::Start {
            sfx: 0,
            origin: SoundOrigin::None,
            at: None,
        },
    ];
    let r = s.tic(&records, 4, &listener(), &at);
    assert_eq!(
        r,
        Err(SessionRefusal::Sound {
            tic: 1,
            at: RefusedAt::Record(1),
            refusal: SoundRefusal::SfxId { sfx: 0 }
        })
    );
    assert_eq!(s, before);
    // A button origin in slot 15 (p_spec.c:1151 reads past buttonlist).
    let r = s.tic(
        &[SoundRecord::Start {
            sfx: 23,
            origin: SoundOrigin::Button(15),
            at: Some(SoundPlacement {
                origin: [0, 0],
                listener: [0, 0],
                listener_angle: 0,
            }),
        }],
        4,
        &listener(),
        &at,
    );
    assert!(matches!(
        r,
        Err(SessionRefusal::Sound {
            refusal: SoundRefusal::ButtonPastArray { slot: 15 },
            ..
        })
    ));
    assert_eq!(s, before);
}

/// Review finding M1 (review-sound): an actor that is not the listener is always measured
/// (s_sound.c:304-320), so its start without a placement is refused with the session unchanged;
/// the listener's own body takes none.
#[test]
fn an_unplaced_start_of_another_actor_is_refused() {
    let mut s = session();
    let at = Positions {
        actor: Some([64 << 16, 0]),
    };
    let before = s.clone();
    let unplaced = |actor| SoundRecord::Start {
        sfx: DOROPN,
        origin: SoundOrigin::Actor(actor),
        at: None,
    };
    let r = s.tic(
        &[unplaced(player()), unplaced(monster())],
        2,
        &listener(),
        &at,
    );
    assert_eq!(
        r,
        Err(SessionRefusal::Sound {
            tic: 0,
            at: RefusedAt::Record(1),
            refusal: SoundRefusal::Placement
        })
    );
    assert_eq!(s, before);
    // A bad sound number is still refused as such first.
    let bad = SoundRecord::Start {
        sfx: 0,
        origin: SoundOrigin::Actor(monster()),
        at: None,
    };
    assert!(matches!(
        s.tic(&[bad], 2, &listener(), &at),
        Err(SessionRefusal::Sound {
            refusal: SoundRefusal::SfxId { sfx: 0 },
            ..
        })
    ));
    assert_eq!(s, before);
    // The listener's own body WITH a placement is refused too: the C never measures it
    // (s_sound.c:304), and the simulation never records one.
    let placed_listener = SoundRecord::Start {
        sfx: DOROPN,
        origin: SoundOrigin::Actor(player()),
        at: Some(SoundPlacement {
            origin: [64 << 16, 0],
            listener: [0, 0],
            listener_angle: 0,
        }),
    };
    assert_eq!(
        s.tic(&[placed_listener], 2, &listener(), &at),
        Err(SessionRefusal::Sound {
            tic: 0,
            at: RefusedAt::Record(0),
            refusal: SoundRefusal::Placement
        })
    );
    assert_eq!(s, before);
    // The listener's own body without a placement is its non-positional start.
    let t = s
        .tic(&[unplaced(player())], 2, &listener(), &at)
        .expect("tic");
    assert_eq!((t.draws, t.rndindex), (1, 3));
}

#[test]
fn a_refused_update_leaves_the_session_unchanged() {
    let mut s = session();
    let gone = Positions { actor: None };
    let before = s.clone();
    // The start is kept by the logic, then the update cannot read the monster's position.
    let r = s.tic(&[monster_start(DOROPN)], 9, &listener(), &gone);
    assert_eq!(
        r,
        Err(SessionRefusal::Sound {
            tic: 0,
            at: RefusedAt::Update,
            refusal: SoundRefusal::OriginGone { cnum: 0 }
        })
    );
    assert_eq!(s, before);
    assert_eq!(s.mixer().mask(), 0);
    assert_eq!(s.schedule().tic(), 0);
}

/// Review finding N3 (review-app): the tic's draw count cannot saturate silently. 65,535 starts
/// that each draw once (a NULL-origin pistol, s_sound.c:337-346) are counted exactly; one more is
/// refused at that record with the session unchanged.
#[test]
fn the_draw_count_is_exact_or_refused() {
    let at = Positions { actor: None };
    let pistol = SoundRecord::Start {
        sfx: PISTOL,
        origin: SoundOrigin::None,
        at: None,
    };
    let mut s = session();
    let records = vec![pistol; 65_535];
    let t = s.tic(&records, 0, &listener(), &at).expect("65,535 draws");
    assert_eq!((t.draws, t.rndindex), (65_535, 255));
    let mut s = session();
    let before = s.clone();
    let records = vec![pistol; 65_536];
    assert_eq!(
        s.tic(&records, 0, &listener(), &at),
        Err(SessionRefusal::DrawCount {
            tic: 0,
            record: 65_535
        })
    );
    assert_eq!(s, before);
}
