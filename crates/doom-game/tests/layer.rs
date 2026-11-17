//! Unit tests of the app's event layer (decisions S4-S7 of the M5b investigation §4). These are
//! Rust-only tests of the app's chosen stream; the genuine path behind it is pinned by the
//! goldens, whose `s6_` and `modern_` scripts are the streams this layer produces.

mod common;

use doom_game::input::{
    Controls, Event, EventLayer, KeyRoute, LayerOptions, MAX_EVENTS_PER_BUILD, MOUSE_CLAMP_PIXELS,
    MouseButton, Preset, keys,
};
use keys::*;

fn vanilla() -> EventLayer {
    EventLayer::new(&Controls::vanilla(), LayerOptions::default())
}

fn modern() -> EventLayer {
    let p = Preset::Modern;
    EventLayer::new(&p.controls(), p.layer_options())
}

fn drain(l: &mut EventLayer) -> Vec<Event> {
    let mut out = Vec::new();
    l.drain(&mut out);
    out
}

use Event::{KeyDown, KeyUp};

fn mouse(buttons: u8, dx: i32, dy: i32) -> Event {
    Event::Mouse { buttons, dx, dy }
}

#[test]
fn keys_pass_in_order_and_idle_posts_nothing() {
    let mut l = vanilla();
    assert_eq!(drain(&mut l), []);
    assert_eq!(l.key(KEY_UPARROW, true), KeyRoute::Game);
    assert_eq!(l.key(b'3', true), KeyRoute::Game);
    assert_eq!(drain(&mut l), [KeyDown(KEY_UPARROW), KeyDown(b'3')]);
    assert!(l.key_held(KEY_UPARROW));
    assert_eq!(drain(&mut l), []);
    l.key(b'3', false);
    l.key(KEY_UPARROW, false);
    assert_eq!(drain(&mut l), [KeyUp(b'3'), KeyUp(KEY_UPARROW)]);
    assert!(!l.key_held(KEY_UPARROW));
}

#[test]
fn withheld_keys_never_forwarded() {
    // S5: Esc, Enter, Tab, Pause, '-', '=', 'q', F1-F12
    let mut l = vanilla();
    for k in keys::WITHHELD {
        assert_eq!(l.key(k, true), KeyRoute::Withheld);
        assert_eq!(l.key(k, false), KeyRoute::Withheld);
    }
    assert_eq!(drain(&mut l), []);
    assert_eq!(l.key(KEY_BACKSPACE, true), KeyRoute::Game);
}

#[test]
fn repeats_and_stray_releases_are_dropped() {
    // S6: a keydown of a key already down is a repeat
    let mut l = vanilla();
    l.key(KEY_LEFTARROW, true);
    l.key(KEY_LEFTARROW, true);
    assert_eq!(drain(&mut l), [KeyDown(KEY_LEFTARROW)]);
    l.key(KEY_LEFTARROW, true);
    assert_eq!(drain(&mut l), []);
    l.key(b'x', false);
    assert_eq!(drain(&mut l), []);
}

#[test]
fn release_in_the_press_pump_is_deferred() {
    // S6: a tap shorter than a tic still lasts one tic
    let mut l = vanilla();
    l.key(KEY_RCTRL, true);
    l.key(KEY_RCTRL, false);
    assert_eq!(drain(&mut l), [KeyDown(KEY_RCTRL)]);
    assert_eq!(drain(&mut l), [KeyUp(KEY_RCTRL)]);
    assert_eq!(drain(&mut l), []);
    // down, up, down in one pump: the later inputs of the key follow the deferred release, and
    // the key ends down
    l.key(b'a', true);
    l.key(b'a', false);
    l.key(b'a', true);
    l.key(b'b', true);
    assert_eq!(drain(&mut l), [KeyDown(b'a'), KeyDown(b'b')]);
    assert_eq!(drain(&mut l), [KeyUp(b'a'), KeyDown(b'a')]);
    assert!(l.key_held(b'a'));
    // up then down of a held key in one pump (an X auto-repeat pair): both pass, the key is
    // never seen up by a build
    l.key(b'a', false);
    l.key(b'a', true);
    assert_eq!(drain(&mut l), [KeyUp(b'a'), KeyDown(b'a')]);
}

