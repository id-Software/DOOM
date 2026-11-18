//! M5c: the rules of the display's random contract (`RandomContract`, M5c note §4.3.4) and the
//! saved display state `DDSP` version 2 (note §4.8.1), Rust-only checks of the display's own API
//! (the frames themselves are compared with the genuine goldens in tests/m5c_display.rs and,
//! end to end, in crates/doom-game/tests/m5c_frames.rs).
//!
//! - The default is the M5b contract, whose state still encodes as version 1.
//! - Under `WithSound`, `observe` and `observe_with` are refused (sound draws cannot be skipped
//!   silently); `observe_after(draws)` advances the index by the draws, then `ST_Ticker` draws once
//!   (g_game.c:730-731, st_stuff.c:992); the first frame adds the wipe's 320 (f_wipe.c:159-166), the
//!   next frames add nothing. Under the M5b contract `observe_after` with draws is refused and with 0
//!   is exactly `observe`. The contract can be chosen only before the first tic. Every refusal
//!   leaves the state (its encoding) unchanged.
//! - `DDSP` version 2: version 1's bytes with the version 2 and one contract byte (1) inserted;
//!   M5b's `decode` refuses it as `Version(2)`, `decode_any` reads both versions; forged contract
//!   bytes, versions and truncations are refused; a version-2 state restores into a fresh display,
//!   which then continues exactly as the uninterrupted one.
mod common;
mod m5c;

use doom_render::{
    Display, DisplayConfig, DisplayObserveError, DisplayState, Frame, RandomContract,
    StateDecodeError, WIPE_DRAWS,
};
use m5c::{load, wire25};

fn index_of(lump: &str) -> usize {
    wire25::ITEMS
        .iter()
        .position(|i| i.0 == lump)
        .expect("item")
}

fn encoded(d: &Display) -> Vec<u8> {
    d.state().expect("state").encode()
}

fn tic(sim: &mut doom_sim::Simulation, cmd: doom_sim::TicCommand) {
    sim.advance([
        cmd,
        Default::default(),
        Default::default(),
        Default::default(),
    ])
    .expect("tic");
}

#[test]
fn the_contract_rules() {
    let mut it = load(index_of("SNDBUSY"));
    let cfg = DisplayConfig { screenblocks: 10 };
    let mut d = Display::new(it.res.clone(), it.level.clone(), cfg).expect("display");
    assert_eq!(d.contract(), RandomContract::SoundFreeWipeFree);
    d.begin_level(&it.sim).expect("level start");
    let mut m5b = d.clone();
    let v1 = encoded(&d);
    assert_eq!(
        &v1[4..8],
        &1u32.to_le_bytes(),
        "the M5b contract writes version 1"
    );
    d.set_contract(RandomContract::WithSound)
        .expect("before the first tic");
    let v2 = encoded(&d);
    assert_eq!(
        &v2[4..9],
        &[2, 0, 0, 0, 1],
        "version 2 and the contract byte"
    );
    assert_eq!(&v2[9..], &v1[8..], "nothing else changes in the layout");
    let mut frame = Frame::new();
    let cmd = it.commands[0];
    tic(&mut it.sim, cmd);
    // refusals under WithSound, state unchanged
    let before = encoded(&d);
    assert_eq!(d.observe(&it.sim, &cmd), Err(DisplayObserveError::Contract));
    assert_eq!(
        d.observe_with(&it.sim, &cmd, |_, _| {}),
        Err(DisplayObserveError::Contract)
    );
    assert_eq!(encoded(&d), before);
    // the M5b display refuses draws but takes 0 exactly as observe
    let m5b_before = encoded(&m5b);
    assert_eq!(
        m5b.observe_after(&it.sim, &cmd, 1),
        Err(DisplayObserveError::Contract)
    );
    assert_eq!(encoded(&m5b), m5b_before);
    let mut plain = m5b.clone();
    assert_eq!(
        m5b.observe_after(&it.sim, &cmd, 0),
        plain.observe(&it.sim, &cmd)
    );
    assert_eq!(encoded(&m5b), encoded(&plain));
    // tic 0 with 3 sound draws: 3 + ST_Ticker's 1, then the wipe's 320 after the frame
    d.observe_after(&it.sim, &cmd, 3).expect("observe");
    assert_eq!(d.status().rndindex, 4);
    d.draw(&it.sim, &mut frame).expect("frame");
    assert_eq!(u16::from(d.status().rndindex), (4 + WIPE_DRAWS) % 256);
    assert_eq!(d.status().rndindex, 68);
    // the M5b display's frame of tic 0 adds nothing
    m5b.draw(&it.sim, &mut frame).expect("frame");
    assert_eq!(m5b.status().rndindex, 1);
    // the contract is fixed after the first tic
    assert_eq!(
        d.set_contract(RandomContract::SoundFreeWipeFree),
        Err(DisplayObserveError::Contract)
    );
    assert_eq!(d.contract(), RandomContract::WithSound);
    // tic 1: 300 draws (mod 256: 44) and ST_Ticker's, no wipe any more
    let cmd = it.commands[1];
    tic(&mut it.sim, cmd);
    d.observe_after(&it.sim, &cmd, 300).expect("observe");
    d.draw(&it.sim, &mut frame).expect("frame");
    assert_eq!(d.status().rndindex, (68 + 44 + 1) as u8);
}

