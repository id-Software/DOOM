#![no_main]
//! The status bar and HUD state of `doom-render` (M5b, `d_display.rs`): the saved display state
//! (`DisplayState`, DDSP v1 embedding DRST v2) and a driven run of `Display`, on the 11 committed
//! M5b rooms (`tests/reference/m5b_wads/*.wad`, map E1M1) with `tests/reference/m5b_gfx.wad`
//! (synthetic graphics, no IWAD). A room loads as `render_frames.rs` loads one.
//!
//! Input: byte 0 picks the part (bit 0), byte 1 the room (`% 11`).
//!
//! Part 0, decode and restore: byte 2 is a number of empty tics `k` (`% 8`) the simulation and a
//! fresh display run before the restore; the rest is a candidate DDSP block. Properties: decode
//! never panics; a block that decodes re-encodes to the same bytes (canonical form) and decodes
//! to an equal value; restoring it into the display at tic `k` either refuses with a typed error
//! and leaves `state()` equal to its value before, or succeeds, after which four tics of observe
//! and draw give identical results (including refusals) on two clones, with palette 0..=13 and a
//! state that round-trips.
//!
//! Part 1, driven run: byte 2 bit 0 picks screenblocks 10 or 11, bits 1-7 the saved tic; byte 3
//! the budget (`16 << (data[3] % 3)` tics); then six-byte records: forward, side, `angleturn`
//! high byte, buttons (low six bits as a version-110 ticcmd, bits 6-7 the repeat 1, 2, 4, 8),
//! then an `observe_with` perturbation (opcode `% 14`, value) applied on each repeated tic: the
//! health, armour, ammunition, owned weapons, ready weapon, keys, damage and bonus counts,
//! powers, godmode, attack-down, attacker, position, angle, or the message (none, short, empty,
//! over 80 characters). Commands are clamped into the wire-24 domain. Properties: no panic or
//! overflow; every `draw` is a frame with palette 0..=13 or a typed refusal; a refused `observe`
//! or `draw` leaves `state()` equal to its value before; after every tic `state().encode()`
//! decodes to an equal state that encodes to the same bytes; the state saved at the chosen tic,
//! restored into a fresh `Display` beside a clone of the simulation, continues with the same
//! frames (hashed), palette numbers, faces, message flags and messages taken, and ends in the
//! same encoded state. `-max_len=391` (4 + 6 x 64 bytes of records; part 0 takes the DDSP
//! bytes of a room, about 65 KB, so run it with `-max_len=70000` or on its seeds).
use doom_render::hu_stuff::Message;
use doom_render::st_stuff::{Attacker, StatusInputs};
use doom_render::{
    Display, DisplayConfig, DisplayFrame, DisplayObserveError, DisplayRefusal, DisplayState, Frame,
    GameMode, Level, LumpDirectory, Resources,
};
use doom_sim::*;
use libfuzzer_sys::fuzz_target;
use std::hash::{Hash, Hasher};
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
const TEXTS: [&str; 4] = [
    "Picked up a clip.",
    "",
    "x",
    "a message far longer than the eighty characters of one heads-up line, which is cut at HU_MAXLINELENGTH",
];

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

