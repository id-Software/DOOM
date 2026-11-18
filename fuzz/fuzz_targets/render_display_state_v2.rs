#![no_main]
//! The saved display state version 2 of M5c (`DDSP` version 2: M5b's layout with one contract
//! byte after the version, `DisplayState::decode_any`, note section 4.8.1) and the restore paths
//! that take it: `Display::restore` and doom-game's `FrameSource::restore`, which restores a
//! display state and a sound state (`DSND`) together. It sits beside `render_display_state` (M5b),
//! whose input format and checks are unchanged. Rust-only safety and consistency properties; the
//! frames and sound rows are compared with the genuine C elsewhere.
//!
//! Input: byte 0 picks the part (`% 3`), byte 1 the room (parts 0 and 1, `% 11`) or the saved
//! tic (part 2, `% 5`), byte 2 the empty tics `k` (`% 8`) before the restore (part 1) or the two
//! modes (part 2); the rest is the candidate (part 2: see below).
//!
//! Part 0, `decode_any` on arbitrary bytes. Never a panic. Fewer than 4 bytes: `Truncated`; a wrong
//! magic: `Magic`; fewer than 8: `Truncated`; a version other than 1 and 2: `Version(v)`. A
//! version-1 block gives exactly the result of the frozen `decode` (same state or same error). A
//! version-2 block: the frozen `decode` refuses it as `Version(2)`; a contract byte other than 1
//! is `Invalid("contract")` (a missing one `Truncated`); with contract byte 1 the result is the
//! frozen `decode` of the same bytes with version 1 and the byte removed, with the contract set to
//! `WithSound` (same error otherwise). An accepted block re-encodes to the same bytes (canonical),
//! decodes again to an equal state, and its version byte says its contract (1 for
//! `SoundFreeWipeFree`, 2 for `WithSound`).
//!
//! Part 1, `Display::restore`: an accepted block is restored into a fresh display of room `byte 1`
//! at tic `k` next to its simulation (the contract is the block's). Either a typed refusal that
//! leaves `state()` equal to its value before, or success, after which `state()` is the restored
//! state, it re-encodes to the same bytes, and a clone restored again gives the same state.
//!
//! Part 2, `FrameSource::restore` (display and sound together, as `SavedPresentation`): over the
//! committed synthetic item `SNDBUSY` with sound (`m5c_wads/sndbusy.wad`, `m5b_gfx.wad`,
//! `m5c_sfx.wad`, `m5c_music.wad`) at screen size 10. A genuine frame source runs the item's
//! demo; its saves at five tics (0, 10, 40, 80 and the last) and clones of the simulation are the
//! base. Byte 2 bits 0-1 are the display mode and bits 2-3 the sound mode of the candidate: 0 the
//! genuine bytes, 1 the genuine bytes with edits, 2 raw bytes. Bytes 3-4 are the length (little
//! endian) of the display payload, which follows, and the rest is the sound payload; an edits
//! payload is three-byte records (offset low, offset high, value) applied in order, offsets past
//! the end ignored; a mode 0 payload is ignored. Both genuine: the restore must succeed.
//! Otherwise either a typed refusal, or success, after which the
//! fresh source saves exactly the candidate bytes (both encodings are canonical), and a
//! second restore from that save gives an equal save; the restored source then runs one tic of the
//! item's demo (any result, never a panic; an `Err` leaves its save unchanged). Never a panic.
//! Run it with `-max_len=70000` or on its seeds (the display bytes of a room are about 65 KB).
use doom_game::app::{FrameSource, GameData, SavedPresentation};
use doom_render::{
    Display, DisplayConfig, DisplayState, Frame, GameMode, Level, LumpDirectory, RandomContract,
    Resources, StateDecodeError,
};
use doom_sim::*;
use libfuzzer_sys::fuzz_target;
use std::sync::{Arc, OnceLock};

const GFX: &[u8] = include_bytes!("../../tests/reference/m5b_gfx.wad");
const WADS: [&[u8]; 11] = [
    include_bytes!("../../tests/reference/m5b_wads/cheats.wad"),
    include_bytes!("../../tests/reference/m5b_wads/facedir.wad"),
    include_bytes!("../../tests/reference/m5b_wads/faceevil.wad"),
    include_bytes!("../../tests/reference/m5b_wads/facelook.wad"),
    include_bytes!("../../tests/reference/m5b_wads/faceouch.wad"),
    include_bytes!("../../tests/reference/m5b_wads/message.wad"),
    include_bytes!("../../tests/reference/m5b_wads/palbonus.wad"),
    include_bytes!("../../tests/reference/m5b_wads/palred.wad"),
    include_bytes!("../../tests/reference/m5b_wads/palsuit.wad"),
    include_bytes!("../../tests/reference/m5b_wads/starms.wad"),
    include_bytes!("../../tests/reference/m5b_wads/stdigit.wad"),
];
const ITEM_WAD: &[u8] = include_bytes!("../../tests/reference/m5c_wads/sndbusy.wad");
const ITEM_DEMO: &[u8] = include_bytes!("../../tests/reference/m5c_scripts/SNDBUSY.lmp");
const SFX: &[u8] = include_bytes!("../../tests/reference/m5c_sfx.wad");
const MUSIC: &[u8] = include_bytes!("../../tests/reference/m5c_music.wad");