#[test]
fn mouse_motion_accumulates_clamps_and_scales() {
    // S4: one event per build, accumulated, clamped to 319 pixels, << 2, forward positive
    let mut l = vanilla();
    l.mouse_motion(3, 1);
    l.mouse_motion(4, -6);
    assert_eq!(drain(&mut l), [mouse(0, 28, 20)]);
    assert_eq!(drain(&mut l), []);
    l.mouse_motion(400, -400);
    assert_eq!(drain(&mut l), [mouse(0, 1276, 1276)]);
    l.mouse_motion(-319, 319);
    assert_eq!(drain(&mut l), [mouse(0, -1276, -1276)]);
    l.mouse_motion(-320, 320);
    assert_eq!(drain(&mut l), [mouse(0, -1276, -1276)]);
    l.mouse_motion(i32::MAX, i32::MIN);
    l.mouse_motion(i32::MAX, i32::MIN);
    assert_eq!(drain(&mut l), [mouse(0, 1276, 1276)]);
    assert_eq!(MOUSE_CLAMP_PIXELS, 319);
    // motion that sums to zero posts nothing
    l.mouse_motion(5, 5);
    l.mouse_motion(-5, -5);
    assert_eq!(drain(&mut l), []);
}

#[test]
fn modern_preset_posts_no_vertical_motion() {
    let mut l = modern();
    assert_eq!(drain(&mut l), [KeyDown(KEY_RSHIFT)]);
    l.mouse_motion(0, 50);
    assert_eq!(drain(&mut l), []);
    l.mouse_motion(-10, 50);
    assert_eq!(drain(&mut l), [mouse(0, -40, 0)]);
}

#[test]
fn buttons_carry_current_state_with_deferral() {
    // bits 1, 2, 4 (i_video.c:221-223); a button change posts an event; a release in the press
    // pump is deferred like a key's
    let mut l = vanilla();
    assert!(l.mouse_button(MouseButton::Left, true));
    l.mouse_button(MouseButton::Right, true);
    assert_eq!(drain(&mut l), [mouse(5, 0, 0)]);
    l.mouse_button(MouseButton::Right, true);
    assert_eq!(drain(&mut l), []);
    l.mouse_button(MouseButton::Middle, true);
    l.mouse_button(MouseButton::Middle, false);
    l.mouse_button(MouseButton::Left, false);
    l.mouse_motion(1, 0);
    assert_eq!(drain(&mut l), [mouse(6, 4, 0)]);
    assert_eq!(drain(&mut l), [mouse(4, 0, 0)]);
    l.mouse_button(MouseButton::Middle, false);
    assert_eq!(drain(&mut l), []);
    l.mouse_button(MouseButton::Right, false);
    l.mouse_button(MouseButton::Right, true);
    assert_eq!(drain(&mut l), []);
}

#[test]
fn always_run_holds_and_inverts_the_speed_key() {
    // S7: key_speed down at the start; the physical key inverts it
    let mut l = modern();
    assert!(l.key_held(KEY_RSHIFT));
    assert_eq!(drain(&mut l), [KeyDown(KEY_RSHIFT)]);
    l.key(KEY_RSHIFT, true);
    assert_eq!(drain(&mut l), [KeyUp(KEY_RSHIFT)]);
    l.key(KEY_RSHIFT, true); // auto-repeat of the physical key
    assert_eq!(drain(&mut l), []);
    l.key(KEY_RSHIFT, false);
    assert_eq!(drain(&mut l), [KeyDown(KEY_RSHIFT)]);
    // without always-run the speed key is a plain key
    let mut v = vanilla();
    v.key(KEY_RSHIFT, true);
    assert_eq!(drain(&mut v), [KeyDown(KEY_RSHIFT)]);
}

