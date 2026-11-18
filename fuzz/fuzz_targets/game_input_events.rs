#![no_main]
//! The input code of `doom-game` (M5b): the app's `EventLayer`, the core `Event` path
//! (`InputState::respond` and `build_ticcmd`) and the demo round trip, on arbitrary bindings
//! and event streams.
//!
//! Input: a two-byte header, then `data[1] % 4` five-byte `Controls::set` records, then
//! five-byte operations. Header byte 0: bit 0 picks the modern preset (else vanilla), bits 1-2
//! flip the preset's `always_run` and `vertical_mouse`; byte 1 is the set count. A set record is
//! a name index (`% 15`: the 14 accepted names, then one unknown name) and an `i32` value in
//! four little-endian bytes (refusals must be clean errors that change nothing). An operation is an opcode
//! (`% 11`) and four bytes `a b c d`:
//!
//! - layer path (events through `EventLayer::drain` into `Input`): 0 key down `a`, 1 key up `a`
//!   (any code, withheld ones included), 2 mouse motion `a as i8 << (c % 24)`, `b as i8 << (d % 24)`,
//!   3 mouse button `a % 3` down if `b` odd, 4 `release_all`, 5 drain, post and build a tic;
//! - core path (events straight to an `InputState`): 6 `Event::KeyDown(a)`, 7 `Event::KeyUp(a)`,
//!   8 `Event::Mouse { buttons: a, dx: b as i8 << (c % 25), dy: ... d }` (up to +-2^31, so the
//!   `MouseOverflow` refusal is reachable), 9 build a tic, 10 `load_level` on both paths.
//!
//! Properties: no panic or overflow; one drain never exceeds `MAX_EVENTS_PER_BUILD` events and
//! the layer's events never contain a withheld keydown, and the `Input` never refuses one
//! (nor a mouse event); every built command is admitted by wire 24 (the predicate of
//! `crates/doom-game/tests/common/mod.rs`, restated here), its `demo` bytes are
//! `write_ticcmd_bytes(built)`, its `executed` is `read_ticcmd_bytes(demo)` with a zero low byte
//! of `angle_turn`, and forward, side and buttons survive the round trip; a refused core event
//! (`WithheldKey` only for withheld keydowns, `MouseOverflow` only for an overflowing scaling)
//! leaves the state equal to a copy taken before, accessor by accessor; running the same bytes
//! twice from fresh state gives the same commands and final states.
//! `-max_len=1017` (2 + 3 x 5 + 200 x 5 bytes at most are used; the rest is ignored).
use doom_game::input::{
    Event, EventLayer, Input, InputError, InputState, MAX_EVENTS_PER_BUILD, MouseButton, Preset,
    RecordedTic, keys, read_ticcmd_bytes, write_ticcmd_bytes,
};
use doom_sim::{BT_ATTACK, BT_CHANGE, BT_USE, BT_WEAPONMASK, TicCommand};
use libfuzzer_sys::fuzz_target;

const NAMES: [&str; 15] = [
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
    "mouseb_fire",
    "mouseb_strafe",
    "mouseb_forward",
    "mouse_sensitivity",
    "joyb_fire",
];
const MAX_OPS: usize = 200;

fn wire24_admits(c: &TicCommand) -> bool {
    c.buttons & !(BT_ATTACK | BT_USE | BT_CHANGE | BT_WEAPONMASK) == 0
        && (-127..=127).contains(&c.forward)
        && (-127..=127).contains(&c.side)
        && (c.buttons & BT_CHANGE != 0 || c.buttons & BT_WEAPONMASK == 0)
}

fn check_tic(r: &RecordedTic) {
    assert!(
        wire24_admits(&r.built()) && wire24_admits(&r.executed()),
        "{r:?}"
    );
    assert_eq!(r.demo(), write_ticcmd_bytes(&r.built()));
    assert_eq!(Some(r.executed()), read_ticcmd_bytes(r.demo()));
    assert_eq!(r.executed().angle_turn & 0xff, 0);
    assert_eq!(
        (r.built().forward, r.built().side, r.built().buttons),
        (
            r.executed().forward,
            r.executed().side,
            r.executed().buttons
        )
    );
}