struct Setup {
    map: doom_map::Map,
    res: Arc<Resources>,
    level: Arc<Level>,
}

fn setup(index: usize) -> &'static Setup {
    static SETUPS: [OnceLock<Setup>; 11] = [const { OnceLock::new() }; 11];
    SETUPS[index].get_or_init(|| {
        let wad = doom_wad::Wad::parse(WADS[index], doom_wad::Limits::default()).unwrap();
        let gfx = doom_wad::Wad::parse(GFX, doom_wad::Limits::default()).unwrap();
        let mut dir = LumpDirectory::new();
        dir.add_wad(&wad).unwrap();
        dir.add_wad(&gfx).unwrap();
        let res = Resources::load(dir, GameMode::Registered).expect("a committed room loads");
        let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).unwrap();
        let level = Level::load(&res, &map, 1).expect("a committed map resolves its textures");
        Setup {
            map,
            res: Arc::new(res),
            level: Arc::new(level),
        }
    })
}

fn start(index: usize) -> Simulation {
    let mut s = Simulation::original_player_episode1_v2_map(
        setup(index).map.clone(),
        1,
        Skill::Medium,
        MapContextOptions::default(),
    )
    .expect("a committed room loads under wire 24");
    s.spawn_map_things().expect("its THINGs spawn");
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .expect("the default inventory");
    s.spawn_level_specials().expect("its specials start");
    s
}

fn display(index: usize) -> Display {
    let s = setup(index);
    Display::new(
        s.res.clone(),
        s.level.clone(),
        DisplayConfig { screenblocks: 10 },
    )
    .expect("the committed graphics hold the status bar and font")
}

fn step(sim: &mut Simulation, cmd: TicCommand) -> bool {
    sim.advance([
        cmd,
        Default::default(),
        Default::default(),
        Default::default(),
    ])
    .is_ok()
}

fn part0(bytes: &[u8]) {
    let any = DisplayState::decode_any(bytes);
    let expect_err = |e: StateDecodeError| assert_eq!(any, Err(e), "{bytes:02x?}");
    if bytes.len() < 4 {
        return expect_err(StateDecodeError::Truncated);
    }
    if &bytes[..4] != b"DDSP" {
        return expect_err(StateDecodeError::Magic);
    }
    if bytes.len() < 8 {
        return expect_err(StateDecodeError::Truncated);
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    match version {
        1 => assert_eq!(
            any,
            DisplayState::decode(bytes),
            "a version-1 block differs"
        ),
        2 => {
            assert_eq!(
                DisplayState::decode(bytes),
                Err(StateDecodeError::Version(2))
            );
            match bytes.get(8) {
                None => return expect_err(StateDecodeError::Truncated),
                Some(&c) if c != 1 => return expect_err(StateDecodeError::Invalid("contract")),
                Some(_) => {}
            }
            let mut v1 = b"DDSP\x01\x00\x00\x00".to_vec();
            v1.extend_from_slice(&bytes[9..]);
            let want = DisplayState::decode(&v1).map(|mut s| {
                s.contract = RandomContract::WithSound;
                s
            });
            assert_eq!(any, want, "a version-2 block is not version 1 with a byte");
        }
        v => return expect_err(StateDecodeError::Version(v)),
    }
    let Ok(state) = any else { return };
    assert_eq!(state.encode(), bytes, "an accepted block is not canonical");
    let expected = match state.contract {
        RandomContract::SoundFreeWipeFree => 1,
        RandomContract::WithSound => 2,
    };
    assert_eq!(version, expected);
    assert_eq!(
        DisplayState::decode_any(&state.encode()).as_ref(),
        Ok(&state)
    );
}

fn part1(index: usize, k: u8, bytes: &[u8]) {
    let Ok(state) = DisplayState::decode_any(bytes) else {
        return;
    };
    let (mut sim, mut d) = (start(index), display(index));
    d.begin_level(&sim).expect("tic 0 is observable");
    for _ in 0..k % 8 {
        let cmd = TicCommand::default();
        if !step(&mut sim, cmd) || d.observe(&sim, &cmd).is_err() {
            return;
        }
    }
    let before = d.state();
    let mut again = d.clone();
    if d.restore(state.clone(), &sim).is_err() {
        assert_eq!(d.state(), before, "a refused restore changed the state");
        return;
    }
    assert_eq!(
        d.state().as_ref(),
        Some(&state),
        "restore does not give the state back"
    );
    assert_eq!(d.state().map(|s| s.encode()).as_deref(), Some(bytes));
    assert_eq!(d.contract(), state.contract);
    // The same state restored into a second display is the same state again.
    again
        .restore(state.clone(), &sim)
        .expect("a restore that worked once works again");
    assert_eq!(again.state(), d.state());
}

struct Item {
    data: GameData,
    commands: Vec<TicCommand>,
    /// (saved presentation, simulation at that tic) at the five saved tics.
    saves: Vec<(SavedPresentation, Simulation)>,
}

fn item() -> &'static Item {
    static ITEM: OnceLock<Item> = OnceLock::new();
    ITEM.get_or_init(|| {
        let (map, skill) = (ITEM_DEMO[3], ITEM_DEMO[1]);
        let (_, parsed) = doom_sim::demo::parse_demo_episode1_v2(ITEM_DEMO, map).expect("demo");
        let data = GameData::load_with_sound(
            &[ITEM_WAD, GFX, SFX, MUSIC],
            GameMode::Registered,
            map,
            skill,
        )
        .expect("the committed item loads with sound");
        let commands = parsed.commands;
        let n = commands.len();
        let mut sim = data.start().clone();
        let mut fs = FrameSource::start(&data, 10, &sim).expect("start");
        let mut frame = Frame::new();
        let wanted = [0, 10, 40, 80, n - 1];
        let mut saves = Vec::new();
        for (t, cmd) in commands.iter().enumerate() {
            assert!(step(&mut sim, *cmd), "the item's demo runs");
            fs.tic(&data, &sim, cmd, &mut frame)
                .expect("a tic of the item");
            if wanted.contains(&t) {
                saves.push((fs.save().expect("a save"), sim.clone()));
            }
        }
        assert_eq!(saves.len(), 5);
        Item {
            data,
            commands,
            saves,
        }
    })
}

