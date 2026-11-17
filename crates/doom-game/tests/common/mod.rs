//! Readers of the M5b input scripts and goldens (format: tests/reference/m5b_input/README.md),
//! shared by the integration tests. No Rust output feeds an expectation: the goldens were
//! written by the genuine linuxdoom-1.10 input driver only.
#![allow(dead_code)]

use doom_game::input::{Event, TicCommand};
use std::path::{Path, PathBuf};

/// tests/reference/m5b_input.
pub fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference/m5b_input")
}

/// Every script name, sorted.
pub fn script_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir().join("scripts"))
        .expect("scripts directory")
        .map(|e| e.expect("entry").file_name().into_string().expect("utf-8"))
        .filter_map(|n| n.strip_suffix(".txt").map(str::to_owned))
        .collect();
    names.sort();
    names
}

/// The app's event domain: every script whose name does not start with `x_`, `s5_` or
/// `beyond_app_` (tools/reference/m5b_input_scripts.py docstring).
pub fn is_app_domain(name: &str) -> bool {
    !(name.starts_with("x_") || name.starts_with("s5_") || name.starts_with("beyond_app_"))
}

/// `strtol (s, &end, 0)` for the forms the scripts use: optional sign, decimal or `0x` hex.
pub fn c_int(s: &str) -> i64 {
    let (neg, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let v = match body.strip_prefix("0x") {
        Some(hex) => i64::from_str_radix(hex, 16),
        None => body.parse::<i64>(),
    }
    .unwrap_or_else(|_| panic!("not a number: {s}"));
    if neg { -v } else { v }
}

/// One script directive the event-level runner understands.
#[derive(Clone, Debug)]
pub enum Directive {
    /// `ev ...`: post the event now.
    Post(Event),
    /// `tic N`.
    Tic(u32),
    /// `loadlevel`.
    LoadLevel,
    /// `enddemo`.
    EndDemo,
    /// An X-level directive (`press`, `bpress`, `motion`, ...): not an `event_t`.
    XLevel,
}

/// A parsed script.
#[derive(Clone, Debug)]
pub struct Script {
    /// `config set NAME VALUE` rows, in order.
    pub config: Vec<(String, i32)>,
    /// Recording is on unless `config record 0`.
    pub record: bool,
    /// The directives after the configuration.
    pub directives: Vec<Directive>,
}

pub fn read_script(name: &str) -> Script {
    read_script_in("scripts", name)
}

/// A script of `scripts/` or `witness/`.
pub fn read_script_in(sub: &str, name: &str) -> Script {
    let text =
        std::fs::read_to_string(dir().join(sub).join(format!("{name}.txt"))).expect("script");
    let mut s = Script {
        config: Vec::new(),
        record: true,
        directives: Vec::new(),
    };
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.is_empty() || w[0].starts_with('#') {
            continue;
        }
        let d = match w.as_slice() {
            ["config", "set", n, v] => {
                s.config.push(((*n).to_owned(), c_int(v) as i32));
                continue;
            }
            ["config", "record", v] => {
                s.record = c_int(v) != 0;
                continue;
            }
            ["config", ..] => continue,
            ["ev", "keydown", k] => Directive::Post(Event::KeyDown(key(c_int(k)))),
            ["ev", "keyup", k] => Directive::Post(Event::KeyUp(key(c_int(k)))),
            ["ev", "mouse", b, x, y] => Directive::Post(Event::Mouse {
                buttons: u8::try_from(c_int(b)).expect("buttons"),
                dx: i32::try_from(c_int(x)).expect("dx"),
                dy: i32::try_from(c_int(y)).expect("dy"),
            }),
            ["tic"] => Directive::Tic(1),
            ["tic", n] => Directive::Tic(c_int(n) as u32),
            ["loadlevel"] => Directive::LoadLevel,
            ["enddemo"] => Directive::EndDemo,
            _ => Directive::XLevel,
        };
        s.directives.push(d);
    }
    s
}

fn key(v: i64) -> u8 {
    u8::try_from(v).expect("event-level key codes are 0..=255")
}