/// The accessors of an `InputState`, to compare beside its `Eq`.
type Snapshot = (Vec<u8>, [i32; 4], (i32, i32), i32, [i32; 6]);
fn snapshot(s: &InputState) -> Snapshot {
    let (d, e) = (s.double_click(), s.strafe_double_click());
    (
        s.keys_down().collect(),
        s.mouse_array(),
        s.mouse_motion(),
        s.turnheld(),
        [d.time, d.state, d.clicks, e.time, e.state, e.clicks],
    )
}

/// What a run produces: every executed demo record and the final states.
type Outcome = (Vec<[u8; 4]>, Input, InputState, EventLayer);

fn run(data: &[u8]) -> Outcome {
    let preset = if data[0] & 1 != 0 {
        Preset::Modern
    } else {
        Preset::Vanilla
    };
    let mut options = preset.layer_options();
    options.always_run ^= data[0] & 2 != 0;
    options.vertical_mouse ^= data[0] & 4 != 0;
    let mut controls = preset.controls();
    let sets = usize::from(data[1] % 4);
    let mut at = 2;
    for _ in 0..sets {
        let Some(r) = data.get(at..at + 5) else { break };
        at += 5;
        let name = NAMES[usize::from(r[0]) % NAMES.len()];
        let value = i32::from_le_bytes([r[1], r[2], r[3], r[4]]);
        let before = controls;
        match controls.set(name, value) {
            Ok(()) => assert_eq!(controls.get(name), Some(value)),
            Err(_) => assert_eq!(controls, before, "a refused set changed {name}"),
        }
    }
    let mut layer = EventLayer::new(&controls, options);
    let mut input = Input::new(controls);
    let mut core = InputState::new();
    let mut trace = Vec::new();
    let mut events = Vec::new();
    for op in data[at..].chunks_exact(5).take(MAX_OPS) {
        let (a, b, c, d) = (op[1], op[2], op[3], op[4]);
        let shl = |v: u8, n: u8, m: u8| i32::from(v as i8) << (n % m);
        match op[0] % 11 {
            0 => {
                layer.key(a, true);
            }
            1 => {
                layer.key(a, false);
            }
            2 => layer.mouse_motion(shl(a, c, 24), shl(b, d, 24)),
            3 => {
                let button = [MouseButton::Left, MouseButton::Middle, MouseButton::Right];
                layer.mouse_button(button[usize::from(a) % 3], b & 1 != 0);
            }
            4 => layer.release_all(),
            5 => {
                events.clear();
                layer.drain(&mut events);
                assert!(
                    events.len() <= MAX_EVENTS_PER_BUILD,
                    "{} events",
                    events.len()
                );
                for e in &events {
                    assert!(!matches!(e, Event::KeyDown(k) if keys::is_withheld(*k)));
                    input.post(*e);
                }
                let r = input.tic().expect("the layer's events are never refused");
                check_tic(&r);
                trace.push(r.demo());
            }
            6 | 7 | 8 => {
                let event = match op[0] % 11 {
                    6 => Event::KeyDown(a),
                    7 => Event::KeyUp(a),
                    _ => Event::Mouse {
                        buttons: a,
                        dx: shl(b, c, 25),
                        dy: shl(d, c ^ d, 25),
                    },
                };
                let (before, copy) = (snapshot(&core), core.clone());
                match core.respond(&controls, event) {
                    Ok(_) => {}
                    Err(InputError::WithheldKey(k)) => {
                        assert_eq!(event, Event::KeyDown(k));
                        assert!(keys::is_withheld(k));
                        assert_eq!((core.clone(), snapshot(&core)), (copy, before));
                    }
                    Err(InputError::MouseOverflow) => {
                        let Event::Mouse { dx, dy, .. } = event else {
                            panic!("{event:?}")
                        };
                        let f = controls.mouse_sensitivity() + 5;
                        assert!(dx.checked_mul(f).is_none() || dy.checked_mul(f).is_none());
                        assert_eq!((core.clone(), snapshot(&core)), (copy, before));
                    }
                }
            }
            9 => {
                let r = core.build_ticcmd(&controls).record();
                check_tic(&r);
                trace.push(r.demo());
            }
            _ => {
                core.load_level();
                input.load_level();
            }
        }
    }
    (trace, input, core, layer)
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let first = run(data);
    let second = run(data);
    assert!(
        first == second,
        "the same bytes gave different commands or states"
    );
});