/// The candidate of one part: the genuine bytes, the genuine bytes with edits, or raw bytes.
fn candidate(mode: u8, genuine: &[u8], payload: &[u8]) -> Vec<u8> {
    match mode & 3 {
        0 => genuine.to_vec(),
        1 => {
            let mut b = genuine.to_vec();
            for r in payload.chunks_exact(3) {
                let at = usize::from(u16::from_le_bytes([r[0], r[1]]));
                if let Some(v) = b.get_mut(at) {
                    *v = r[2];
                }
            }
            b
        }
        _ => payload.to_vec(),
    }
}

fn part2(which: u8, modes: u8, rest: &[u8]) {
    if rest.len() < 2 {
        return;
    }
    let it = item();
    let (saved, sim) = &it.saves[usize::from(which) % it.saves.len()];
    let dlen = usize::from(u16::from_le_bytes([rest[0], rest[1]])).min(rest.len() - 2);
    let (dpay, spay) = rest[2..].split_at(dlen);
    let display = candidate(modes, &saved.display, dpay);
    let sound = candidate(modes >> 2, saved.sound.as_ref().expect("DSND"), spay);
    let cand = SavedPresentation {
        display,
        sound: Some(sound),
    };
    let restored = FrameSource::restore(&it.data, sim, &cand);
    if modes & 0b1111 == 0 {
        assert!(
            restored.is_ok(),
            "a genuine save does not restore: {:?}",
            restored.err()
        );
    }
    let Ok(mut fresh) = restored else {
        return;
    };
    let back = fresh.save().expect("a restored source saves");
    assert_eq!(
        back, cand,
        "an accepted candidate does not re-encode to itself"
    );
    let again = FrameSource::restore(&it.data, sim, &back).expect("a saved source restores");
    assert_eq!(again.save().as_ref(), Some(&back));
    // One tic of the item's demo beyond the saved tic (any result, no panic).
    let tic = usize::try_from(sim.tic()).expect("small");
    if let Some(cmd) = it.commands.get(tic) {
        let mut sim2 = sim.clone();
        if step(&mut sim2, *cmd) {
            let mut frame = Frame::new();
            let before = fresh.save();
            if fresh.tic(&it.data, &sim2, cmd, &mut frame).is_err() {
                assert_eq!(
                    fresh.save(),
                    before,
                    "a refused tic changed the saved state"
                );
            }
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }
    match data[0] % 3 {
        0 => part0(&data[3..]),
        1 => part1(usize::from(data[1]) % WADS.len(), data[2], &data[3..]),
        _ => part2(data[1], data[2], &data[3..]),
    }
});
