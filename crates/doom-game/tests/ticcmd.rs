//! Unit tests of the genuine input path from cited C facts (M5b investigation §1.1.3-§1.1.6 and
//! §1.2.2). Each table entry, clamp and boundary is pinned here as well as by the goldens.

mod common;

use doom_game::input::{
    ANGLETURN, Controls, ControlsError, Event, FORWARDMOVE, Input, InputError, InputState,
    MAXEVENTS, MAXPLMOVE, MouseBinding, Preset, SIDEMOVE, SLOWTURNTICS, TicCommand, keys,
    read_ticcmd_bytes, write_ticcmd_bytes,
};

use keys::*;

fn cmd(input: &mut Input) -> TicCommand {
    input.process_events().expect("events");
    input.build_ticcmd().command()
}

fn hold(input: &mut Input, key: u8) {
    input.post(Event::KeyDown(key));
}

fn mouse(buttons: u8, dx: i32, dy: i32) -> Event {
    Event::Mouse { buttons, dx, dy }
}

#[test]
fn tables_and_constants() {
    // g_game.c:171-179
    assert_eq!(FORWARDMOVE, [25, 50]);
    assert_eq!(SIDEMOVE, [24, 40]);
    assert_eq!(ANGLETURN, [640, 1280, 320]);
    assert_eq!(MAXPLMOVE, 50);
    assert_eq!(SLOWTURNTICS, 6);
    assert_eq!(MAXEVENTS, 64);
}

#[test]
fn vanilla_defaults_and_modern_preset() {
    // m_misc.c:236, 243-253, 269-271
    let v = Controls::vanilla();
    let names = [
        ("key_right", 0xae),
        ("key_left", 0xac),
        ("key_up", 0xad),
        ("key_down", 0xaf),
        ("key_strafeleft", 0x2c),
        ("key_straferight", 0x2e),
        ("key_fire", 0x9d),
        ("key_use", 0x20),
        ("key_strafe", 0xb8),
        ("key_speed", 0xb6),
        ("mouseb_fire", 0),
        ("mouseb_strafe", 1),
        ("mouseb_forward", 2),
        ("mouse_sensitivity", 5),
    ];
    for (n, val) in names {
        assert_eq!(v.get(n), Some(val), "{n}");
    }
    assert_eq!(Controls::default(), v);
    assert_eq!(Preset::default(), Preset::Vanilla);
    assert_eq!(Preset::Vanilla.controls(), v);
    let m = Preset::Modern.controls();
    for (n, val) in [
        ("key_up", 'w' as i32),
        ("key_down", 's' as i32),
        ("key_strafeleft", 'a' as i32),
        ("key_straferight", 'd' as i32),
        ("key_use", 'e' as i32),
        ("mouseb_strafe", -1),
        ("mouseb_forward", -1),
        ("key_right", 0xae),
        ("key_speed", 0xb6),
        ("mouseb_fire", 0),
    ] {
        assert_eq!(m.get(n), Some(val), "{n}");
    }
    let o = Preset::Modern.layer_options();
    assert!(o.always_run && !o.vertical_mouse);
    let o = Preset::Vanilla.layer_options();
    assert!(!o.always_run && o.vertical_mouse);
    assert_eq!(o, Default::default());
}

#[test]
fn controls_validation() {
    let mut c = Controls::vanilla();
    assert_eq!(c.get("joyb_fire"), None);
    assert!(matches!(
        c.set("joyb_fire", 0),
        Err(ControlsError::UnknownName(_))
    ));
    assert!(matches!(
        c.set("key_up", 256),
        Err(ControlsError::KeyOutOfRange { value: 256, .. })
    ));
    assert!(matches!(
        c.set("key_up", -1),
        Err(ControlsError::KeyOutOfRange { .. })
    ));
    assert!(matches!(
        c.set("key_use", 27),
        Err(ControlsError::WithheldKey { value: 27, .. })
    ));
    assert!(matches!(
        c.set("mouseb_fire", 3),
        Err(ControlsError::MouseButtonOutOfRange { .. })
    ));
    assert!(matches!(
        c.set("mouseb_fire", -2),
        Err(ControlsError::MouseButtonOutOfRange { .. })
    ));
    assert_eq!(
        c.set("mouse_sensitivity", 10),
        Err(ControlsError::SensitivityOutOfRange(10))
    );
    assert_eq!(
        c.set("mouse_sensitivity", -1),
        Err(ControlsError::SensitivityOutOfRange(-1))
    );
    assert_eq!(c, Controls::vanilla(), "refusals change nothing");
    c.set("key_up", 255 - 1).unwrap();
    c.set("key_up", 0).unwrap();
    c.set("mouseb_fire", -1).unwrap();
    c.set("mouseb_fire", 2).unwrap();
    c.set("mouse_sensitivity", 0).unwrap();
    c.set("mouse_sensitivity", 9).unwrap();
    assert_eq!(c.mouseb_fire(), MouseBinding::RIGHT);
    assert_eq!(MouseBinding::new(-1), Some(MouseBinding::NONE));
    assert_eq!(MouseBinding::new(1), Some(MouseBinding::MIDDLE));
    assert_eq!(MouseBinding::LEFT.value(), 0);
}

