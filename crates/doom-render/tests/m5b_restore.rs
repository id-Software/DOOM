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
use doom_render::st_stuff::Attacker;
use doom_render::v_video::DrawError;
use doom_render::{
    Display, DisplayConfig, DisplayObserveError, DisplayRefusal, DisplayState, Frame,
    StateDecodeError,
};
use doom_sim::{PickupMessage, Simulation, TicCommand};
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

#[test]
fn restore_and_continue_matches_the_uninterrupted_run() {
    for &(lump, restore, target, why) in CASES {
        let it = item(lump);
        for sb in [10u8, 11] {
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
