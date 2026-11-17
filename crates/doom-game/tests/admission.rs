//! Every command the input path builds is admitted by wire 24, for the goldens' scripts and for
//! deterministic pseudo-random event streams, through the core directly and through the app's
//! layer. The executed command is also accepted by the simulation's own demo writer
//! (`doom_sim::demo::write_demo_episode1_v2`, which refuses −128 moves, buttons 0xc0 and a
//! nonzero `angle_turn` low byte).

mod common;

use common::{Directive, wire24_admits};
use doom_game::input::{
    Controls, Event, EventLayer, Input, InputError, InputState, MouseButton, Preset, RecordedTic,
    keys,
};
use doom_sim::demo::write_demo_episode1_v2;

/// A small deterministic generator (xorshift64*), so the streams are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo + 1) as u64) as i32
    }
}

fn check(r: &RecordedTic, all: &mut Vec<doom_sim::TicCommand>) {
    assert!(wire24_admits(&r.built), "built {:?}", r.built);
    assert!(wire24_admits(&r.executed), "executed {:?}", r.executed);
    assert_eq!(r.executed.angle_turn & 0xff, 0);
    assert!(r.built.buttons & 0x80 == 0 && r.built.forward != -128 && r.built.side != -128);
    all.push(r.executed);
}

fn flush(all: &mut Vec<doom_sim::TicCommand>) {
    write_demo_episode1_v2(1, 2, all).expect("the demo writer accepts every executed command");
    all.clear();
}

#[test]
fn every_script_command_is_admitted() {
    // every event-level script, the x_ and s5_ ones included up to their first X directive; a
    // refused event (a withheld keydown) changes nothing and is skipped
    let (mut n, mut refused) = (0, 0);
    let mut all = Vec::new();
    for name in common::script_names() {
        let s = common::read_script(&name);
        let mut controls = Controls::vanilla();
        for (k, v) in &s.config {
            controls.set(k, *v).unwrap();
        }
        let mut st = InputState::new();
        'script: for d in s.directives {
            match d {
                Directive::Post(e) => {
                    if st.respond(&controls, e).is_err() {
                        refused += 1;
                    }
                }
                Directive::Tic(t) => {
                    for _ in 0..t {
                        check(&st.build_ticcmd(&controls).record(), &mut all);
                        n += 1;
                    }
                }
                Directive::LoadLevel => st.load_level(),
                Directive::EndDemo | Directive::XLevel => break 'script,
            }
        }
        flush(&mut all);
    }
    eprintln!("script commands admitted: {n}; refused events: {refused}");
    assert!(n > 35_861 && refused > 0);
}

#[test]
fn random_events_through_the_core_are_admitted() {
    let mut rng = Rng(0x5eed_1234_abcd_0001);
    let presets = [Preset::Vanilla.controls(), Preset::Modern.controls()];
    let (mut built, mut refused) = (0u32, 0u32);
    let mut all = Vec::new();
    for run in 0..200 {
        let mut controls = presets[run % 2];
        controls
            .set("mouse_sensitivity", (run % 10) as i32)
            .unwrap();
        let mut st = InputState::new();
        for _tic in 0..300 {
            for _ in 0..rng.below(6) {
                let e = match rng.below(10) {
                    0..=3 => Event::KeyDown(rng.below(256) as u8),
                    4..=6 => Event::KeyUp(rng.below(256) as u8),
                    7 => Event::Mouse {
                        buttons: rng.below(256) as u8,
                        dx: rng.next() as i32,
                        dy: rng.next() as i32,
                    },
                    _ => Event::Mouse {
                        buttons: rng.below(8) as u8,
                        dx: rng.range(-1276, 1276),
                        dy: rng.range(-1276, 1276),
                    },
                };
                let before = st.clone();
                match st.respond(&controls, e) {
                    Ok(_) => {}
                    Err(InputError::WithheldKey(_) | InputError::MouseOverflow) => {
                        assert_eq!(st, before, "a refusal changes nothing");
                        refused += 1;
                    }
                }
            }
            if rng.below(97) == 0 {
                st.load_level();
            }
            check(&st.build_ticcmd(&controls).record(), &mut all);
            built += 1;
        }
        flush(&mut all);
    }
    eprintln!("random core commands admitted: {built}; refused events: {refused}");
    assert!(refused > 0 && built == 60_000);
}

#[test]
fn random_platform_input_through_the_layer_is_admitted_and_never_refused() {
    let mut rng = Rng(0x0dd_ba11_cafe_f00d);
    let mut built = 0u32;
    let mut all = Vec::new();
    let mut events = Vec::new();
    for run in 0..200 {
        let preset = if run % 2 == 0 {
            Preset::Vanilla
        } else {
            Preset::Modern
        };
        let controls = preset.controls();
        let mut layer = EventLayer::new(&controls, preset.layer_options());
        let mut input = Input::new(controls);
        for _tic in 0..300 {
            for _ in 0..rng.below(8) {
                match rng.below(12) {
                    0..=4 => {
                        layer.key(rng.below(256) as u8, rng.below(2) == 0);
                    }
                    5..=7 => layer.mouse_motion(rng.next() as i32, rng.next() as i32),
                    8 | 9 => {
                        let b = [MouseButton::Left, MouseButton::Middle, MouseButton::Right]
                            [rng.below(3) as usize];
                        layer.mouse_button(b, rng.below(2) == 0);
                    }
                    10 => layer.mouse_motion(rng.range(-5, 5), rng.range(-5, 5)),
                    _ => {
                        if rng.below(20) == 0 {
                            layer.release_all();
                        }
                    }
                }
            }
            if rng.below(97) == 0 {
                input.load_level();
            }
            events.clear();
            layer.drain(&mut events);
            assert!(events.len() < 64);
            for e in &events {
                if let Event::KeyDown(k) | Event::KeyUp(k) = e {
                    assert!(!keys::is_withheld(*k));
                }
                input.post(*e);
            }
            let r = input.tic().expect("the layer's stream is never refused");
            check(&r, &mut all);
            built += 1;
        }
        flush(&mut all);
    }
    eprintln!("random layer commands admitted: {built}");
    assert_eq!(built, 60_000);
}
