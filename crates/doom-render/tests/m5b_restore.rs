//! M5b step 3: the saved display state (decision S18) and the display's refusals (decision S17).
//!
//! Restore and continue: at tic N-k the uninterrupted display's state is encoded (`DDSP` v1,
//! which embeds the DRST v2 bytes unchanged), decoded and restored into a fresh display next to a
//! clone of the simulation at N-k (doom-sim has no restore from its snapshot; `Simulation` is
//! `Clone`, as in M5a's rewind test). The fresh display then observes and draws N-k+1..=N: every
//! frame, palette number, palette set, face, `message_on`, message taken and `M_Random` index, and
//! the encoded state at N, must equal the uninterrupted run's, and every frame its genuine golden.
//!
//! Refusals: each named witness is produced from a constructed state and leaves the display's
//! complete state (its encoding) unchanged.
mod common;
mod m5b;

use doom_render::d_display::DisplayRestoreError;
use doom_render::hu_stuff::Message;
use doom_render::st_stuff::{Attacker, StatusInputs};
use doom_render::v_video::DrawError;
use doom_render::{
    Display, DisplayConfig, DisplayObserveError, DisplayRefusal, DisplayState, Frame,
    StateDecodeError,
};
use doom_sim::{PickupMessage, PlayerSlot, Simulation, TicCommand};
use m5b::{Item, items, load, read_stream};

fn item(lump: &str) -> Item {
    items().into_iter().find(|i| i.lump == lump).expect("item")
}

fn step(sim: &mut Simulation, cmd: TicCommand) {
    sim.advance([
        cmd,
        Default::default(),
        Default::default(),
        Default::default(),
    ])
    .expect("tic");
}

/// What one tic shows: the frame digest, the five P fields and the message taken.
type Shown = (String, [i64; 5], Option<Message>);

fn show(d: &mut Display, sim: &Simulation, cmd: &TicCommand, frame: &mut Frame) -> Shown {
    let m = d.observe(sim, cmd).expect("observe");
    let f = d.draw(sim, frame).expect("draw");
    let p = [
        i64::from(f.palette),
        f.palette_set.map_or(-1, i64::from),
        i64::from(f.face),
        i64::from(f.message_on),
        i64::from(d.status().rndindex),
    ];
    (doom_test_evidence::sha256(frame.pixels()), p, m)
}

/// (lump, restore tic N-k, target tic N, why), as frame-stream tics (the frame after
/// `advance` number t is tic t).
const CASES: &[(&str, usize, usize, &str)] = &[
    (
        "M5MSG",
        440,
        560,
        "a message showing and restarted, its counter carried; the turbo tic",
    ),
    (
        "M5STARM",
        10,
        200,
        "arms digits erased to STBAR, the evil grin and the key boxes",
    ),
    (
        "M5STDIG",
        150,
        330,
        "pain level 4, the rampage counter, three-digit ammunition",
    ),
    (
        "M5LOOK",
        830,
        965,
        "the look-around: facecount, priority and the M_Random index",
    ),
    (
        "M5PALST",
        1990,
        2060,
        "the radiation suit's blink and palette sets",
    ),
];

/// One case at one screenblocks value (each is its own test, so the 10 run on parallel threads).
fn restore_and_continue(case: usize, sb: u8) {
    let (lump, restore, target, why) = CASES[case];
    {
        let it = item(lump);
        {
            let golden = read_stream(lump, sb);
            let (res, level, mut sim, commands) = load(&it);
            let cfg = DisplayConfig { screenblocks: sb };
            let mut d = Display::new(res.clone(), level.clone(), cfg).expect("display");
            d.begin_level(&sim).expect("start");
            let mut frame = Frame::new();
            let mut saved = None;
            let mut straight = Vec::new();
            for (t, cmd) in commands.iter().enumerate().take(target + 1) {
                step(&mut sim, *cmd);
                let s = show(&mut d, &sim, cmd, &mut frame);
                if t == restore {
                    saved = Some((sim.clone(), d.state().expect("state").encode()));
                } else if t > restore {
                    straight.push(s);
                }
            }
            let end_state = d.state().expect("state").encode();
            let (mut sim2, bytes) = saved.expect("saved");
            let state = DisplayState::decode(&bytes).expect("decode");
            assert_eq!(state.encode(), bytes, "{lump}: encoding round trip");
            let mut d2 = Display::new(res, level, cfg).expect("display");
            d2.restore(state, &sim2).expect("restore");
            for (k, cmd) in commands[restore + 1..=target].iter().enumerate() {
                step(&mut sim2, *cmd);
                let s = show(&mut d2, &sim2, cmd, &mut frame);
                let t = (restore + 1 + k) as i64;
                assert_eq!(s, straight[k], "{lump} sb{sb} tic {t} ({why})");
                let g = &golden.frames[&t];
                assert_eq!(s.1, g.status, "{lump} sb{sb} tic {t}: golden P row");
                if g.columns.is_empty() {
                    assert_eq!(s.0, g.sha256, "{lump} sb{sb} tic {t}: golden frame");
                }
            }
            assert_eq!(
                d2.state().expect("state").encode(),
                end_state,
                "{lump} sb{sb}"
            );
        }
    }
}

