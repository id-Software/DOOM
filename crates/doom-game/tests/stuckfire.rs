//! Reproduction attempts for the owner's stuck-fire report of 2026-11-18 (Windows, native
//! artifact, `--bindings modern`): "firing with the left mouse button while dragging the mouse
//! left or right leaves fire stuck on", that is, "a left-button release that arrives together
//! with motion is lost, while a release with no motion is delivered".
//!
//! These tests drive the app's event layer ([`EventLayer`], decisions S4 and S6 of the M5b
//! investigation §4) and the app's pump ([`App::mouse_button`], [`App::mouse_motion`],
//! [`App::update`]) with a press, motion and a release of the left button in the same pump and
//! in adjacent pumps, in every order, including rapid repeated clicks during continuous motion,
//! and check the mouse events that reach the game and the fire bit of every built command.
//!
//! What the game should see, and where that comes from:
//!
//! - `G_Responder` (g_game.c:575-581) sets `mousebuttons[0] = ev->data1 & 1` from every
//!   `ev_mouse`, whatever its motion, and `G_BuildTiccmd` sets `BT_ATTACK` (d_event.h:75) when
//!   `mousebuttons[mousebfire]` is set (g_game.c:330-332); `mouseb_fire` is 0, the left button,
//!   under both presets (m_misc.c:269; [`Controls::modern`] keeps it). So the fire bit
//!   of a command is the left bit of the last `ev_mouse` delivered before it. The genuine input
//!   driver shows it: in `ev_mouse_button_then_motion` and `x_fire_mouse`
//!   (tests/reference/m5b_input/goldens) an `ev_mouse` with `data1` 0 clears fire, with and
//!   without motion ([`the_genuine_driver_clears_fire_on_a_release_with_or_without_motion`]).
//! - linuxdoom's X layer posts one `ev_mouse` per ButtonPress, ButtonRelease and MotionNotify,
//!   each carrying the button mask (i_video.c:215-264). The app instead posts one accumulated
//!   mouse event per command build carrying the current buttons (decision S4, confirmed
//!   2026-11-17), and holds back a release that arrives in the same pump as its own press to
//!   the next pump (decision S6, confirmed 2026-11-17). Without linuxdoom's mask slip
//!   (i_video.c:218, 231, 247; quirk (4) of S4, declared not reproduced), the X stream of the
//!   same physical events leaves the same last left bit before each build as the app's stream,
//!   except for the held-back release, which S6 delays by exactly one build.
//!
//! So the expectation for every case is: before each build, the left bit is the physical
//! state after the pump's inputs, except that a release whose press came in the same pump
//! (with every later input of that button) waits for the next pump; and once the inputs stop,
//! a released button gives commands without `BT_ATTACK`. These are Rust tests of the app's
//! chosen stream against that expectation; the genuine path behind the stream is pinned by
//! the M5b goldens (tests/input_goldens.rs).

mod common;

use doom_game::app::{App, GameData, Host, SessionConfig, Timestamp};
use doom_game::input::{Controls, Event, EventLayer, Input, MouseButton, Preset};
use doom_render::GameMode;
use std::path::{Path, PathBuf};

/// An `ev_mouse`'s `data1`, `data2` and `data3`.
type MouseEvent = (u8, i32, i32);

/// `BT_ATTACK` (d_event.h:75).
const BT_ATTACK: u8 = 1;

/// One platform input of the left button or the mouse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum A {
    /// The left button went down.
    Press,
    /// The left button went up.
    Release,
    /// The mouse moved sideways by this many units.
    Motion(i32),
}

use A::{Motion, Press, Release};

/// The app's input path without the window: the event layer, then the genuine event ring,
/// `G_Responder` and `G_BuildTiccmd`, then the demo round trip.
struct Pipe {
    layer: EventLayer,
    input: Input,
}

/// What one build produced: the mouse events posted, the fire bit of the executed command and
/// `mousebuttons[0]` afterwards (`mousearray[1]`, g_game.c:186-187).
#[derive(Clone, Debug, Eq, PartialEq)]
struct Build {
    mouse: Vec<MouseEvent>,
    fire: bool,
    left: i32,
}

