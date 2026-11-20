//! The layout detector of decision T12 (doom_sound::detector), from cited facts: the release
//! origin's 64-bit reading (x = slot i+1's timer, y = 0) against every 32-bit reading
//! (x = a texture number, y = the timer), p_spec.c:1151 with M5c note §1.2.6.3 and §4.4.5;
//! audibility, volume and separation from S_AdjustSoundParams (s_sound.c:752-818) with the
//! same-position rule (s_sound.c:312-316). Rust-only tests, except the SNDBTN2 case, which
//! uses the positions of the genuine stream's row (tests/reference/m5c_sound/SNDBTN2.sound).

mod common;

use common::{directory, sound_set, texture_count};
use doom_sim::ActorId;
use doom_sound::detector::{DetectorError, LayoutDetector, MAX_TEXTURES};
use doom_sound::logic::{
    Listener, OriginPositions, S_CLIPPING_DIST, SoundOrigin, SoundPlacement, SoundRecord,
    adjust_sound_params,
};
use doom_sound::session::{Sound, SoundConfig};

const SWTCHN: u16 = 23;

fn placed(origin: [i32; 2], listener: [i32; 2], angle: u32) -> SoundPlacement {
    SoundPlacement {
        origin,
        listener,
        listener_angle: angle,
    }
}

/// The listener on the x axis so that the declared position (timer, 0) is exactly at the
/// distance where the volume is 1 (130 whole units inside S_CLIPPING_DIST: 8 * 130 / 1040,
/// s_sound.c:812-814); every 32-bit candidate (t, timer) is farther and gets volume 0.
fn at_the_audibility_edge(timer: i32) -> SoundPlacement {
    let edge = S_CLIPPING_DIST - (130 << 16);
    placed([timer, 0], [edge + timer, 0], 0)
}

#[test]
fn it_fires_when_a_candidate_flips_audibility_at_the_edge() {
    let at = at_the_audibility_edge(34);
    // The declared reading is audible with volume 1; texture 0's reading is not.
    let declared = adjust_sound_params(at.listener, 0, at.origin, 1, 8).unwrap();
    assert!(declared.audible());
    assert!(
        !adjust_sound_params(at.listener, 0, [0, 34], 1, 8)
            .unwrap()
            .audible()
    );
    let mut d = LayoutDetector::new(1).unwrap();
    assert!(d.check_start(40, &at, 1, 8).unwrap());
    assert_eq!((d.layout_dependent_from(), d.firings()), (Some(40), 1));
    // A later firing keeps the first tic and counts.
    assert!(d.check_start(41, &at, 1, 8).unwrap());
    assert_eq!((d.layout_dependent_from(), d.firings()), (Some(40), 2));
    // With the next slot free both builds read 0, 0: never a firing.
    let free = placed([0, 0], at.listener, 0);
    assert!(!d.check_start(42, &free, 1, 8).unwrap());
    // No texture, no candidate.
    assert!(
        !LayoutDetector::new(0)
            .unwrap()
            .check_start(40, &at, 1, 8)
            .unwrap()
    );
    // The same at a later S_UpdateSounds read (audible or not).
    let l = Listener {
        actor: ActorId::from_parts(0, 0),
        xy: at.listener,
        angle: 0,
    };
    let mut u = LayoutDetector::new(1).unwrap();
    assert!(u.check_update(77, &l, [34, 0], 1, 8).unwrap());
    assert!(!u.check_update(78, &l, [0, 0], 1, 8).unwrap());
    assert_eq!((u.layout_dependent_from(), u.firings()), (Some(77), 1));
}