macro_rules! cases {
    ($($name:ident = ($case:expr, $sb:expr)),*) => {$(
        #[test]
        fn $name() {
            restore_and_continue($case, $sb);
        }
    )*};
}

#[test]
fn the_cases_are_the_five_of_the_generated_tests() {
    let lumps: Vec<&str> = CASES.iter().map(|c| c.0).collect();
    assert_eq!(lumps, ["M5MSG", "M5STARM", "M5STDIG", "M5LOOK", "M5PALST"],);
}

cases!(
    restore_and_continue_m5msg_sb10 = (0, 10),
    restore_and_continue_m5msg_sb11 = (0, 11),
    restore_and_continue_m5starm_sb10 = (1, 10),
    restore_and_continue_m5starm_sb11 = (1, 11),
    restore_and_continue_m5stdig_sb10 = (2, 10),
    restore_and_continue_m5stdig_sb11 = (2, 11),
    restore_and_continue_m5look_sb10 = (3, 10),
    restore_and_continue_m5look_sb11 = (3, 11),
    restore_and_continue_m5palst_sb10 = (4, 10),
    restore_and_continue_m5palst_sb11 = (4, 11)
);

/// D_Display's `fullscreen` rule (d_main.c:242-243): after switching from screenblocks 11 to
/// 10, the next frame is a refresh (`ST_doRefresh`, st_stuff.c:1089-1100). From the switch on,
/// every frame and face equals that of a continuous screenblocks-10 display forced to refresh at
/// the same frame (`st_firsttime` set, icons never drawn), whose own frames match the goldens.
#[test]
fn switching_to_screenblocks_10_refreshes_the_bar() {
    let it = item("M5STARM");
    let (res, level, mut sim, commands) = load(&it);
    let mut d = Display::new(
        res.clone(),
        level.clone(),
        DisplayConfig { screenblocks: 11 },
    )
    .expect("display");
    let mut dd = Display::new(res, level, DisplayConfig { screenblocks: 10 }).expect("display");
    d.begin_level(&sim).expect("start");
    dd.begin_level(&sim).expect("start");
    let (mut f1, mut f2) = (Frame::new(), Frame::new());
    for (t, cmd) in commands.iter().enumerate().take(200) {
        step(&mut sim, *cmd);
        if t == 90 {
            d.set_screenblocks(10).expect("switch");
            dd.observe(&sim, cmd).expect("observe");
            let mut st = dd.state().expect("state");
            st.status.firsttime = true;
            // The icons were never drawn at screenblocks 11: no old box to erase
            // (st_lib.c:224-235), which can leave STARMS pixels the erase would restore.
            (st.status.arms_old, st.status.face_old, st.status.keys_old) = ([-1; 6], -1, [-1; 3]);
            dd.restore(st, &sim).expect("restore");
            let a = show_drawn(&mut d, &sim, cmd, &mut f1, true);
            let b = show_drawn(&mut dd, &sim, cmd, &mut f2, false);
            assert_eq!(a, b, "tic {t}");
            continue;
        }
        let a = show(&mut d, &sim, cmd, &mut f1);
        let b = show(&mut dd, &sim, cmd, &mut f2);
        if t > 90 {
            assert_eq!(a, b, "tic {t}");
        }
    }
    assert!(d.set_screenblocks(9).is_err());
}