#[test]
fn slow_turn_boundary_and_speeds() {
    // g_game.c:264-275, 298-301: turnheld 1..5 slow (320), from 6 normal or fast
    for (key, sign) in [(KEY_LEFTARROW, 1), (KEY_RIGHTARROW, -1)] {
        for run in [false, true] {
            let mut i = Input::new(Controls::vanilla());
            hold(&mut i, key);
            if run {
                hold(&mut i, KEY_RSHIFT);
            }
            for t in 1..=8 {
                let c = cmd(&mut i);
                let want = if t < 6 {
                    320
                } else if run {
                    1280
                } else {
                    640
                };
                assert_eq!(i32::from(c.angle_turn), sign * want, "tic {t} run {run}");
                assert_eq!(i.state().turnheld(), t);
            }
            i.post(Event::KeyUp(key));
            assert_eq!(cmd(&mut i).angle_turn, 0);
            assert_eq!(i.state().turnheld(), 0);
        }
    }
}

#[test]
fn opposite_turn_keys_count_turnheld_and_cancel() {
    let mut i = Input::new(Controls::vanilla());
    hold(&mut i, KEY_LEFTARROW);
    hold(&mut i, KEY_RIGHTARROW);
    for t in 1..=7 {
        assert_eq!(cmd(&mut i).angle_turn, 0);
        assert_eq!(i.state().turnheld(), t);
    }
}

#[test]
fn strafe_modifier_side_moves_and_clamp() {
    // g_game.c:278-294, 322-325, 417-420: the strafe modifier turns left/right into side moves;
    // the strafe keys add too; side is clamped by MAXPLMOVE (50), not sidemove[1]
    let mut i = Input::new(Controls::vanilla());
    hold(&mut i, KEY_RALT);
    hold(&mut i, KEY_RIGHTARROW);
    let c = cmd(&mut i);
    assert_eq!((c.side, c.angle_turn), (24, 0));
    hold(&mut i, KEY_RSHIFT);
    assert_eq!(cmd(&mut i).side, 40);
    hold(&mut i, b'.');
    assert_eq!(cmd(&mut i).side, 50);
    i.post(Event::KeyUp(KEY_RSHIFT));
    assert_eq!(cmd(&mut i).side, 48);
    i.post(Event::KeyUp(KEY_RIGHTARROW));
    hold(&mut i, KEY_LEFTARROW);
    hold(&mut i, b',');
    i.post(Event::KeyUp(b'.'));
    assert_eq!(cmd(&mut i).side, -48);
    hold(&mut i, KEY_RSHIFT);
    assert_eq!(cmd(&mut i).side, -50);
}

#[test]
fn forward_and_side_keys() {
    for (key, want, run_want) in [
        (KEY_UPARROW, (25, 0), (50, 0)),
        (KEY_DOWNARROW, (-25, 0), (-50, 0)),
        (b'.', (0, 24), (0, 40)),
        (b',', (0, -24), (0, -40)),
    ] {
        let mut i = Input::new(Controls::vanilla());
        hold(&mut i, key);
        let c = cmd(&mut i);
        assert_eq!((i32::from(c.forward), i32::from(c.side)), want);
        hold(&mut i, KEY_RSHIFT);
        let c = cmd(&mut i);
        assert_eq!((i32::from(c.forward), i32::from(c.side)), run_want);
    }
}

