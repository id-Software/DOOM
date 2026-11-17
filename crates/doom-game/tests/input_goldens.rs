//! The Rust input path against the genuine goldens of tests/reference/m5b_input, tic for tic.
//!
//! Source equivalence: the goldens were written by linuxdoom-1.10's own `D_ProcessEvents`,
//! `G_Responder`, `G_BuildTiccmd` and `G_Ticker` (tools/reference/m5b_input_driver.c, built by
//! tools/reference/m5b_input_build.py with its declared adapters). Every app-domain script is
//! replayed from its event script through [`Input`] (ring, responder, build, round trip); the
//! `x_` and `s5_` scripts are replayed at the event level from the events the golden records as
//! delivered to `G_Responder`'s switch.

mod common;

use common::{Directive, Row, TRow, fields, read_golden, read_script, wire24_admits};
use doom_game::input::{Controls, Event, Input, InputError, RecordedTic};

/// `G_RecordDemo`'s `maxsize = 0x20000` (g_game.c:1538) and the refusal of the tic whose
/// offset `13+4k` exceeds `maxsize-16` (g_game.c:1515-1520): tics 0..32760 are recorded and
/// tic 32761 ends the demo (M5b investigation §1.2.2 fact 7).
const DEMO_TIC_LIMIT: i64 = (0x20000 - 29) / 4 + 1;
/// The 13 header bytes of `G_BeginRecording` (g_game.c:1555-1566).
const DEMO_HEADER: i64 = 13;
/// `G_CheckDemoStatus`' `I_Error` text for the driver's demo name (g_game.c:1683).
const DEMO_RECORDED: &str = "I_Error Demo m5binput.lmp recorded";

#[derive(Default, Debug)]
struct Coverage {
    scripts: usize,
    tics: usize,
    fields: usize,
}

fn controls_of(config: &[(String, i32)]) -> Controls {
    let mut c = Controls::vanilla();
    for (n, v) in config {
        c.set(n, *v)
            .unwrap_or_else(|e| panic!("config set {n} {v}: {e}"));
    }
    c
}

fn t_row(input: &Input, rec: &RecordedTic, record: bool) -> TRow {
    let s = input.state();
    let (executed, demo) = if record {
        (
            rec.executed,
            rec.demo
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        )
    } else {
        (rec.built, "-".to_owned())
    };
    let keys: Vec<String> = s.keys_down().map(|k| format!("{k:x}")).collect();
    let d = s.double_click();
    let d2 = s.strafe_double_click();
    TRow {
        built: fields(&rec.built),
        executed: fields(&executed),
        demo,
        // the driver caps turnheld at SLOWTURNTICS and the timers at 21 (m5b_input_driver.c, dump_tic)
        turnheld: s.turnheld().min(6),
        dclick: [d.time.min(21), d.state, d.clicks],
        dclick2: [d2.time.min(21), d2.state, d2.clicks],
        mb: s
            .mouse_array()
            .iter()
            .enumerate()
            .map(|(i, v)| i32::from(*v != 0) << i)
            .sum(),
        keys: if keys.is_empty() {
            "-".to_owned()
        } else {
            keys.join(",")
        },
    }
}

/// The event as the golden prints it: `TYPE D1 D2 D3` (key events with `-`).
fn e_fields(e: &Event) -> (i64, i64, Option<i64>, Option<i64>) {
    match *e {
        Event::KeyDown(k) => (0, i64::from(k), None, None),
        Event::KeyUp(k) => (1, i64::from(k), None, None),
        Event::Mouse { buttons, dx, dy } => (
            2,
            i64::from(buttons),
            Some(i64::from(dx)),
            Some(i64::from(dy)),
        ),
    }
}