impl Pipe {
    fn new(preset: Preset) -> Self {
        Self {
            layer: EventLayer::new(&preset.controls(), preset.layer_options()),
            input: Input::new(preset.controls()),
        }
    }

    fn feed(&mut self, a: A) {
        match a {
            Press => assert!(self.layer.mouse_button(MouseButton::Left, true)),
            Release => assert!(self.layer.mouse_button(MouseButton::Left, false)),
            Motion(dx) => self.layer.mouse_motion(dx, 0),
        }
    }

    /// One pump: the inputs, then the drain and one command build.
    fn pump(&mut self, inputs: &[A]) -> Build {
        for a in inputs {
            self.feed(*a);
        }
        let mut events = Vec::new();
        self.layer.drain(&mut events);
        let mut mouse = Vec::new();
        for e in &events {
            if let Event::Mouse { buttons, dx, dy } = *e {
                mouse.push((buttons, dx, dy));
            }
            self.input.post(*e);
        }
        let executed = self.input.tic().expect("an admitted event").executed();
        Build {
            mouse,
            fire: executed.buttons & BT_ATTACK != 0,
            left: self.input.state().mouse_array()[1],
        }
    }
}

/// The expectation of the module documentation for a whole run of pumps, written from the
/// rule of S6 (a release in the pump of its own press waits, with every later input of that
/// button, for the next pump) and S4 (one event per build carrying the current buttons and the
/// pump's motion `<< 2`, i_video.c:250). Returns, per pump, the mouse event expected (if any)
/// and the left state the game holds after it.
fn expected(pumps: &[Vec<A>]) -> Vec<(Option<MouseEvent>, bool)> {
    let mut carry: Vec<A> = Vec::new();
    let (mut state, mut posted) = (false, false);
    let mut out = Vec::new();
    for inputs in pumps {
        let mut queue: Vec<A> = std::mem::take(&mut carry);
        queue.extend(inputs.iter().filter(|a| !matches!(a, Motion(_))));
        let motion: i32 = inputs
            .iter()
            .map(|a| if let Motion(d) = a { *d } else { 0 })
            .sum();
        let mut pressed_now = false;
        let mut waiting = false;
        for a in queue {
            if waiting {
                carry.push(a);
                continue;
            }
            match a {
                Press if !state => {
                    state = true;
                    pressed_now = true;
                }
                Release if state && pressed_now => {
                    waiting = true;
                    carry.push(a);
                }
                Release if state => state = false,
                _ => {} // a press while down, a release while up
            }
        }
        let dx = motion.clamp(-319, 319) << 2;
        let event = (dx != 0 || state != posted).then_some((u8::from(state), dx, 0));
        posted = state;
        out.push((event, state));
    }
    out
}

fn run_case(pumps: &[Vec<A>], preset: Preset) -> Vec<Build> {
    let mut p = Pipe::new(preset);
    let mut builds = Vec::new();
    for inputs in pumps {
        builds.push(p.pump(inputs));
    }
    builds
}

/// Checks a case against [`expected`] and against the physical end state.
fn check(pumps: &[Vec<A>], preset: Preset) {
    let builds = run_case(pumps, preset);
    let want = expected(pumps);
    for (k, (b, (event, state))) in builds.iter().zip(&want).enumerate() {
        let got_event = match b.mouse.as_slice() {
            [] => None,
            [one] => Some(*one),
            more => panic!("{pumps:?} {preset:?} pump {k}: more than one mouse event {more:?}"),
        };
        assert_eq!(
            got_event, *event,
            "{pumps:?} {preset:?} pump {k}: mouse event"
        );
        assert_eq!(b.fire, *state, "{pumps:?} {preset:?} pump {k}: BT_ATTACK");
        assert_eq!(b.left, i32::from(*state), "{pumps:?} {preset:?} pump {k}");
    }
    // the owner's symptom: once the inputs stop, a released button must not fire
    let physical = pumps.iter().flatten().fold(false, |s, a| match a {
        Press => true,
        Release => false,
        Motion(_) => s,
    });
    let last = builds.last().expect("at least one pump");
    assert_eq!(last.fire, physical, "{pumps:?} {preset:?}: the end state");
}