#[test]
fn release_all_on_pause_or_focus_loss() {
    // S6: a keyup for every held key (ascending) and a zero mouse event; pending input and
    // motion are forgotten
    let mut l = vanilla();
    l.key(KEY_UPARROW, true);
    l.key(b'1', true);
    l.mouse_button(MouseButton::Left, true);
    assert_eq!(
        drain(&mut l),
        [KeyDown(KEY_UPARROW), KeyDown(b'1'), mouse(1, 0, 0)]
    );
    l.key(b'2', true);
    l.mouse_motion(9, 9);
    l.release_all();
    assert_eq!(
        drain(&mut l),
        [KeyUp(b'1'), KeyUp(KEY_UPARROW), mouse(0, 0, 0)]
    );
    assert_eq!(drain(&mut l), []);
    // with nothing held, still the zero mouse event
    l.release_all();
    assert_eq!(drain(&mut l), [mouse(0, 0, 0)]);
    // always-run: the speed key stays down, or goes down again if the physical key was held
    let mut m = modern();
    m.key(b'w', true);
    drain(&mut m);
    m.release_all();
    assert_eq!(drain(&mut m), [KeyUp(b'w'), mouse(0, 0, 0)]);
    m.key(KEY_RSHIFT, true);
    assert_eq!(drain(&mut m), [KeyUp(KEY_RSHIFT)]);
    m.release_all();
    assert_eq!(drain(&mut m), [KeyDown(KEY_RSHIFT), mouse(0, 0, 0)]);
    assert!(m.key_held(KEY_RSHIFT));
}

#[test]
fn at_most_63_events_per_build_in_order() {
    // MAXEVENTS - 1: the genuine ring never loses a batch (d_main.c:150-154)
    assert_eq!(MAX_EVENTS_PER_BUILD, 63);
    let mut l = vanilla();
    for k in 0..=u8::MAX {
        if !keys::is_withheld(k) {
            l.key(k, true);
        }
    }
    l.mouse_motion(1, 0);
    let mut all = Vec::new();
    loop {
        let out = drain(&mut l);
        assert!(out.len() <= MAX_EVENTS_PER_BUILD);
        if out.is_empty() {
            break;
        }
        all.extend(out);
    }
    let forwarded: Vec<u8> = (0..=u8::MAX).filter(|k| !keys::is_withheld(*k)).collect();
    let downs: Vec<u8> = all
        .iter()
        .filter_map(|e| if let KeyDown(k) = e { Some(*k) } else { None })
        .collect();
    assert_eq!(downs, forwarded);
    assert_eq!(
        all.iter()
            .filter(|e| matches!(e, Event::Mouse { .. }))
            .count(),
        1
    );
    // release_all of many held keys is spread over builds too
    l.release_all();
    let first = drain(&mut l);
    assert_eq!(first.len(), MAX_EVENTS_PER_BUILD);
    assert_eq!(first.last(), Some(&mouse(0, 0, 0)));
    let mut ups = first.len() - 1;
    loop {
        let out = drain(&mut l);
        if out.is_empty() {
            break;
        }
        assert!(out.len() <= MAX_EVENTS_PER_BUILD);
        ups += out.len();
    }
    assert_eq!(ups, forwarded.len());
}

#[test]
fn raw_input_is_bounded() {
    let mut l = vanilla();
    l.key(b'a', true);
    drain(&mut l);
    let mut route = KeyRoute::Game;
    for i in 0..2000 {
        route = l.key(b'b', i % 2 == 0);
        if route != KeyRoute::Game {
            break;
        }
    }
    assert_eq!(route, KeyRoute::Overflowed);
    // everything was released
    assert_eq!(drain(&mut l), [KeyUp(b'a'), mouse(0, 0, 0)]);
    let mut l = vanilla();
    let mut ok = true;
    for i in 0..2000 {
        ok &= l.mouse_button(MouseButton::Left, i % 2 == 0);
    }
    assert!(!ok);
    assert_eq!(l.options(), LayerOptions::default());
}

/// The events a script posts before each tic, indexed by tic.
fn script_pumps(name: &str) -> Vec<Vec<Event>> {
    let mut pumps = vec![Vec::new()];
    for d in common::read_script(name).directives {
        match d {
            common::Directive::Post(e) => pumps.last_mut().unwrap().push(e),
            common::Directive::Tic(n) => {
                for _ in 0..n {
                    pumps.push(Vec::new());
                }
            }
            _ => panic!("{name}"),
        }
    }
    pumps.pop();
    pumps
}