#[test]
fn it_fires_when_a_candidate_changes_the_separation() {
    // Search the listener's angle for one where the declared reading (34, 0) and the 32-bit
    // reading (19, 34) give the same volume and audibility and different separations, with the
    // listener 64 units away (inside S_CLOSE_DIST, so the volume is 8 for both).
    let listener = [34 + (40 << 16), 30 << 16];
    let found = (0..1u64 << 20)
        .map(|k| (k << 12) as u32)
        .find(|&angle| {
            let a = adjust_sound_params(listener, angle, [34, 0], 1, 8).unwrap();
            let b = adjust_sound_params(listener, angle, [19, 34], 1, 8).unwrap();
            a != b && a.audible() && b.audible()
        })
        .expect("an angle at a separation boundary");
    let at = placed([34, 0], listener, found);
    // Texture numbers 0..20 include 19.
    let mut d = LayoutDetector::new(20).unwrap();
    assert!(d.check_start(5, &at, 1, 8).unwrap());
    // Only audibility counts at an update: the same listener gives no firing there.
    let l = Listener {
        actor: ActorId::from_parts(0, 0),
        xy: listener,
        angle: found,
    };
    let mut u = LayoutDetector::new(20).unwrap();
    assert!(!u.check_update(5, &l, [34, 0], 1, 8).unwrap());
    // The same-position rule: a candidate on the listener's x, y is centred (128).
    let on = placed([34, 0], [5, 34], 0);
    let declared = adjust_sound_params([5, 34], 0, [34, 0], 1, 8).unwrap();
    assert_ne!(
        declared,
        adjust_sound_params([5, 34], 0, [5, 34], 1, 8).unwrap()
    );
    let mut s = LayoutDetector::new(6).unwrap();
    assert!(s.check_start(9, &on, 1, 8).unwrap());
}

/// SNDBTN2: the busy-slot release of the genuine stream (S,128,6: slot 0 released while slot 1
/// is busy; 64-bit x, y = 34, 0; the genuine 32-bit build read 19, 34 and gave the same volume
/// 4 and separation 35, M5c note §5 T12). No texture number of the item's WAD changes anything.
#[test]
fn it_does_not_fire_on_sndbtn2() {
    let textures = texture_count(&directory(&["m5c_wads/sndbtn.wad"]));
    assert!(textures > 19);
    let at = placed([34, 0], [32_643_278, 7_238_990], 1_073_741_824);
    let mut d = LayoutDetector::new(textures).unwrap();
    assert!(!d.check_start(128, &at, 1, 8).unwrap());
    assert_eq!((d.layout_dependent_from(), d.firings()), (None, 0));
}

struct Buttons([i32; 2]);
impl OriginPositions for Buttons {
    fn actor_xy(&self, _: ActorId) -> Option<[i32; 2]> {
        None
    }
    fn sector_xy(&self, _: u16) -> Option<[i32; 2]> {
        None
    }
    fn button_xy(&self, _: u8) -> Option<[i32; 2]> {
        Some(self.0)
    }
}

/// Through a session: the label is the session tic, the session follows the 64-bit reading
/// (the start is audible and kept), and the later update read fires as well while the channel
/// is examined.
#[test]
fn a_session_labels_itself_and_continues() {
    let at = at_the_audibility_edge(34);
    let mut s = Sound::new(sound_set("m5c_sfx.wad"), SoundConfig::fresh(1, 3)).unwrap();
    let l = Listener {
        actor: ActorId::from_parts(0, 0),
        xy: at.listener,
        angle: 0,
    };
    let quiet = Buttons([0, 0]);
    s.tic(&[], 0, &l, &quiet).unwrap();
    let record = SoundRecord::Start {
        sfx: SWTCHN,
        origin: SoundOrigin::Button(0),
        at: Some(at),
    };
    let t = s.tic(&[record], 0, &l, &Buttons([34, 0])).unwrap();
    assert_eq!(t.draws, 1, "the 64-bit reading is audible");
    // The start fired and so did the update read of the same tic.
    assert_eq!(s.detector().layout_dependent_from(), Some(1));
    assert_eq!(s.detector().firings(), 2);
    assert!(s.logic().channels()[0].is_some());
    assert!(
        LayoutDetector::new(MAX_TEXTURES + 1) == Err(DetectorError::Textures)
            && LayoutDetector::from_parts(1, Some(3), 0) == Err(DetectorError::Label)
            && LayoutDetector::from_parts(1, None, 2) == Err(DetectorError::Label)
    );
    assert!(
        Sound::new(
            sound_set("m5c_sfx.wad"),
            SoundConfig::fresh(1, MAX_TEXTURES + 1)
        )
        .is_err()
    );
}