#[test]
fn buttons_and_weapon_keys() {
    // g_game.c:330-348: fire 1, use 2, keys '1'..'8' give BT_CHANGE | (n-1)<<3; '9' and '0'
    // nothing; the lowest held digit wins
    let mut i = Input::new(Controls::vanilla());
    hold(&mut i, KEY_RCTRL);
    assert_eq!(cmd(&mut i).buttons, 1);
    i.post(Event::KeyUp(KEY_RCTRL));
    hold(&mut i, b' ');
    assert_eq!(cmd(&mut i).buttons, 2);
    i.post(Event::KeyUp(b' '));
    for (n, ch) in (b'1'..=b'8').enumerate() {
        hold(&mut i, ch);
        assert_eq!(
            cmd(&mut i).buttons,
            4 | ((n as u8) << 3),
            "key {}",
            ch as char
        );
        i.post(Event::KeyUp(ch));
    }
    for ch in *b"90" {
        hold(&mut i, ch);
        assert_eq!(cmd(&mut i).buttons, 0);
        i.post(Event::KeyUp(ch));
    }
    hold(&mut i, b'5');
    hold(&mut i, b'3');
    assert_eq!(cmd(&mut i).buttons, 4 | (2 << 3));
    i.post(Event::KeyUp(b'3'));
    assert_eq!(cmd(&mut i).buttons, 4 | (4 << 3));
}

#[test]
fn sensitivity_scaling_truncates_toward_zero() {
    // g_game.c:579-580; the probe of §1.1.3 fact 19: data2 = -12 gives -6..-16 for 0..9
    let want = [-6, -7, -8, -9, -10, -12, -13, -14, -15, -16];
    for (s, w) in want.iter().enumerate() {
        let mut c = Controls::vanilla();
        c.set("mouse_sensitivity", s as i32).unwrap();
        let mut st = InputState::new();
        assert_eq!(st.respond(&c, mouse(0, -12, 12)), Ok(true));
        assert_eq!(st.mouse_motion(), (*w, -*w), "sensitivity {s}");
    }
}

#[test]
fn mouse_motion_is_assigned_and_used_once() {
    // g_game.c:579-580 assign; g_game.c:411 clears after the build
    let c = Controls::vanilla();
    let mut st = InputState::new();
    st.respond(&c, mouse(0, 40, 0)).unwrap();
    st.respond(&c, mouse(0, -8, 0)).unwrap();
    assert_eq!(st.mouse_motion(), (-8, 0));
    st.respond(&c, mouse(1, 0, 0)).unwrap();
    assert_eq!(st.mouse_motion(), (0, 0));
    st.respond(&c, mouse(0, 20, 12)).unwrap();
    let b = st.build_ticcmd(&c).command();
    assert_eq!((b.forward, b.angle_turn), (12, -160));
    assert_eq!(st.mouse_motion(), (0, 0));
    assert_eq!(st.build_ticcmd(&c).command(), TicCommand::default());
}

#[test]
fn mouse_buttons_store_raw_values() {
    // g_game.c:576-578: data1 & 1, & 2, & 4; mousearray[0] is never written
    let c = Controls::vanilla();
    let mut st = InputState::new();
    st.respond(&c, mouse(0xff, 0, 0)).unwrap();
    assert_eq!(st.mouse_array(), [0, 1, 2, 4]);
    st.respond(&c, mouse(0xf8, 0, 0)).unwrap();
    assert_eq!(st.mouse_array(), [0, 0, 0, 0]);
}