/// One platform action of a pump.
enum P {
    Key(u8, bool),
    Button(MouseButton, bool),
    Motion(i32, i32),
    ReleaseAll,
}

/// Runs `pumps` of platform actions through a layer and requires the script's stream; with
/// `as_set`, the events of a pump may come in another order (release_all posts ascending).
fn layer_produces(name: &str, preset: Preset, pumps: &[&[P]], as_set: bool) {
    let want = script_pumps(name);
    assert_eq!(pumps.len(), want.len(), "{name}");
    let mut l = EventLayer::new(&preset.controls(), preset.layer_options());
    for (tic, (actions, want)) in pumps.iter().zip(&want).enumerate() {
        for a in actions.iter() {
            match *a {
                P::Key(k, d) => {
                    l.key(k, d);
                }
                P::Button(b, d) => {
                    l.mouse_button(b, d);
                }
                P::Motion(x, y) => l.mouse_motion(x, y),
                P::ReleaseAll => l.release_all(),
            }
        }
        let mut got = drain(&mut l);
        let mut want = want.clone();
        if as_set {
            let key = |e: &Event| format!("{e:?}");
            got.sort_by_key(key);
            want.sort_by_key(key);
        }
        assert_eq!(got, want, "{name} tic {tic}");
    }
}

#[test]
fn the_layer_produces_the_s6_and_modern_streams_of_the_goldens() {
    // a tap inside one pump becomes the deferred stream (s6_tap_*_deferred)
    for (n, k) in [
        ("fire", KEY_RCTRL),
        ("use", b' '),
        ("weapon3", b'3'),
        ("left", KEY_LEFTARROW),
        ("up", KEY_UPARROW),
    ] {
        let tap: &[P] = &[P::Key(k, true), P::Key(k, false)];
        layer_produces(
            &format!("s6_tap_{n}_deferred"),
            Preset::Vanilla,
            &[&[], tap, &[], &[], &[]],
            false,
        );
    }
    // always-run and the inverted physical Shift
    let shift = KEY_RSHIFT;
    layer_produces(
        "modern_alwaysrun_shift_inverts",
        Preset::Modern,
        &[
            &[],
            &[P::Key(b'w', true)],
            &[],
            &[P::Key(shift, true)],
            &[],
            &[P::Key(shift, false)],
            &[],
            &[P::Key(b'w', false)],
        ],
        false,
    );
    // focus loss, pause and minimise release everything with a zero mouse event
    let (l, m, r) = (MouseButton::Left, MouseButton::Middle, MouseButton::Right);
    layer_produces(
        "s6_focus_loss",
        Preset::Vanilla,
        &[
            &[],
            &[
                P::Key(KEY_UPARROW, true),
                P::Key(KEY_LEFTARROW, true),
                P::Key(KEY_RCTRL, true),
                P::Button(l, true),
                P::Button(r, true),
                P::Motion(2, 0),
            ],
            &[],
            &[],
            &[P::ReleaseAll],
            &[],
            &[],
            &[P::Key(KEY_LEFTARROW, true)],
            &[],
            &[P::Key(KEY_LEFTARROW, false)],
        ],
        true,
    );
    let press = [
        P::Key(shift, true),
        P::Key(KEY_RIGHTARROW, true),
        P::Key(b'.', true),
        P::Button(m, true),
    ];
    let (release, right_down, right_up) = (
        [P::ReleaseAll],
        [P::Key(KEY_RIGHTARROW, true)],
        [P::Key(KEY_RIGHTARROW, false)],
    );
    let mut pause: Vec<&[P]> = vec![&[], &press];
    pause.extend([&[] as &[P]; 6]);
    pause.extend([&release as &[P], &[], &right_down, &[], &right_up]);
    layer_produces("s6_pause", Preset::Vanilla, &pause, true);
    layer_produces(
        "s6_minimise",
        Preset::Vanilla,
        &[
            &[],
            &[
                P::Key(KEY_RALT, true),
                P::Key(KEY_LEFTARROW, true),
                P::Key(b'2', true),
                P::Button(r, true),
            ],
            &[],
            &[P::ReleaseAll],
            &[],
            &[],
        ],
        true,
    );
}