#[test]
fn the_genuine_driver_clears_fire_on_a_release_with_or_without_motion() {
    // The goldens of the genuine linuxdoom-1.10 input driver: an `ev_mouse` with data1 0
    // delivered in a tic gives that tic's command no BT_ATTACK and `mousearray[1]` 0 (MB bit 1
    // clear), with motion (x_fire_mouse tic 6: data2 680, data3 -440) and without
    // (ev_mouse_button_then_motion tic 2).
    for (name, tic, d2) in [
        ("x_fire_mouse", 6, 680),
        ("ev_mouse_button_then_motion", 2, 0),
    ] {
        let rows = common::read_golden(name);
        let delivered = rows.iter().any(|r| {
            matches!(r, common::Row::E { tic: t, ty: 2, d1: 0, d2: Some(x), .. }
                if *t == tic && *x == d2)
        });
        assert!(delivered, "{name}: the release event of tic {tic}");
        let t = rows
            .iter()
            .find_map(|r| match r {
                common::Row::T(t, row) if *t == tic => Some(row.clone()),
                _ => None,
            })
            .expect("the tic's row");
        assert_eq!(t.built[3] & i32::from(BT_ATTACK), 0, "{name}: built fire");
        assert_eq!(t.executed[3] & i32::from(BT_ATTACK), 0, "{name}: executed");
        assert_eq!(t.mb & 2, 0, "{name}: mousebuttons[0]");
        // the same event through the app's genuine path gives the same
        let mut input = Input::new(Controls::vanilla());
        input.post(Event::Mouse {
            buttons: 1,
            dx: 0,
            dy: 0,
        });
        assert_eq!(input.tic().unwrap().executed().buttons & BT_ATTACK, 1);
        input.post(Event::Mouse {
            buttons: 0,
            dx: d2 as i32,
            dy: 0,
        });
        assert_eq!(input.tic().unwrap().executed().buttons & BT_ATTACK, 0);
        assert_eq!(input.state().mouse_array()[1], 0);
    }
}

#[test]
fn press_motion_and_release_in_one_pump_in_every_order() {
    // every order of one press, one motion and one release inside one pump, then still and
    // moving pumps after it
    let perms: [[A; 3]; 6] = [
        [Press, Motion(5), Release],
        [Press, Release, Motion(5)],
        [Motion(5), Press, Release],
        [Release, Press, Motion(5)],
        [Release, Motion(5), Press],
        [Motion(5), Release, Press],
    ];
    for preset in [Preset::Vanilla, Preset::Modern] {
        for p in perms {
            for after in [vec![], vec![Motion(3)]] {
                let pumps = vec![p.to_vec(), after.clone(), after.clone(), after];
                check(&pumps, preset);
            }
        }
    }
    // the owner's case spelled out: press, drag, release while dragging, keep dragging
    let pumps = vec![
        vec![Press, Motion(6)],
        vec![Motion(6)],
        vec![Motion(6), Release, Motion(6)],
        vec![Motion(6)],
    ];
    let b = run_case(&pumps, Preset::Modern);
    let mouse: Vec<_> = b.iter().map(|b| b.mouse.clone()).collect();
    assert_eq!(
        mouse,
        [
            vec![(1, 24, 0)],
            vec![(1, 24, 0)],
            vec![(0, 48, 0)],
            vec![(0, 24, 0)],
        ]
    );
    let fire: Vec<_> = b.iter().map(|b| b.fire).collect();
    assert_eq!(fire, [true, true, false, false]);
}

#[test]
fn press_motion_and_release_in_adjacent_pumps_in_every_order() {
    // the press and the release in different pumps, motion before, between or after each
    let place: [&[A]; 4] = [&[], &[Motion(4)], &[Motion(4), Motion(4)], &[]];
    for preset in [Preset::Vanilla, Preset::Modern] {
        for gap in 0..3 {
            for before_p in [false, true] {
                for after_p in [false, true] {
                    for before_r in [false, true] {
                        for after_r in [false, true] {
                            let mut pumps = Vec::new();
                            let mut first = Vec::new();
                            if before_p {
                                first.push(Motion(4));
                            }
                            first.push(Press);
                            if after_p {
                                first.push(Motion(4));
                            }
                            pumps.push(first);
                            for _ in 0..gap {
                                pumps.push(place[1].to_vec());
                            }
                            let mut last = Vec::new();
                            if before_r {
                                last.push(Motion(-4));
                            }
                            last.push(Release);
                            if after_r {
                                last.push(Motion(-4));
                            }
                            pumps.push(last);
                            pumps.push(place[0].to_vec());
                            pumps.push(place[2].to_vec());
                            pumps.push(place[3].to_vec());
                            check(&pumps, preset);
                        }
                    }
                }
            }
        }
    }
}