#[test]
fn mouse_terms_and_clamps() {
    // g_game.c:351-352, 405-420
    let c = Controls::vanilla();
    let mut st = InputState::new();
    st.respond(&c, mouse(4, 0, 0)).unwrap();
    assert_eq!(st.build_ticcmd(&c).command().forward, 25);
    st.respond(&c, mouse(4, 0, 26)).unwrap();
    assert_eq!(st.build_ticcmd(&c).command().forward, 50);
    st.respond(&c, mouse(0, 0, -51)).unwrap();
    assert_eq!(st.build_ticcmd(&c).command().forward, -50);
    st.respond(&c, mouse(0, 0, 50)).unwrap();
    assert_eq!(st.build_ticcmd(&c).command().forward, 50);
    // strafe button: side += mousex*2, clamped to 50
    st.respond(&c, mouse(2, 25, 0)).unwrap();
    let b = st.build_ticcmd(&c).command();
    assert_eq!((b.side, b.angle_turn), (50, 0));
    st.respond(&c, mouse(2, -13, 0)).unwrap();
    assert_eq!(st.build_ticcmd(&c).command().side, -26);
    st.respond(&c, mouse(2, -26, 0)).unwrap();
    assert_eq!(st.build_ticcmd(&c).command().side, -50);
    // turn: angleturn -= mousex*8, narrowed to short
    st.respond(&c, mouse(0, 4095, 0)).unwrap();
    assert_eq!(st.build_ticcmd(&c).command().angle_turn, -32760);
    st.respond(&c, mouse(0, -4096, 0)).unwrap();
    assert_eq!(st.build_ticcmd(&c).command().angle_turn, -32768);
    st.respond(&c, mouse(0, 8192, 0)).unwrap();
    assert_eq!(st.build_ticcmd(&c).command().angle_turn, 0);
}

#[test]
fn mouse_overflow_is_refused_without_change() {
    // g_game.c:579-580: data2*(mouseSensitivity+5) must fit int
    let mut c = Controls::vanilla();
    let mut st = InputState::new();
    let before = st.clone();
    assert_eq!(
        st.respond(&c, mouse(1, 214_748_365, 0)),
        Err(InputError::MouseOverflow)
    );
    assert_eq!(
        st.respond(&c, mouse(1, 0, -214_748_365)),
        Err(InputError::MouseOverflow)
    );
    assert_eq!(st, before);
    assert_eq!(
        st.respond(&c, mouse(0, 214_748_364, -214_748_364)),
        Ok(true)
    );
    assert_eq!(st.mouse_motion(), (214_748_364, -214_748_364));
    c.set("mouse_sensitivity", 9).unwrap();
    assert_eq!(st.respond(&c, mouse(0, 153_391_689, 0)), Ok(true));
    assert_eq!(
        st.respond(&c, mouse(0, 153_391_690, 0)),
        Err(InputError::MouseOverflow)
    );
    // the largest motions still build: turn, strafe and forward stay inside int
    for (b, dx, dy) in [
        (0, 214_748_364, 0),
        (0, -214_748_364, 0),
        (2, 214_748_364, 0),
        (2, -214_748_364, 214_748_364),
    ] {
        let mut st = InputState::new();
        st.respond(&Controls::vanilla(), mouse(b, dx, dy)).unwrap();
        let cmd = st.build_ticcmd(&Controls::vanilla()).command();
        assert!(common::wire24_admits(&cmd), "{cmd:?}");
    }
    // the witnesses beyond the app's event domain (tests/reference/m5b_input/witness): the
    // genuine overflow build passes ev_mouse_int_edge and stops on ev_mouse_int_overflow
    // (tools/reference/m5b_input_reference.py); Rust passes the first and refuses the second
    for (name, want) in [
        ("ev_mouse_int_edge", None),
        ("ev_mouse_int_overflow", Some(InputError::MouseOverflow)),
    ] {
        let s = common::read_script_in("witness", name);
        let mut i = Input::new(Controls::vanilla());
        let mut refused = None;
        for d in s.directives {
            match d {
                common::Directive::Post(e) => i.post(e),
                common::Directive::Tic(_) => {
                    if let Err(e) = i.tic() {
                        refused = Some(e);
                        break;
                    }
                }
                _ => {}
            }
        }
        assert_eq!(refused, want, "{name}");
    }
}

#[test]
fn withheld_keydowns_are_refused_keyups_accepted() {
    let c = Controls::vanilla();
    let mut st = InputState::new();
    for k in keys::WITHHELD {
        assert_eq!(
            st.respond(&c, Event::KeyDown(k)),
            Err(InputError::WithheldKey(k))
        );
        assert_eq!(st.respond(&c, Event::KeyUp(k)), Ok(false));
    }
    assert_eq!(st, InputState::new());
    assert_eq!(keys::WITHHELD.len(), 19);
    assert!(
        keys::is_withheld(KEY_PAUSE) && keys::is_withheld(b'q') && keys::is_withheld(KEY_F[11])
    );
    assert!(!keys::is_withheld(KEY_BACKSPACE) && !keys::is_withheld(b'w'));
    // a refused event stops the walk and is dropped; the events after it come next
    let mut i = Input::new(c);
    i.post(Event::KeyDown(b'5'));
    i.post(Event::KeyDown(KEY_PAUSE));
    i.post(Event::KeyDown(b'6'));
    assert_eq!(i.process_events(), Err(InputError::WithheldKey(KEY_PAUSE)));
    assert!(i.state().key_down(b'5') && !i.state().key_down(b'6'));
    assert_eq!(i.process_events(), Ok(()));
    assert!(i.state().key_down(b'6'));
}