#[test]
fn ddsp_version_2_round_trips_and_forgeries_are_refused() {
    let mut it = load(index_of("SNDBUSY"));
    let cfg = DisplayConfig { screenblocks: 10 };
    let mut d = Display::new(it.res.clone(), it.level.clone(), cfg).expect("display");
    d.begin_level(&it.sim).expect("level start");
    d.set_contract(RandomContract::WithSound).expect("contract");
    let mut frame = Frame::new();
    for cmd in &it.commands[..20] {
        tic(&mut it.sim, *cmd);
        d.observe_after(&it.sim, cmd, 1).expect("observe");
        d.draw(&it.sim, &mut frame).expect("frame");
    }
    let state = d.state().expect("state");
    assert_eq!(state.contract, RandomContract::WithSound);
    let bytes = state.encode();
    assert_eq!(
        DisplayState::decode(&bytes),
        Err(StateDecodeError::Version(2)),
        "M5b's decoder refuses version 2"
    );
    let back = DisplayState::decode_any(&bytes).expect("version 2 decodes");
    assert_eq!(back, state);
    assert_eq!(back.encode(), bytes, "canonical");
    // a version-1 state still decodes as the M5b contract through both decoders
    let mut v1_state = state.clone();
    v1_state.contract = RandomContract::SoundFreeWipeFree;
    let v1 = v1_state.encode();
    assert_eq!(&v1[4..8], &1u32.to_le_bytes());
    assert_eq!(DisplayState::decode(&v1).as_ref(), Ok(&v1_state));
    assert_eq!(DisplayState::decode_any(&v1).as_ref(), Ok(&v1_state));
    // forgeries
    for contract in [0u8, 2, 255] {
        let mut b = bytes.clone();
        b[8] = contract;
        assert_eq!(
            DisplayState::decode_any(&b),
            Err(StateDecodeError::Invalid("contract")),
            "contract byte {contract}"
        );
    }
    for version in [0u32, 3, u32::MAX] {
        let mut b = bytes.clone();
        b[4..8].copy_from_slice(&version.to_le_bytes());
        assert_eq!(
            DisplayState::decode_any(&b),
            Err(StateDecodeError::Version(version))
        );
    }
    let mut b = bytes.clone();
    b[0] = b'X';
    assert_eq!(DisplayState::decode_any(&b), Err(StateDecodeError::Magic));
    let mut b = bytes.clone();
    b.push(0);
    assert_eq!(
        DisplayState::decode_any(&b),
        Err(StateDecodeError::Trailing)
    );
    for n in 0..bytes.len() {
        assert!(
            DisplayState::decode_any(&bytes[..n]).is_err(),
            "prefix of {n} bytes"
        );
    }
    // the v1 field rules apply to version 2: screenblocks 12 after the DRST block
    let drst = u32::from_le_bytes(bytes[9..13].try_into().expect("4")) as usize;
    let mut b = bytes.clone();
    b[13 + drst] = 12;
    assert_eq!(
        DisplayState::decode_any(&b),
        Err(StateDecodeError::Invalid("display flags"))
    );
}

#[test]
fn a_version_2_state_restores_and_continues_exactly() {
    for (lump, at) in [("SNDBUSY", 0usize), ("SNDBUSY", 40), ("SNDMONST", 200)] {
        let mut it = load(index_of(lump));
        let cfg = DisplayConfig { screenblocks: 10 };
        let mut d = Display::new(it.res.clone(), it.level.clone(), cfg).expect("display");
        d.begin_level(&it.sim).expect("level start");
        d.set_contract(RandomContract::WithSound).expect("contract");
        let mut fresh = Display::new(it.res.clone(), it.level.clone(), cfg).expect("display");
        let mut frame = Frame::new();
        let draws = |t: usize| (t % 3) as u16;
        let run = |d: &mut Display, sim: &doom_sim::Simulation, t: usize, frame: &mut Frame| {
            d.observe_after(sim, &it.commands[t], draws(t))
                .expect("observe");
            let f = d.draw(sim, frame).expect("frame");
            (
                doom_test_evidence::sha256(frame.pixels()),
                f.palette,
                f.face,
                d.status().rndindex,
            )
        };
        let mut restored_sim = None;
        let mut shown = Vec::new();
        for t in 0..at + 30 {
            tic(&mut it.sim, it.commands[t]);
            shown.push(run(&mut d, &it.sim, t, &mut frame));
            if t == at {
                let bytes = d.state().expect("state").encode();
                let state = DisplayState::decode_any(&bytes).expect("decode");
                // a forged state (screenblocks 9) is refused with the target unchanged
                let mut wrong = state.clone();
                wrong.screenblocks = 9;
                let untouched = fresh.clone();
                assert!(fresh.restore(wrong, &it.sim).is_err());
                assert_eq!(fresh.state(), untouched.state());
                fresh.restore(state, &it.sim).expect("restore");
                assert_eq!(fresh.contract(), RandomContract::WithSound);
                restored_sim = Some(it.sim.clone());
            }
        }
        let mut sim = restored_sim.expect("restored");
        for (k, t) in (at + 1..at + 30).enumerate() {
            tic(&mut sim, it.commands[t]);
            assert_eq!(
                run(&mut fresh, &sim, t, &mut frame),
                shown[at + 1 + k],
                "{lump} {t}"
            );
        }
        assert_eq!(encoded(&fresh), encoded(&d), "{lump}");
    }
}