/// The frame digest and face of a draw, observing first when `observe`.
fn show_drawn(
    d: &mut Display,
    sim: &Simulation,
    cmd: &TicCommand,
    frame: &mut Frame,
    observe: bool,
) -> (String, i32) {
    if observe {
        d.observe(sim, cmd).expect("observe");
    }
    let f = d.draw(sim, frame).expect("draw");
    (doom_test_evidence::sha256(frame.pixels()), f.face)
}

/// A display of M5MSG after `tics` tics, its simulation and commands.
fn msg_display(tics: usize) -> (Display, Simulation, Vec<TicCommand>) {
    let (res, level, mut sim, commands) = load(&item("M5MSG"));
    let mut d = Display::new(res, level, DisplayConfig { screenblocks: 10 }).expect("display");
    d.begin_level(&sim).expect("start");
    let mut frame = Frame::new();
    for cmd in &commands[..tics] {
        step(&mut sim, *cmd);
        show(&mut d, &sim, cmd, &mut frame);
    }
    (d, sim, commands)
}

fn encoded(d: &Display) -> Vec<u8> {
    d.state().expect("state").encode()
}

#[test]
fn each_witness_refuses_and_keeps_the_state() {
    let (mut d, sim, commands) = msg_display(30);
    let mut frame = Frame::new();
    let before = encoded(&d);
    let mut next = sim.clone();
    step(&mut next, commands[30]);
    // MessageUnderivable: a fed command with forward 51 on gametic 0 mod 128 would assign the
    // turbo message, which the simulation (fed forward <= 50) did not store.
    let mut s0 = sim.clone();
    let (mut d0, ..) = msg_display(0);
    let zero = TicCommand {
        forward: 51,
        ..commands[0]
    };
    let mut fresh = load(&item("M5MSG")).2;
    step(&mut fresh, commands[0]);
    let b0 = encoded(&d0);
    assert_eq!(
        d0.observe(&fresh, &zero),
        Err(DisplayObserveError::MessageUnderivable(PickupMessage::None))
    );
    assert_eq!(encoded(&d0), b0);
    // AttackerRemoved: damage from an attacker whose storage is gone (S17).
    let r = d.observe_with(&next, &commands[30], |i, _| {
        i.damagecount = 3;
        i.attacker = Attacker::Removed;
    });
    assert_eq!(r, Err(DisplayObserveError::AttackerRemoved));
    assert_eq!(encoded(&d), before);
    // MessageNotTaken: messages off (showMessages 0) and a message in the tic.
    let mut st = d.state().expect("state");
    st.hud.show_messages = 0;
    let mut d1 = d.clone();
    d1.restore(st, &sim).expect("restore");
    let b1 = encoded(&d1);
    let r = d1.observe_with(&next, &commands[30], |_, m| {
        *m = Some(Message { code: 1, text: "X" });
    });
    assert_eq!(r, Err(DisplayObserveError::MessageNotTaken));
    assert_eq!(encoded(&d1), b1);
    // A gap: observing the same tic twice is the renderer's gap refusal.
    assert!(matches!(
        d.observe(&sim, &commands[29]),
        Err(DisplayObserveError::Render(_))
    ));
    assert_eq!(encoded(&d), before);
    // UndefinedBackground: a diff frame at screenblocks 10 with a BG no patch wrote.
    let mut st = d.state().expect("state");
    st.status.bg_drawn = false;
    let mut d2 = d.clone();
    d2.restore(st, &sim).expect("restore");
    step(&mut s0, commands[30]);
    d2.observe(&s0, &commands[30]).expect("observe");
    let b2 = encoded(&d2);
    assert_eq!(
        d2.draw(&s0, &mut frame),
        Err(DisplayRefusal::Draw(DrawError::UndefinedBackground))
    );
    assert_eq!(encoded(&d2), b2);
    // BeforeFirstTic: D_Display draws nothing at gametic 0.
    let (mut d3, s3, _) = msg_display(0);
    let b3 = encoded(&d3);
    assert_eq!(
        d3.draw(&s3, &mut frame),
        Err(DisplayRefusal::BeforeFirstTic)
    );
    assert_eq!(encoded(&d3), b3);
}