#[test]
fn keydown_eaten_keyup_not() {
    // g_game.c:568, 573, 581
    let c = Controls::vanilla();
    let mut st = InputState::new();
    assert_eq!(st.respond(&c, Event::KeyDown(b'a')), Ok(true));
    assert!(st.key_down(b'a'));
    assert_eq!(st.respond(&c, Event::KeyUp(b'a')), Ok(false));
    assert!(!st.key_down(b'a'));
    assert_eq!(st.respond(&c, mouse(0, 0, 0)), Ok(true));
}

/// Press the forward mouse button for `press` tics after an idle lead-in, release for `gap`,
/// press again for 3; returns whether any tic carried BT_USE.
fn double_click(button: u8, press: usize, gap: usize) -> bool {
    let c = Controls::vanilla();
    let mut st = InputState::new();
    let mut used = false;
    let mut run = |st: &mut InputState, b: u8, n: usize| {
        st.respond(&c, mouse(b, 0, 0)).unwrap();
        for _ in 0..n {
            used |= st.build_ticcmd(&c).command().buttons & 2 != 0;
        }
    };
    run(&mut st, 0, 30);
    run(&mut st, button, press);
    run(&mut st, 0, gap);
    run(&mut st, button, 3);
    run(&mut st, 0, 3);
    used
}

#[test]
fn double_click_window() {
    // g_game.c:354-403; observed in the genuine sweep (M5b investigation §2.2.4): BT_USE when
    // P + G <= 24, never for P = 1 with G <= 2 or P = 2 with G = 1
    for button in [4, 2] {
        for p in 1..=3usize {
            for g in 1..=26usize {
                let want = p + g <= 24 && !(p == 1 && g <= 2) && !(p == 2 && g == 1);
                assert_eq!(
                    double_click(button, p, g),
                    want,
                    "button {button} P {p} G {g}"
                );
            }
        }
    }
}

#[test]
fn double_click_state_details() {
    let c = Controls::vanilla();
    let mut st = InputState::new();
    // dclicktime starts at 0: a press at tic 0 is not registered (dclick_at_start)
    st.respond(&c, mouse(4, 0, 0)).unwrap();
    st.build_ticcmd(&c);
    assert_eq!(st.double_click().state, 0);
    assert_eq!(st.double_click().time, 1);
    st.build_ticcmd(&c);
    st.build_ticcmd(&c);
    // registered at time 2 > 1: state holds the raw value 4, clicks 1, time reset
    let d = st.double_click();
    assert_eq!((d.time, d.state, d.clicks), (0, 4, 1));
    // the strafe machine stores 0/1
    let mut st2 = InputState::new();
    st2.respond(&c, mouse(2, 0, 0)).unwrap();
    for _ in 0..3 {
        st2.build_ticcmd(&c);
    }
    let d = st2.strafe_double_click();
    assert_eq!((d.time, d.state, d.clicks), (0, 1, 1));
    // held past 20 tics: the timeout forces state 0 and clicks 0 while held
    for _ in 0..21 {
        st.build_ticcmd(&c);
    }
    let d = st.double_click();
    assert_eq!((d.time, d.state, d.clicks), (21, 0, 0));
    // the use key clears dclicks only (g_game.c:334-339)
    let mut st = InputState::new();
    st.respond(&c, mouse(4, 0, 0)).unwrap();
    for _ in 0..3 {
        st.build_ticcmd(&c);
    }
    st.respond(&c, Event::KeyDown(b' ')).unwrap();
    st.build_ticcmd(&c);
    let d = st.double_click();
    assert_eq!((d.state, d.clicks), (4, 0));
}