/// The 18 fields of a `t` row after its tic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TRow {
    pub built: [i32; 4],
    pub executed: [i32; 4],
    pub demo: String,
    pub turnheld: i32,
    pub dclick: [i32; 3],
    pub dclick2: [i32; 3],
    pub mb: i32,
    pub keys: String,
}

/// One golden row; `r` rows are expanded into `T` rows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Row {
    H(String),
    S(i64, String),
    L(i64),
    E {
        tic: i64,
        ty: i64,
        d1: i64,
        d2: Option<i64>,
        d3: Option<i64>,
        who: String,
    },
    Q(i64, i64, i64),
    T(i64, TRow),
    C(Vec<i64>),
    X(i64, String),
    Z(i64, i64),
}

pub fn read_golden(name: &str) -> Vec<Row> {
    let text = std::fs::read_to_string(dir().join("goldens").join(format!("{name}.golden")))
        .expect("golden");
    let mut rows: Vec<Row> = Vec::new();
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| c_int(w[i]);
        let opt = |s: &str| if s == "-" { None } else { Some(c_int(s)) };
        let row = match w[0] {
            "h" => Row::H(w[1].to_owned()),
            "s" => Row::S(n(1), w[2].to_owned()),
            "L" => Row::L(n(1)),
            "e" => Row::E {
                tic: n(1),
                ty: n(2),
                d1: n(3),
                d2: opt(w[4]),
                d3: opt(w[5]),
                who: w[6].to_owned(),
            },
            "q" => Row::Q(n(1), n(2), n(3)),
            "t" => {
                assert_eq!(w.len(), 20, "t row: {line}");
                let i = |k: usize| n(k) as i32;
                Row::T(
                    n(1),
                    TRow {
                        built: [i(2), i(3), i(4), i(5)],
                        executed: [i(6), i(7), i(8), i(9)],
                        demo: w[10].to_owned(),
                        turnheld: i(11),
                        dclick: [i(12), i(13), i(14)],
                        dclick2: [i(15), i(16), i(17)],
                        mb: i(18),
                        keys: w[19].to_owned(),
                    },
                )
            }
            "r" => {
                let Some(Row::T(tic, t)) = rows.last().cloned() else {
                    panic!("r row not after a t row in {name}");
                };
                for k in 1..=n(1) {
                    rows.push(Row::T(tic + k, t.clone()));
                }
                continue;
            }
            "c" => Row::C(w[1..].iter().map(|s| c_int(s)).collect()),
            "x" => Row::X(n(1), w[2..].join(" ")),
            "z" => Row::Z(n(1), n(2)),
            other => panic!("unknown row kind {other} in {name}"),
        };
        rows.push(row);
    }
    rows
}

/// The four command fields as golden integers.
pub fn fields(c: &TicCommand) -> [i32; 4] {
    [
        i32::from(c.forward),
        i32::from(c.side),
        i32::from(c.angle_turn),
        i32::from(c.buttons),
    ]
}

/// Wire 24 (`PlayerMonstersEpisode1V2`) admission of a command, restated from the simulation's
/// preflight (crates/doom-sim/src/player_commands.rs, `preflight_player_commands`: under the
/// movers policy the buttons must lie in `BT_ATTACK | BT_USE | BT_CHANGE | BT_WEAPONMASK`, and
/// under the Episode 1 policy `forward` and `side` in −127..=127) and
/// crates/doom-sim/src/player_weapons.rs (`validate_command`: weapon bits only with
/// `BT_CHANGE`).
pub fn wire24_admits(c: &TicCommand) -> bool {
    use doom_sim::{BT_ATTACK, BT_CHANGE, BT_USE, BT_WEAPONMASK};
    c.buttons & !(BT_ATTACK | BT_USE | BT_CHANGE | BT_WEAPONMASK) == 0
        && (-127..=127).contains(&c.forward)
        && (-127..=127).contains(&c.side)
        && (c.buttons & BT_CHANGE != 0 || c.buttons & BT_WEAPONMASK == 0)
}