/// Every run of up to four pumps, each holding up to three inputs drawn from "the next change
/// of the left button" (a press when up, a release when down) and a motion, followed by three
/// pumps of continuous motion: 15^4 = 50,625 runs per preset.
#[test]
fn every_run_of_four_pumps_of_up_to_three_inputs() {
    let mut shapes: Vec<Vec<bool>> = vec![vec![]]; // true: the button changes, false: motion
    for len in 1..=3 {
        for bits in 0..(1u32 << len) {
            shapes.push((0..len).map(|i| bits >> i & 1 == 1).collect());
        }
    }
    assert_eq!(shapes.len(), 15);
    let mut runs = 0usize;
    for preset in [Preset::Vanilla, Preset::Modern] {
        for i in 0..shapes.len().pow(4) {
            let mut down = false;
            let mut pumps = Vec::new();
            let mut n = i;
            for _ in 0..4 {
                let shape = &shapes[n % shapes.len()];
                n /= shapes.len();
                let pump: Vec<A> = shape
                    .iter()
                    .map(|&change| {
                        if change {
                            down = !down;
                            if down { Press } else { Release }
                        } else {
                            Motion(7)
                        }
                    })
                    .collect();
                pumps.push(pump);
            }
            for _ in 0..3 {
                pumps.push(vec![Motion(7)]);
            }
            check(&pumps, preset);
            runs += 1;
        }
    }
    assert_eq!(runs, 2 * 50_625);
}

#[test]
fn rapid_repeated_clicks_during_continuous_motion() {
    // every pump moves; clicks at every phase: press and release in one pump with motion
    // around them, press at the end of one pump and release at the start of the next, two
    // clicks in one pump, a click every pump, a click every other pump
    let patterns: Vec<Vec<Vec<A>>> = vec![
        vec![vec![Motion(9), Press, Motion(9), Release, Motion(9)]; 12],
        vec![vec![Motion(9), Press], vec![Release, Motion(9)]],
        vec![
            vec![
                Press,
                Motion(9),
                Release,
                Motion(9),
                Press,
                Motion(9),
                Release
            ];
            6
        ],
        vec![
            vec![Press, Motion(9)],
            vec![Motion(9)],
            vec![Release, Motion(9)],
        ],
        vec![
            vec![Motion(9), Press, Motion(9)],
            vec![Motion(9), Release, Motion(9), Press, Motion(9)],
            vec![Motion(9), Release],
        ],
        vec![vec![Release, Motion(9), Press, Motion(9)]],
    ];
    for preset in [Preset::Vanilla, Preset::Modern] {
        for pat in &patterns {
            let mut pumps: Vec<Vec<A>> = Vec::new();
            for _ in 0..8 {
                pumps.extend(pat.iter().cloned());
            }
            // the stray release of the last pattern needs a release at the end
            if pumps
                .iter()
                .flatten()
                .rev()
                .find(|a| !matches!(a, Motion(_)))
                == Some(&Press)
            {
                pumps.push(vec![Motion(9), Release]);
            }
            // S6 passes at most one press of a button per build (a later release waits), so
            // a burst of clicks faster than one per build drains at one per build after it:
            // as many moving pumps as presses, then four more
            let presses = pumps.iter().flatten().filter(|a| **a == Press).count();
            for _ in 0..presses + 4 {
                pumps.push(vec![Motion(9)]);
            }
            check(&pumps, preset);
            let builds = run_case(&pumps, preset);
            assert!(
                builds.iter().rev().take(3).all(|b| !b.fire),
                "{preset:?} {pat:?}: fire is off once the clicks stop"
            );
        }
    }
}