#[test]
fn level_load_clears_keys_motion_and_left_button_only() {
    // g_game.c:490-496 with the declared sizeof(boolean) adapter
    let c = Controls::vanilla();
    let mut st = InputState::new();
    st.respond(&c, Event::KeyDown(KEY_LEFTARROW)).unwrap();
    st.respond(&c, mouse(7, 5, 5)).unwrap();
    st.build_ticcmd(&c);
    st.respond(&c, mouse(7, 5, 5)).unwrap();
    let (th, d, d2) = (st.turnheld(), st.double_click(), st.strafe_double_click());
    st.load_level();
    assert_eq!(st.keys_down().count(), 0);
    assert_eq!(st.mouse_motion(), (0, 0));
    assert_eq!(st.mouse_array(), [0, 0, 2, 4]);
    assert_eq!(
        (st.turnheld(), st.double_click(), st.strafe_double_click()),
        (th, d, d2)
    );
}

#[test]
fn demo_round_trip_table() {
    // g_game.c:1510-1513, 1499-1502: byte = floor((a+128)/256) mod 256; re-read = byte<<8 as short
    for a in i16::MIN..=i16::MAX {
        let c = TicCommand {
            forward: -50,
            side: 50,
            angle_turn: a,
            buttons: 0x3f,
        };
        let bytes = write_ticcmd_bytes(&c);
        let hi = (i32::from(a) + 128).div_euclid(256);
        assert_eq!(bytes, [0xce, 0x32, hi.rem_euclid(256) as u8, 0x3f], "{a}");
        let back = read_ticcmd_bytes(bytes).unwrap();
        let want = (hi * 256 + 32768).rem_euclid(65536) - 32768;
        assert_eq!(i32::from(back.angle_turn), want, "{a}");
        assert_eq!((back.forward, back.side, back.buttons), (-50, 50, 0x3f));
    }
    // edges: -128..127 -> 0, 128 -> 256 (an exact half rounds up), -129 -> -256,
    // 32640.. -> -32768 (wrap), 320 -> 256, 640 -> 768, -640 -> -512
    for (a, x) in [
        (127, 0),
        (-128, 0),
        (128, 256),
        (-129, -256),
        (32639, 32512),
        (32640, -32768),
        (32767, -32768),
        (-32768, -32768),
        (320, 256),
        (640, 768),
        (-640, -512),
        (1280, 1280),
        (-320, -256),
    ] {
        let c = TicCommand {
            angle_turn: a,
            ..Default::default()
        };
        assert_eq!(
            read_ticcmd_bytes(write_ticcmd_bytes(&c))
                .unwrap()
                .angle_turn,
            x,
            "{a}"
        );
    }
    // DEMOMARKER (g_game.c:1488, 1493)
    assert_eq!(read_ticcmd_bytes([0x80, 0, 0, 0]), None);
    assert_eq!(
        read_ticcmd_bytes([0x7f, 0x80, 0x80, 0x80]).unwrap().forward,
        127
    );
    assert_eq!(
        read_ticcmd_bytes([0x81, 0x80, 0x80, 0x80]).unwrap().side,
        -128
    );
}

#[test]
fn input_tic_records_the_built_command() {
    let mut i = Input::new(Controls::vanilla());
    hold(&mut i, KEY_LEFTARROW);
    let r = i.tic().unwrap();
    assert_eq!(r.built().angle_turn, 320);
    assert_eq!(r.demo(), [0, 0, 1, 0]);
    assert_eq!(r.executed().angle_turn, 256);
    assert_eq!(i.controls(), &Controls::vanilla());
    i.load_level();
    assert_eq!(i.state().keys_down().count(), 0);
}

#[test]
fn a_refused_event_is_dropped_and_the_events_after_it_are_processed_next() {
    let mut input = Input::new(Controls::vanilla());
    input.post(Event::KeyDown(keys::KEY_ESCAPE));
    input.post(Event::KeyDown(keys::KEY_UPARROW));
    let before = input.state().clone();
    assert_eq!(input.tic(), Err(InputError::WithheldKey(keys::KEY_ESCAPE)));
    assert_eq!(input.state(), &before, "the refusal changed nothing");
    assert_eq!(input.tic().expect("not wedged").executed().forward, 25);
    assert_eq!(input.tic().expect("held").executed().forward, 25);
}