fn display(index: usize, screenblocks: u8) -> Display {
    let s = setup(index);
    Display::new(
        s.res.clone(),
        s.level.clone(),
        DisplayConfig { screenblocks },
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

/// What one tic shows, comparable between two runs: the observe result, then the draw result
/// reduced to palette, palette set, face, message flag and a hash of the pixels.
type Shown = (
    Result<Option<Message>, DisplayObserveError>,
    Option<Result<(u8, Option<u8>, i32, bool, u64), DisplayRefusal>>,
);

fn check_state(d: &Display) {
    let state = d.state().expect("a level began");
    let bytes = state.encode();
    let back = DisplayState::decode(&bytes).expect("an encoded display state decodes");
    assert_eq!(back, state);
    assert_eq!(back.encode(), bytes);
}

/// One tic of observe and draw with the checks of the module documentation.
fn show(
    d: &mut Display,
    sim: &Simulation,
    cmd: &TicCommand,
    adjust: impl FnOnce(&mut StatusInputs, &mut Option<Message>),
) -> Shown {
    let before = d.state();
    let observed = d.observe_with(sim, cmd, adjust);
    if observed.is_err() {
        assert_eq!(d.state(), before, "a refused observe changed the state");
        return (observed, None);
    }
    check_state(d);
    let before = d.state();
    let mut frame = Frame::new();
    let drawn = d.draw(sim, &mut frame);
    if drawn.is_err() {
        assert_eq!(d.state(), before, "a refused draw changed the state");
    } else {
        check_state(d);
    }
    let drawn = drawn.map(|f: DisplayFrame| {
        assert!(f.palette <= 13, "palette {}", f.palette);
        assert!(f.palette_set.is_none_or(|p| p <= 13));
        let mut h = std::hash::DefaultHasher::new();
        frame.pixels().hash(&mut h);
        (f.palette, f.palette_set, f.face, f.message_on, h.finish())
    });
    (observed, Some(drawn))
}

fn part0(index: usize, k: u8, bytes: &[u8]) {
    let Ok(state) = DisplayState::decode(bytes) else {
        return;
    };
    assert_eq!(state.encode(), bytes, "a decoded block is not canonical");
    assert_eq!(DisplayState::decode(&state.encode()).as_ref(), Ok(&state));
    let (mut sim, mut d) = (start(index), display(index, 10));
    d.begin_level(&sim).expect("tic 0 is observable");
    for _ in 0..k % 8 {
        let cmd = TicCommand::default();
        if !step(&mut sim, cmd) || d.observe(&sim, &cmd).is_err() {
            return;
        }
    }
    let before = d.state();
    if d.restore(state, &sim).is_err() {
        assert_eq!(d.state(), before, "a refused restore changed the state");
        return;
    }
    check_state(&d);
    let (mut d2, mut sim2) = (d.clone(), sim.clone());
    for _ in 0..4 {
        let cmd = TicCommand::default();
        if !step(&mut sim, cmd) || !step(&mut sim2, cmd) {
            return;
        }
        let a = show(&mut d, &sim, &cmd, |_, _| {});
        let b = show(&mut d2, &sim2, &cmd, |_, _| {});
        assert_eq!(a, b, "a restored display is not deterministic");
        if a.0.is_err() {
            return;
        }
    }
    assert_eq!(
        d.state().map(|s| s.encode()),
        d2.state().map(|s| s.encode())
    );
}

/// One perturbation of `observe_with`, from an opcode and a value.
fn perturb(op: u8, v: u8, i: &mut StatusInputs, m: &mut Option<Message>) {
    let (n, u) = (usize::from(v), i32::from(v as i8));
    match op % 14 {
        1 => i.health = n as i32 * 8, // 0..=2040: P_DamageMobj clamps health at 0 (p_inter.c)
        2 => i.armor = u * 3,
        3 => i.ammo[n & 3] = u * 3,
        4 => i.weaponowned[n % 9] ^= true,
        5 => i.readyweapon = v % 9,
        6 => i.cards[n % 6] ^= true,
        7 => i.damagecount = i32::from(v),
        8 => i.bonuscount = i32::from(v),
        9 => i.powers[n % 6] = u * 40,
        10 => {
            i.godmode ^= v & 1 == 1;
            i.attackdown ^= v & 2 == 2;
        }
        11 => {
            i.attacker = match v % 4 {
                0 => Attacker::None,
                1 => Attacker::Player,
                2 => Attacker::At([u << 20, -(u << 18)]),
                _ => Attacker::Removed,
            }
        }
        12 => i.position = [u << 16, u.wrapping_mul(0x1234_5678)],
        13 => {
            i.angle = u32::from(v) << 24;
            *m = (v & 1 == 1).then(|| Message {
                code: -1,
                text: TEXTS[n >> 1 & 3],
            });
        }
        _ => {}
    }
}

/// The commands of a run: (command, perturbation opcode, value) per tic, clamped to wire 24.
fn commands(data: &[u8], budget: usize) -> Vec<(TicCommand, u8, u8)> {
    let mut out = Vec::new();
    for r in data.get(4..).unwrap_or(&[]).as_chunks::<6>().0 {
        let mut buttons = r[3] & 0x3f;
        let weapon = (buttons & BT_WEAPONMASK) >> 3;
        buttons &= !BT_WEAPONMASK;
        if buttons & BT_CHANGE != 0 {
            buttons |= (weapon & 3) << 3;
        }
        let c = TicCommand {
            forward: i8::from_le_bytes([r[0]]).clamp(-50, 50),
            side: i8::from_le_bytes([r[1]]).clamp(-50, 50),
            angle_turn: i16::from_le_bytes([0, r[2]]),
            buttons,
        };
        for _ in 0..1 << (r[3] >> 6) {
            if out.len() == budget {
                return out;
            }
            out.push((c, r[4], r[5]));
        }
    }
    out
}

fn part1(index: usize, data: &[u8]) {
    let screenblocks = 10 + (data[2] & 1);
    let budget = 16usize << (data[3] % 3);
    let cmds = commands(data, budget);
    let save = usize::from(data[2] >> 1) % cmds.len().max(1);
    let (mut sim, mut d) = (start(index), display(index, screenblocks));
    d.begin_level(&sim).expect("tic 0 is observable");
    check_state(&d);
    let mut log = Vec::new();
    let mut saved = None;
    for (t, &(cmd, op, v)) in cmds.iter().enumerate() {
        if !step(&mut sim, cmd) {
            break;
        }
        let shown = show(&mut d, &sim, &cmd, |i, m| perturb(op, v, i, m));
        let stop = shown.0.is_err();
        log.push(shown);
        if t == save {
            saved = Some((sim.clone(), d.state().expect("state").encode()));
        }
        if stop {
            break;
        }
    }
    let Some((mut sim2, bytes)) = saved else {
        return;
    };
    let state = DisplayState::decode(&bytes).expect("a saved state decodes");
    let mut d2 = display(index, screenblocks);
    d2.begin_level(&start(index)).expect("tic 0");
    d2.restore(state, &sim2)
        .expect("a state saved at this tic restores");
    for (t, &(cmd, op, v)) in cmds.iter().enumerate().take(log.len()).skip(save + 1) {
        assert!(step(&mut sim2, cmd));
        let shown = show(&mut d2, &sim2, &cmd, |i, m| perturb(op, v, i, m));
        assert_eq!(shown, log[t], "tic {t}: the restored run differs");
    }
    // The runs end together (a refused observe ends both, its state unchanged).
    assert_eq!(
        d.state().map(|s| s.encode()),
        d2.state().map(|s| s.encode())
    );
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }
    let index = usize::from(data[1]) % WADS.len();
    if data[0] & 1 == 0 {
        part0(index, data[2], &data[3..]);
    } else if data.len() >= 4 {
        part1(index, data);
    }
});