#[test]
fn restore_and_decode_refuse_bad_states() {
    let (mut d, sim, _) = msg_display(20);
    let good = d.state().expect("state");
    let before = encoded(&d);
    let mut other = sim.clone();
    other.advance(Default::default()).expect("tic");
    assert!(matches!(
        d.restore(good.clone(), &other),
        Err(DisplayRestoreError::Tic {
            state: 20,
            simulation: 21
        })
    ));
    let mut bad = good.clone();
    bad.status.faceindex = 42;
    assert_eq!(
        d.restore(bad, &sim),
        Err(DisplayRestoreError::Value("faceindex"))
    );
    let mut bad = good.clone();
    bad.screenblocks = 9;
    assert!(matches!(
        d.restore(bad, &sim),
        Err(DisplayRestoreError::Config(_))
    ));
    assert_eq!(encoded(&d), before);
    let bytes = good.encode();
    assert_eq!(
        DisplayState::decode(&bytes[..10]).err(),
        Some(StateDecodeError::Truncated)
    );
    let mut b = bytes.clone();
    b[0] = b'X';
    assert_eq!(
        DisplayState::decode(&b).err(),
        Some(StateDecodeError::Magic)
    );
    let mut b = bytes.clone();
    b[4] = 2;
    assert_eq!(
        DisplayState::decode(&b).err(),
        Some(StateDecodeError::Version(2))
    );
    let mut b = bytes.clone();
    b.push(0);
    assert_eq!(
        DisplayState::decode(&b).err(),
        Some(StateDecodeError::Trailing)
    );
    // Every field's bound: corrupt the byte after the DRST block (screenblocks).
    let drst = u32::from_le_bytes(bytes[8..12].try_into().expect("4")) as usize;
    let mut b = bytes.clone();
    b[12 + drst] = 12;
    assert_eq!(
        DisplayState::decode(&b).err(),
        Some(StateDecodeError::Invalid("display flags"))
    );
    let mut bad = good.clone();
    bad.hud.message_counter = 141;
    assert_eq!(
        DisplayState::decode(&bad.encode()).err(),
        Some(StateDecodeError::Invalid("message_counter"))
    );
    let mut bad = good;
    bad.status.keyboxes[1] = 2;
    assert_eq!(
        DisplayState::decode(&bad.encode()).err(),
        Some(StateDecodeError::Invalid("keyboxes"))
    );
}

#[test]
fn adjusted_inputs_no_player_state_holds_are_refused_before_any_change() {
    let (mut d, sim, commands) = msg_display(20);
    let mut next = sim.clone();
    step(&mut next, commands[20]);
    let before = encoded(&d);
    type Probe = fn(&mut StatusInputs);
    let probes: [(Probe, &str); 6] = [
        (|i| i.health = -1, "health"),
        (|i| i.health = 201, "health"),
        (|i| i.readyweapon = 9, "readyweapon"),
        (|i| i.damagecount = i32::MAX, "damagecount"),
        (|i| i.bonuscount = i32::MAX, "bonuscount or powers"),
        (|i| i.ammo[0] = i.maxammo[0] + 1, "ammo"),
    ];
    for (probe, name) in probes {
        let r = d.observe_with(&next, &commands[20], |i, _| probe(i));
        assert_eq!(r, Err(DisplayObserveError::AdjustedInputOutOfRange(name)));
        assert_eq!(encoded(&d), before, "{name}");
    }
    // in range: observed and drawn, and the state still encodes and decodes (only that is
    // asserted; `DisplayState::check` would refuse it as `oldhealth`, since the adjusted health
    // of 0 is not the simulation's 100)
    let r = d.observe_with(&next, &commands[20], |i, _| {
        (i.health, i.damagecount, i.bonuscount) = (0, 100, i32::MAX - 7);
    });
    assert_eq!(r, Ok(None));
    d.draw(&next, &mut Frame::new()).expect("draw");
    let st = d.state().expect("state");
    assert_eq!(DisplayState::decode(&st.encode()).as_ref(), Ok(&st));
}