/// Replays an app-domain (or `beyond_app_`) script through [`Input`] and requires every golden
/// row. Returns the number of tics and compared fields.
fn run_script(name: &str) -> (usize, usize) {
    let script = read_script(name);
    let golden = read_golden(name);
    let mut rows = golden.iter().peekable();
    let mut cmp = 0usize;
    // h and s rows before the first tic. The start state is the only s row of every app-domain
    // golden; its input-owned fields (sendpause, paused) stay 0 because nothing here sets them.
    let mut s_rows = 0;
    while let Some(r) = rows.peek() {
        match r {
            Row::H(hex) => assert_eq!(hex.len(), 26, "{name}: header"),
            Row::S(tic, state) => {
                assert_eq!(*tic, -1, "{name}: the state changed");
                assert!(state.contains(",paused=0,sendpause=0,"), "{name}: {state}");
                s_rows += 1;
                cmp += 1;
            }
            _ => break,
        }
        rows.next();
    }
    assert_eq!(s_rows, 1, "{name}: one start state row");
    let mut input = Input::new(controls_of(&script.config));
    let mut tic: i64 = 0;
    let mut posted = 0i64;
    let mut ended = false;
    'directives: for d in &script.directives {
        match d {
            Directive::Post(e) => {
                input.post(*e);
                posted += 1;
            }
            Directive::LoadLevel => {
                input.load_level();
                assert_eq!(
                    rows.next(),
                    Some(&Row::L(tic)),
                    "{name}: loadlevel before {tic}"
                );
                cmp += 1;
            }
            Directive::EndDemo => {
                assert_eq!(
                    rows.next(),
                    Some(&Row::X(tic, DEMO_RECORDED.to_owned())),
                    "{name}: enddemo"
                );
                cmp += 2;
                ended = true;
                break;
            }
            Directive::XLevel => panic!("{name}: X-level directive in an event-level script"),
            Directive::Tic(n) => {
                for _ in 0..*n {
                    if script.record && tic == DEMO_TIC_LIMIT {
                        assert_eq!(
                            rows.next(),
                            Some(&Row::X(tic, DEMO_RECORDED.to_owned())),
                            "{name}: demo limit"
                        );
                        cmp += 2;
                        ended = true;
                        break 'directives;
                    }
                    let mut delivered = Vec::new();
                    input
                        .process_events_with(|e| delivered.push(*e))
                        .unwrap_or_else(|e| panic!("{name} tic {tic}: {e}"));
                    for e in &delivered {
                        let (ty, d1, d2, d3) = e_fields(e);
                        let want = Row::E {
                            tic,
                            ty,
                            d1,
                            d2,
                            d3,
                            who: "G".to_owned(),
                        };
                        assert_eq!(rows.next(), Some(&want), "{name}: event in tic {tic}");
                        cmp += 5;
                    }
                    if posted != delivered.len() as i64 {
                        let want = Row::Q(tic, posted, delivered.len() as i64);
                        assert_eq!(rows.next(), Some(&want), "{name}: lost events in tic {tic}");
                        cmp += 2;
                    }
                    posted = 0;
                    let rec = input.build_ticcmd().record();
                    let want = Row::T(tic, t_row(&input, &rec, script.record));
                    assert_eq!(rows.next(), Some(&want), "{name}: tic {tic}");
                    cmp += 18;
                    assert!(wire24_admits(&rec.built) && wire24_admits(&rec.executed));
                    tic += 1;
                }
            }
        }
    }
    if !ended {
        let len = if script.record {
            DEMO_HEADER + 4 * tic
        } else {
            -1
        };
        assert_eq!(rows.next(), Some(&Row::Z(tic, len)), "{name}: end");
        cmp += 2;
    }
    assert_eq!(rows.next(), None, "{name}: rows left over");
    (tic as usize, cmp)
}

#[test]
fn app_domain_scripts_match_the_genuine_goldens() {
    let names: Vec<String> = common::script_names()
        .into_iter()
        .filter(|n| common::is_app_domain(n))
        .collect();
    let mut cov = Coverage::default();
    for name in &names {
        let (tics, f) = run_script(name);
        cov.scripts += 1;
        cov.tics += tics;
        cov.fields += f;
    }
    eprintln!("app domain: {cov:?}");
    // the counts of tools/reference/m5b_input_scripts.py's app domain
    assert_eq!((cov.scripts, cov.tics), (175, 35_861));
}

#[test]
fn beyond_app_script_matches_including_the_short_wrap() {
    // data2 up to ±262,140, beyond the app's clamp of ±1276 but inside int: the short narrowing
    // of g_game.c:409 and the re-read of a built turn of 32640 or more as -32768
    // (g_game.c:1512, 1501). The core is exact here too.
    let (tics, f) = run_script("beyond_app_mouse_short_edge");
    eprintln!("beyond_app_mouse_short_edge: {tics} tics, {f} fields");
    assert_eq!(tics, 13);
}

/// The outcome of an event-level replay of an `x_` or `s5_` golden.
#[derive(Debug, Eq, PartialEq)]
enum Replay {
    Exact { tics: usize, fields: usize },
    Refused(InputError),
    Differs { tic: i64 },
}

/// Replays a golden at the event level: every event the golden shows delivered to
/// `G_Responder`'s switch (`WHO` = `G`) is posted, the others were eaten by a responder ahead of
/// it; `L` rows load the level; each `t` row is built and compared. Key events with
/// `data1 >= NUMKEYS` (raw X keysyms) are skipped: the switch ignores them (g_game.c:566, 571).
/// Mouse `data1` is passed modulo 256: `G_Responder` reads only bits 0-2 (g_game.c:576-578).
fn replay(name: &str) -> Replay {
    let script = read_script(name);
    let mut input = Input::new(controls_of(&script.config));
    let (mut tics, mut cmp) = (0, 0);
    for row in read_golden(name) {
        match row {
            Row::E {
                ty,
                d1,
                d2,
                d3,
                who,
                ..
            } if who == "G" => {
                let e = match ty {
                    0 | 1 if d1 >= 256 => continue,
                    0 => Event::KeyDown(d1 as u8),
                    1 => Event::KeyUp(d1 as u8),
                    2 => Event::Mouse {
                        buttons: (d1 & 0xff) as u8,
                        dx: d2.expect("dx") as i32,
                        dy: d3.expect("dy") as i32,
                    },
                    _ => panic!("{name}: event type {ty}"),
                };
                input.post(e);
            }
            Row::L(_) => input.load_level(),
            // the run stopped in this tic after its events were delivered
            Row::X(..) => {
                if let Err(e) = input.process_events() {
                    return Replay::Refused(e);
                }
            }
            Row::T(tic, want) => {
                if let Err(e) = input.process_events() {
                    return Replay::Refused(e);
                }
                let rec = input.build_ticcmd().record();
                if t_row(&input, &rec, script.record) != want {
                    return Replay::Differs { tic };
                }
                tics += 1;
                cmp += 18;
            }
            _ => {}
        }
    }
    Replay::Exact { tics, fields: cmp }
}