/// A finding of these tests, not the owner's symptom: S6's hold-back passes at most one press
/// of a button per build, so clicks faster than one per build (35 a second) queue and are
/// played out at one per build after the burst, the fire bit staying on meanwhile (a release
/// and the next press in one build leave the left bit set, as the genuine `G_Responder` would
/// for the same two events, g_game.c:576). Nothing is lost and the queue empties by itself,
/// with or without motion; it is pinned here as the behaviour of the confirmed rule.
#[test]
fn clicks_faster_than_one_per_build_drain_at_one_per_build() {
    for motion in [0, 9] {
        let click = |m: i32| {
            let mut v = vec![
                Press,
                Motion(m),
                Release,
                Motion(m),
                Press,
                Motion(m),
                Release,
            ];
            v.retain(|a| *a != Motion(0));
            v
        };
        let mut pumps = vec![click(motion); 4]; // eight clicks in four builds
        pumps.extend(vec![vec![Motion(motion)]; 8]);
        pumps.iter_mut().for_each(|p| p.retain(|a| *a != Motion(0)));
        let fire: Vec<bool> = run_case(&pumps, Preset::Modern)
            .iter()
            .map(|b| b.fire)
            .collect();
        // on from the first build to the eighth (the eighth press), its release in the ninth
        let mut want = vec![true; 8];
        want.extend([false; 4]);
        assert_eq!(fire, want, "motion {motion}");
        check(&pumps, Preset::Modern);
    }
}

// ---- the app's own pump, on a synthetic map ----

fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}

#[derive(Default)]
struct NullHost;

impl Host for NullHost {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, _name: &str) -> bool {
        false
    }
    fn save(&mut self, _name: &str, _bytes: &[u8]) -> Result<(), String> {
        Ok(())
    }
}

/// The start of tic `k` on the fake clock (µs).
fn at(k: u64) -> u64 {
    (k * 1_000_000).div_ceil(35)
}

/// Runs pumps through [`App`] (one tic each, on the M5a AMP room) and returns the executed
/// commands' fire bits.
fn app_fire(pumps: &[Vec<A>], preset: Preset) -> Vec<bool> {
    let wad = std::fs::read(reference("m5a_wads/amp.wad")).expect("map WAD");
    let gfx = std::fs::read(reference("m5b_gfx.wad")).expect("gfx WAD");
    let data = GameData::load(&[&wad, &gfx], GameMode::Registered, 1, 2).expect("game data");
    let cfg = SessionConfig {
        preset,
        ..SessionConfig::default()
    };
    let mut host = NullHost;
    let mut app = App::new(data, cfg, &mut host);
    assert_eq!(app.update(0, &mut host).ran, 0);
    for (k, inputs) in pumps.iter().enumerate() {
        for a in inputs {
            match *a {
                Press => app.mouse_button(MouseButton::Left, true),
                Release => app.mouse_button(MouseButton::Left, false),
                Motion(dx) => app.mouse_motion(dx, 0),
            }
        }
        assert_eq!(app.update(at(k as u64 + 1), &mut host).ran, 1);
    }
    app.recorder()
        .commands()
        .iter()
        .map(|c| c.buttons & BT_ATTACK != 0)
        .collect()
}

#[test]
fn the_apps_pump_releases_fire_during_motion() {
    let cases: Vec<Vec<Vec<A>>> = vec![
        // press, drag, release while dragging, keep dragging
        vec![
            vec![Press, Motion(6)],
            vec![Motion(6)],
            vec![Motion(6), Release, Motion(6)],
            vec![Motion(6)],
            vec![Motion(6)],
        ],
        // press, motion and release in one pump, then dragging
        vec![
            vec![Press, Motion(6), Release],
            vec![Motion(6)],
            vec![Motion(6)],
        ],
        // rapid clicks while dragging
        vec![
            vec![Motion(6), Press, Motion(6), Release, Motion(6)],
            vec![Motion(6), Press, Motion(6), Release, Motion(6)],
            vec![Motion(6), Press, Motion(6)],
            vec![Release, Motion(6)],
            vec![Motion(6)],
            vec![Motion(6)],
        ],
    ];
    for preset in [Preset::Vanilla, Preset::Modern] {
        for pumps in &cases {
            let want: Vec<bool> = expected(pumps).iter().map(|(_, s)| *s).collect();
            assert_eq!(app_fire(pumps, preset), want, "{preset:?} {pumps:?}");
            assert_eq!(want.last(), Some(&false));
        }
    }
}