#[test]
fn restore_refuses_forged_states_whose_fields_disagree() {
    let (mut d, sim, _) = msg_display(20);
    let good = d.state().expect("state");
    assert_eq!(good.check(&sim).map(|_| ()), Ok(()));
    let before = encoded(&d);
    type Forge = fn(&mut DisplayState);
    let forged: [(Forge, &str); 12] = [
        (
            |s| (s.hud.message_on, s.hud.message_counter) = (true, 0),
            "message_on and message_counter",
        ),
        (
            |s| (s.hud.message_on, s.hud.message_counter) = (false, 9),
            "message_on and message_counter",
        ),
        (|s| s.hud.must_show = true, "must_show or protected"),
        (|s| s.hud.protected = true, "must_show or protected"),
        (|s| s.status.oldhealth = 250, "oldhealth"),
        (|s| s.status.keyboxes[0] = 0, "keyboxes or ready_ammo"),
        (|s| s.status.lastattackdown = 0, "lastattackdown"),
        // the dead face at health 100
        (
            |s| (s.status.faceindex, s.status.priority, s.status.facecount) = (41, 0, 16),
            "face",
        ),
        (
            |s| (s.status.faceindex, s.status.priority, s.status.facecount) = (41, 9, 0),
            "dead face",
        ),
        // the ouch face at pain level 4, which needs a health above 19
        (
            |s| (s.status.faceindex, s.status.priority, s.status.facecount) = (37, 7, 3),
            "face",
        ),
        (
            |s| (s.status.faceindex, s.status.priority, s.status.facecount) = (6, 8, 70),
            "face",
        ),
        (
            |s| (s.status.faceindex, s.status.priority, s.status.facecount) = (40, 0, 0),
            "face",
        ),
    ];
    for (forge, name) in forged {
        let mut bad = good.clone();
        forge(&mut bad);
        let bad = DisplayState::decode(&bad.encode()).expect("each field is in its range");
        assert_eq!(
            d.restore(bad, &sim),
            Err(DisplayRestoreError::Inconsistent(name))
        );
        assert_eq!(encoded(&d), before, "{name}");
    }
    let (d0, sim0, _) = msg_display(0);
    let mut start = d0.state().expect("state");
    assert_eq!(start.check(&sim0).map(|_| ()), Ok(()));
    start.status.oldhealth = 100;
    assert_eq!(
        start.check(&sim0).map(|_| ()),
        Err(DisplayRestoreError::Inconsistent("oldhealth"))
    );
}

#[test]
fn a_removed_attacker_refuses_through_the_simulation() {
    // M5FACED: imps hurt the player. The first tic with damage from an imp that has no
    // fireball in flight, the imp's removal is marked through the simulation's own API
    // (`Simulation::mark_remove`; wire 24 never removes a live monster, so the genuine runs have
    // no such tic) and its storage retires in the next tic, while the damage count is still up.
    let (res, level, mut sim, commands) = load(&item("M5FACED"));
    let mut d = Display::new(res, level, DisplayConfig { screenblocks: 10 }).expect("display");
    d.begin_level(&sim).expect("start");
    let mut frame = Frame::new();
    for (t, cmd) in commands.iter().enumerate() {
        step(&mut sim, *cmd);
        show(&mut d, &sim, cmd, &mut frame);
        let p = sim.original_player(PlayerSlot::One).expect("player");
        let Some(a) = p
            .attacker
            .filter(|a| p.damage_count >= 2 && Some(*a) != p.mo)
        else {
            continue;
        };
        let mut removed = sim.clone();
        if removed.mark_remove(a).is_err() {
            continue; // a fireball of this imp is in flight
        }
        step(&mut removed, commands[t + 1]);
        let q = removed.original_player(PlayerSlot::One).expect("player");
        if removed.actor(a).is_some() || q.attacker != Some(a) || q.damage_count == 0 {
            continue;
        }
        let before = encoded(&d);
        assert_eq!(
            d.observe(&removed, &commands[t + 1]),
            Err(DisplayObserveError::AttackerRemoved)
        );
        assert_eq!(encoded(&d), before);
        eprintln!("attacker removed after tic {t}");
        return;
    }
    panic!("no tic with a removable attacker");
}