/// Why the core refuses or differs on an `x_` or `s5_` script.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NotExact {
    /// The golden delivers a keydown of a withheld key to the switch, which
    /// [`doom_game::input::InputState::respond`] refuses ([`InputError::WithheldKey`]).
    Refused(u8),
    /// Genuine state outside the input path changes the input state.
    Differs,
}

/// The `x_` and `s5_` scripts on which the core is deliberately not exact, with the reason. All
/// other `x_` and `s5_` scripts must match at the event level.
const NOT_EXACT: &[(&str, NotExact, &str)] = &[
    (
        "x_latin1_ydiaeresis_is_KEY_PAUSE",
        NotExact::Refused(0xff),
        "keysym 0xff passes through xlatekey as KEY_PAUSE, whose sendpause writes BT_SPECIAL (g_game.c:561-565, 426-430)",
    ),
    (
        "s5_pause",
        NotExact::Refused(0xff),
        "KEY_PAUSE: sendpause, BT_SPECIAL",
    ),
    (
        "s5_pause_twice",
        NotExact::Refused(0xff),
        "KEY_PAUSE: sendpause, BT_SPECIAL",
    ),
    (
        "s5_q",
        NotExact::Refused(b'q'),
        "'q' held while recording ends the demo (g_game.c:1508-1509)",
    ),
    (
        "s5_q_norecord",
        NotExact::Refused(b'q'),
        "'q' reaches the switch; the app withholds it",
    ),
    (
        "s5_cheat_iddqd",
        NotExact::Refused(b'q'),
        "the cheat's 'q' ends the demo (g_game.c:1508-1509)",
    ),
    (
        "s5_cheat_iddqd_norecord",
        NotExact::Refused(b'q'),
        "the cheat's 'q' reaches the switch",
    ),
    // With the menu or a message box open, M_Responder passes a second F-key press on to
    // G_Responder (the golden's WHO = G), so the withheld keydown reaches the switch.
    (
        "s5_f1",
        NotExact::Refused(0xbb),
        "F1 again with the menu open",
    ),
    (
        "s5_f2",
        NotExact::Refused(0xbc),
        "F2 again with the menu open",
    ),
    (
        "s5_f3",
        NotExact::Refused(0xbd),
        "F3 again with the menu open",
    ),
    (
        "s5_f4",
        NotExact::Refused(0xbe),
        "F4 again with the menu open",
    ),
    (
        "s5_f6",
        NotExact::Refused(0xc0),
        "F6 again with the save menu open",
    ),
    (
        "s5_f7",
        NotExact::Refused(0xc1),
        "F7 again with the end-game message open",
    ),
    (
        "s5_f10",
        NotExact::Refused(0xc4),
        "F10 again with the quit message open",
    ),
    (
        "s5_cheat_idclev12",
        NotExact::Differs,
        "the cheat's G_DeferedInitNew reloads the level in G_Ticker, whose G_DoLoadLevel clears the input (g_game.c:490-496) with no loadlevel row; cheats are not ported (S5)",
    ),
];

#[test]
fn x_and_s5_scripts_at_the_event_level() {
    let (mut scripts, mut tics, mut cmp) = (0usize, 0usize, 0usize);
    let mut seen = Vec::new();
    for name in common::script_names() {
        if !(name.starts_with("x_") || name.starts_with("s5_")) {
            continue;
        }
        let listed = NOT_EXACT.iter().find(|(n, ..)| *n == name);
        match (replay(&name), listed) {
            (Replay::Exact { tics: t, fields }, None) => {
                scripts += 1;
                tics += t;
                cmp += fields;
            }
            (
                Replay::Refused(InputError::WithheldKey(k)),
                Some((_, NotExact::Refused(want), _)),
            ) if k == *want => seen.push(name),
            (Replay::Differs { .. }, Some((_, NotExact::Differs, _))) => seen.push(name),
            (outcome, listed) => panic!("{name}: {outcome:?}, listed as {listed:?}"),
        }
    }
    eprintln!("x_ and s5_ at the event level: {scripts} scripts exact, {tics} tics, {cmp} fields");
    assert_eq!(seen.len(), NOT_EXACT.len());
    assert_eq!(scripts + seen.len(), 42 + 33);
}
