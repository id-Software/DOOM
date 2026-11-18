//! M5b step 11: tests for the mutants of `cargo mutants` that the display tests left alive in
//! `doom-render`'s status bar, HUD and display state code. Every expectation is a C fact already
//! cited next to the code (linuxdoom-1.10 file:line in each test) or a relation between frames
//! that the C implies; none is a captured Rust output. Each test names the mutants it kills as
//! `file:line:col` of the 334cd4b sources. The doc of each test says what it pins.
//!
//! Tests below the public API: none here. Everything runs through `Display`, `DisplayState`,
//! `StatusBarState::tick`, `HudState::erase` and `status_inputs`, which are public. The
//! mutants that no public route reaches (negative numbers, width-2 numbers, `val == false`
//! binary icons, `copy_bg` range checks) are killed by unit tests in `src/st_lib.rs` and
//! `src/v_video.rs`, each saying why.
mod common;
mod m5b;

use doom_render::d_display::{DisplayRestoreError, status_inputs};
use doom_render::hu_stuff::HudState;
use doom_render::st_stuff::{Attacker, DEADFACE, StatusBarState, StatusInputs};
use doom_render::v_video::DrawError;
use doom_render::{
    Display, DisplayConfig, DisplayRefusal, DisplayState, Frame, GameMode, Level, LumpDirectory,
    Resources, StateDecodeError,
};
use doom_sim::{
    DamageRequest, MapContextOptions, PickupMessage, PlayerSlot, Simulation, Skill, TicCommand,
    WeaponInventory,
};
use m5b::{Item, items, load};
use std::sync::Arc;

type Loaded = (Arc<Resources>, Arc<Level>, Simulation, Vec<TicCommand>);

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

/// The genuine lump list with `m5b_gfx.wad`'s bytes altered by `edit` (the same construction as
/// `m5b::load`, whose gfx bytes are fixed): `edit` gets each lump's name and returns a header
/// field to overwrite (byte offset in the patch header, new `i16`).
fn load_edited(it: &Item, edit: &dyn Fn(&str) -> Option<(usize, i16)>) -> Loaded {
    let wad_bytes = std::fs::read(&it.wad).expect("WAD");
    let mut gfx_bytes = std::fs::read(common::reference("m5b_gfx.wad")).expect("gfx");
    let lmp = std::fs::read(&it.lmp).expect("demo");
    let le32 = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().expect("4"));
    let (n, dir) = (le32(&gfx_bytes, 4) as usize, le32(&gfx_bytes, 8) as usize);
    let mut changed = 0;
    for k in 0..n {
        let e = dir + 16 * k;
        let pos = le32(&gfx_bytes, e) as usize;
        let raw = gfx_bytes[e + 8..e + 16].to_vec();
        let name = String::from_utf8_lossy(&raw[..raw.iter().position(|b| *b == 0).unwrap_or(8)])
            .into_owned();
        if let Some((off, v)) = edit(&name) {
            gfx_bytes[pos + off..pos + off + 2].copy_from_slice(&v.to_le_bytes());
            changed += 1;
        }
    }
    assert!(changed > 0, "the edit matched no lump");
    let wad = doom_wad::Wad::parse(&wad_bytes, doom_wad::Limits::default()).expect("WAD");
    let gfx = doom_wad::Wad::parse(&gfx_bytes, doom_wad::Limits::default()).expect("gfx");
    let mut d = LumpDirectory::new();
    d.add_wad(&wad).expect("map WAD");
    d.add_wad(&gfx).expect("gfx");
    d.add_single_lump(&it.lmp.to_string_lossy(), &lmp)
        .expect("demo lump");
    let res = Resources::load(d, GameMode::Registered).expect("resources");
    let map_number = lmp[3];
    let map = doom_map::Map::from_wad(
        &wad,
        &format!("E{}M{map_number}", lmp[2]),
        Default::default(),
    )
    .expect("map");
    let level = Level::load(&res, &map, lmp[2]).expect("level");
    let (header, demo) = doom_sim::demo::parse_demo_episode1_v2(&lmp, map_number).expect("demo");
    let skill = [Skill::Baby, Skill::Easy, Skill::Medium, Skill::Hard][usize::from(header.skill)];
    let mut sim = Simulation::original_player_episode1_v2_map(
        map,
        map_number,
        skill,
        MapContextOptions::default(),
    )
    .expect("admission");
    sim.spawn_map_things().expect("THINGs");
    sim.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .expect("weapons");
    sim.spawn_level_specials().expect("specials");
    (Arc::new(res), Arc::new(level), sim, demo.commands)
}

/// A display at screenblocks 10 that has observed and drawn `pre` tics.
struct Base {
    res: Arc<Resources>,
    d: Display,
    sim: Simulation,
}

fn base(loaded: Loaded, pre: usize) -> Base {
    let (res, level, mut sim, commands) = loaded;
    let mut d =
        Display::new(res.clone(), level, DisplayConfig { screenblocks: 10 }).expect("display");
    d.begin_level(&sim).expect("start");
    let mut frame = Frame::new();
    for cmd in &commands[..pre] {
        step(&mut sim, *cmd);
        d.observe(&sim, cmd).expect("observe");
        d.draw(&sim, &mut frame).expect("draw");
    }
    Base { res, d, sim }
}

impl Base {
    /// The current tic drawn again with the HUD message line replaced by `text`, shown.
    fn with_message(&self, text: &[u8]) -> Vec<u8> {
        let mut st = self.d.state().expect("state");
        st.hud.line.text = text.to_vec();
        st.hud.message_on = true;
        st.hud.message_counter = 35;
        let mut d = self.d.clone();
        d.restore(st, &self.sim).expect("restore");
        let mut frame = Frame::new();
        d.draw(&self.sim, &mut frame).expect("draw");
        frame.pixels().to_vec()
    }
}

fn header(res: &Resources, name: &str) -> [i32; 4] {
    let n = res
        .directory()
        .get_num_for_name(name.as_bytes())
        .expect("lump");
    let b = res.lump(n);
    let w = |i: usize| i32::from(i16::from_le_bytes([b[i], b[i + 1]]));
    [w(0), w(2), w(4), w(6)]
}

// ---------------------------------------------------------------------------------------------
// status bar numbers (st_lib.c:91-148)
// ---------------------------------------------------------------------------------------------

// The negative-number, minus-sign and digit-count tests of `STlib_drawNum` (st_lib.c:91-148) are
// unit tests in `src/st_lib.rs`: `Display::observe_with` refuses a health no player holds, so no
// public route draws a negative number.

// ---------------------------------------------------------------------------------------------
// st_lib.c's I_Error guards (st_lib.c:233, 282), reached through a status bar patch whose
// topoffset puts its box above ST_Y: the genuine code stops, the display refuses the frame
// ---------------------------------------------------------------------------------------------

/// Pins st_lib.c:271-282: `updateBinIcon: y - ST_Y < 0` with `y = bi->y - topoffset`. With
/// STARMS (the arms background, at y 168 = ST_Y, st_stuff.c:150-151) given a topoffset of 1, the
/// first frame has y = 167 and `I_Error`s (kills st_lib.rs:128:16 `-`->`+`, 129:11 `-`->`+` and
/// `-`->`/`, 129:11 `<`->`>`).
#[test]
fn a_binary_icon_above_the_bar_is_refused() {
    let (res, level, mut sim, commands) =
        load_edited(&item("M5STDIG"), &|n| (n == "STARMS").then_some((6, 1)));
    let mut d = Display::new(res, level, DisplayConfig { screenblocks: 10 }).expect("display");
    d.begin_level(&sim).expect("start");
    step(&mut sim, commands[0]);
    d.observe(&sim, &commands[0]).expect("observe");
    let before = d.state().expect("state").encode();
    assert_eq!(
        d.draw(&sim, &mut Frame::new()),
        Err(DisplayRefusal::Draw(DrawError::StatusBarError(
            "updateBinIcon: y - ST_Y < 0"
        )))
    );
    assert_eq!(d.state().expect("state").encode(), before, "state kept");
}

/// Pins st_lib.c:224-233: `updateMultIcon: y - ST_Y < 0` with `y = mi->y - topoffset` of the
/// OLD patch, when `oldinum != -1`. Every STF* patch (the faces, at y 168, st_stuff.c:117-118)
/// gets a topoffset of 1. The first frame draws without an old patch; a refresh of the second
/// frame (`st_firsttime`) erases the old face's box and `I_Error`s (kills st_lib.rs:99:15 `-`->`+`
/// and `-`->`/`).
#[test]
fn a_multiple_icon_above_the_bar_is_refused_when_it_has_an_old_patch() {
    let (res, level, mut sim, commands) = load_edited(&item("M5STDIG"), &|n| {
        n.starts_with("STF").then_some((6, 1))
    });
    let mut d = Display::new(res, level, DisplayConfig { screenblocks: 10 }).expect("display");
    d.begin_level(&sim).expect("start");
    let mut frame = Frame::new();
    step(&mut sim, commands[0]);
    d.observe(&sim, &commands[0]).expect("observe");
    d.draw(&sim, &mut frame)
        .expect("first frame has no old face to erase");
    step(&mut sim, commands[1]);
    d.observe(&sim, &commands[1]).expect("observe");
    let mut st = d.state().expect("state");
    st.status.firsttime = true;
    d.restore(st, &sim).expect("restore");
    let before = d.state().expect("state").encode();
    assert_eq!(
        d.draw(&sim, &mut frame),
        Err(DisplayRefusal::Draw(DrawError::StatusBarError(
            "updateMultIcon: y - ST_Y < 0"
        )))
    );
    assert_eq!(d.state().expect("state").encode(), before, "state kept");
}

// ---------------------------------------------------------------------------------------------
// the face (st_stuff.c:752-922), through `StatusBarState::tick`
// ---------------------------------------------------------------------------------------------

fn fresh() -> StatusBarState {
    StatusBarState::level_start([false; 9])
}

/// Pins ST_initData (st_stuff.c:1250-1278) and STlib_initMultIcon (st_lib.c:203): at a level
/// start `st_oldhealth` is -1, `keyboxes[i]` is -1, `st_palette` is -1 and every multiple icon's
/// `oldinum` is -1. Under wire 24 these are unobservable in frames (the first tic has no damage,
/// `keyboxes` is rewritten before the first draw, and palette 1 and the icons' first erase leave
/// the same pixels), so the values are pinned directly (kills st_stuff.rs:252:24, 258:24,
/// 260:22, 263:24 delete `-`).
#[test]
fn a_level_start_has_minus_one_histories() {
    let s = StatusBarState::level_start([false; 9]);
    assert_eq!(s.oldhealth, -1);
    assert_eq!(s.keyboxes, [-1; 3]);
    assert_eq!(s.palette, -1);
    assert_eq!(s.arms_old, [-1; 6]);
    assert_eq!(s.face_old, -1);
    assert_eq!(s.keys_old, [-1; 3]);
    assert_eq!(s.lastattackdown, -1);
}

/// Pins st_stuff.c:777-796: the evil grin is considered only while `priority < 9`, and the dead
/// face has set priority 9 (st_stuff.c:763-770): a dead player picking up a weapon keeps the
/// dead face and `oldweaponsowned` (kills st_stuff.rs:301:26 `<`->`<=`).
#[test]
fn the_dead_face_is_not_replaced_by_the_evil_grin() {
    let mut s = fresh();
    let mut p = StatusInputs {
        health: 0,
        bonuscount: 1,
        ..StatusInputs::default()
    };
    p.weaponowned[8] = true;
    s.tick(&p).expect("tick");
    assert_eq!((s.faceindex, s.priority), (DEADFACE, 9));
    assert!(!s.oldweaponsowned[8], "oldweaponsowned is not updated");
}

/// The attacker branch, st_stuff.c:798-850. The ouch test is `health - st_oldhealth >
/// ST_MUCHPAIN` (20, st_stuff.c:126) and the ouch face is `ST_calcPainOffset() +
/// ST_OUCHOFFSET` (5). `pain` is `8 * ((100 - health) * 5 / 101)`: 16 at health 50, 24 at 30.
/// Kills st_stuff.rs:323:29 `-`->`/`, 323:46 `>`->`==` and `>`->`>=`, 324:45 `+`->`*` and `+`->`-`.
#[test]
fn the_ouch_face_needs_more_than_twenty_health_gained() {
    let attacked = |health: i32, old: i32| {
        let mut s = fresh();
        s.oldhealth = old;
        let p = StatusInputs {
            health,
            damagecount: 5,
            attacker: Attacker::At([100 << 16, 0]),
            ..StatusInputs::default()
        };
        s.tick(&p).expect("tick");
        (s.faceindex, s.priority, s.facecount)
    };
    // 50 - 10 = 40 > 20: ouch, 16 + 5 (50 / 10 = 5 is not > 20; 16 * 5 and 16 - 5 differ).
    assert_eq!(attacked(50, 10), (21, 7, 34));
    // 30 - 10 = 20 is not > 20: the direction branch, head on (attacker due east of a player
    // facing 0): 24 + ST_RAMPAGEOFFSET (7).
    assert_eq!(attacked(30, 10), (31, 7, 34));
}

/// st_stuff.c:822-850: `diffang > ANG180` and `diffang < ANG45` at exactly the angles. A player
/// at angle 0x4000_0000 with the attacker due south (angle 0xC000_0000, R_PointToAngle2 of a
/// zero `dx` and negative `dy`, r_main.c:320-335) has `diffang` exactly ANG180: not `> ANG180`,
/// so the face turns left (`ST_TURNOFFSET + 1`). A player at 0xA000_0000 has `diffang` exactly
/// ANG45: not `< ANG45`, so the face turns left too. Health 100, old health 100: pain offset 0.
/// Kills st_stuff.rs:330:31 `>`->`>=` and 336:35 `<`->`<=`.
#[test]
fn the_turn_thresholds_are_strict() {
    let facing = |angle: u32| {
        let mut s = fresh();
        s.oldhealth = 100;
        let p = StatusInputs {
            health: 100,
            damagecount: 5,
            angle,
            position: [0, 0],
            attacker: Attacker::At([0, -(100 << 16)]),
            ..StatusInputs::default()
        };
        s.tick(&p).expect("tick");
        s.faceindex
    };
    assert_eq!(facing(0x4000_0000), 4, "diffang == ANG180: turn left");
    assert_eq!(facing(0xa000_0000), 4, "diffang == ANG45: not head on");
    // Just inside ANG45 it is head on, and just past ANG180 the other way (the same inputs'
    // neighbours), so the two exact cases are the boundaries and not constants.
    assert_eq!(facing(0xa000_0001), 7, "diffang == ANG45 - 1: head on");
    assert_eq!(facing(0x3fff_ffff), 3, "diffang == ANG180 + 1: turn right");
}

/// st_stuff.c:798: `if (priority < 8)` guards the attacker branch; the evil grin has set
/// priority 8 (st_stuff.c:790), so an attack on the same tic keeps priority 8, the grin's
/// face counter (only decremented, st_stuff.c:920-921) and the face. (Kills the mutant of that
/// comparison, `<`->`<=`; it was a survivor of the later run, not of the 334cd4b one.)
#[test]
fn an_attack_does_not_replace_the_evil_grin() {
    let mut s = fresh();
    (s.priority, s.facecount, s.faceindex) = (8, 5, 11);
    s.oldhealth = 100;
    let p = StatusInputs {
        health: 100,
        damagecount: 5,
        attacker: Attacker::At([100 << 16, 0]),
        ..StatusInputs::default()
    };
    s.tick(&p).expect("tick");
    assert_eq!((s.priority, s.facecount, s.faceindex), (8, 4, 11));
}

/// st_stuff.c:861-873: `if (priority < 6)` guards the rapid-fire counter; at priority 6 an
/// `attackdown` of false must not reset `lastattackdown` to -1 (kills st_stuff.rs:356:26 `<`->`<=`).
#[test]
fn rapid_fire_is_not_considered_at_priority_six() {
    let mut s = fresh();
    (s.priority, s.lastattackdown, s.facecount) = (6, 30, 5);
    let p = StatusInputs {
        health: 100,
        ..StatusInputs::default()
    };
    s.tick(&p).expect("tick");
    assert_eq!((s.priority, s.lastattackdown), (6, 30));
}

/// st_stuff.c:797: `plyr->attacker != plyr->mo` excludes the player's own damage (splash). A
/// `damage_actor` with the player as its own source is the status bar input `Attacker::Player`
/// (kills d_display.rs:179:20, the match guard `a == mo_id` replaced by `false`, which would make
/// it `Attacker::At` the player's own position).
#[test]
fn own_damage_is_the_player_attacker() {
    let (.., mut sim, _) = load(&item("M5STDIG"));
    let mo = sim
        .original_player(PlayerSlot::One)
        .expect("player")
        .mo
        .expect("mo");
    sim.damage_actor(DamageRequest {
        target: mo,
        inflictor: None,
        source: Some(mo),
        amount: 5,
    })
    .expect("damage");
    let i = status_inputs(&sim).expect("inputs");
    assert!(i.damagecount > 0);
    assert_eq!(i.attacker, Attacker::Player);
}

// ---------------------------------------------------------------------------------------------
// the palettes (st_stuff.c:71-76, 1048)
// ---------------------------------------------------------------------------------------------

/// PLAYPAL is 14 palettes of 768 bytes (st_stuff.c:71-76; `palette*768`, st_stuff.c:1048): the
/// display's copy is exactly the first 14*768 bytes of the lump (kills d_display.rs:210:33
/// `*`->`+` and `*`->`/`, and 251:9 for the three replaced return values).
#[test]
fn playpal_is_the_lump_of_fourteen_palettes() {
    let gfx = std::fs::read(common::reference("m5b_gfx.wad")).expect("gfx");
    let wad = doom_wad::Wad::parse(&gfx, doom_wad::Limits::default()).expect("gfx");
    let mut dir = LumpDirectory::new();
    dir.add_wad(&wad).expect("gfx");
    let lump = dir
        .data(dir.get_num_for_name(b"PLAYPAL").expect("PLAYPAL"))
        .expect("data")
        .to_vec();
    let b = base(load(&item("M5STDIG")), 1);
    assert!(lump.len() >= 14 * 768);
    assert_eq!(b.d.playpal().len(), 14 * 768);
    assert_eq!(b.d.playpal(), &lump[..14 * 768]);
}

// ---------------------------------------------------------------------------------------------
// the display state: value bounds and decoding (docs/STATE_FORMAT.md, "Display state")
// ---------------------------------------------------------------------------------------------

fn good_state() -> (Base, DisplayState) {
    let b = base(load(&item("M5MSG")), 20);
    let st = b.d.state().expect("state");
    (b, st)
}

fn decoded(st: &DisplayState) -> Result<DisplayState, StateDecodeError> {
    DisplayState::decode(&st.encode())
}

fn invalid(st: &DisplayState, name: &'static str) {
    assert_eq!(decoded(st).err(), Some(StateDecodeError::Invalid(name)));
}

/// The bounds of `value_error` at their edges (the ranges st_stuff.c and hu_stuff.c produce):
/// `oldhealth` -1 (st_stuff.c:1268) is admitted and -2 is not; the key boxes are `-1`, `i` or
/// `i + 3` (st_stuff.c:953-959), so `[0, 4, 5]` is admitted and a box 1 of 3 (box 0's skull) is
/// not; a ready ammunition index is below 4 (st_stuff.c:932-935); the message line is at most
/// HU_MAXLINELENGTH (80, hu_lib.h:36) characters and has no NUL. Kills d_display.rs:502:25
/// delete `-`, 508:38 `+`->`*` and `+`->`-`, 511:40 `<`->`<=`, 521:63 `&&`->`||`.
#[test]
fn value_bounds_are_exact() {
    let (b, good) = good_state();
    let mut s = good.clone();
    s.status.oldhealth = -1;
    assert!(decoded(&s).is_ok(), "oldhealth -1");
    s.status.oldhealth = -2;
    invalid(&s, "oldhealth");
    let mut s = good.clone();
    s.status.keyboxes = [0, 4, 5];
    assert!(decoded(&s).is_ok(), "keyboxes 0, 4, 5");
    s.status.keyboxes = [0, 3, 5];
    invalid(&s, "keyboxes");
    s.status.keyboxes = [3, 1, 2];
    assert!(decoded(&s).is_ok(), "keyboxes 3, 1, 2: i + 3 for box 0");
    // The ready index through the encoding's own bound (a byte above 3, `ready > 3` below) and
    // through `restore`, which runs only `value_error`.
    let mut s = good.clone();
    s.status.ready_ammo = Some(3);
    assert!(decoded(&s).is_ok(), "ready_ammo 3");
    let mut d = b.d.clone();
    // `restore` now also checks consistency with the simulation, so index 3 (a value the bound
    // admits) is refused as `Inconsistent`, after the value check; it must not be `Value`.
    assert_eq!(
        d.restore(s.clone(), &b.sim),
        Err(DisplayRestoreError::Inconsistent("keyboxes or ready_ammo")),
        "restore ready_ammo 3 passes the value bound"
    );
    s.status.ready_ammo = Some(4);
    assert_eq!(
        d.restore(s, &b.sim),
        Err(DisplayRestoreError::Value("ready_ammo"))
    );
    // The face histories: -1 (no face yet, st_lib.c:203) is admitted, -2 and one past the dead
    // face (st_stuff.c:889-900, 41 faces + the dead one) are refused.
    let mut s = good.clone();
    s.status.face_old = -1;
    assert!(decoded(&s).is_ok(), "face_old -1");
    s.status.face_old = -2;
    invalid(&s, "face_old");
    s.status.face_old = DEADFACE + 1;
    invalid(&s, "face_old");
    // The line: 81 characters, and a NUL in a short line.
    let mut s = good.clone();
    s.hud.line.text = vec![b'A'; 80];
    assert!(decoded(&s).is_ok(), "80 characters");
    s.hud.line.text = vec![b'A'; 81];
    invalid(&s, "line");
    s.hud.line.text = vec![b'A', 0, b'B'];
    invalid(&s, "line");
}

/// The decoder's own flag checks on the bytes after the DRST block: byte 3-4 is the owned-weapon
/// mask (nine bits, st_stuff.c:381), then `ready` (0..=3 or 0xff for `largeammo`,
/// st_stuff.c:932-935), the status flags (three bits) and the HUD flags (four bits). Each unused
/// bit alone is refused, `ready` 3 and 0xff are admitted and `ready` 4 is refused as flags
/// (kills d_display.rs:632:67 `||`->`&&`, 635:19 `>`->`==` and `>`->`>=`, 635:32 `!=`->`==`,
/// 635:41 `||`->`&&`, 635:61 `||`->`&&`).
#[test]
fn the_decoder_refuses_each_unused_flag_alone() {
    let (_, good) = good_state();
    let bytes = good.encode();
    let at = 12 + u32::from_le_bytes(bytes[8..12].try_into().expect("4")) as usize;
    let flags = |name: &'static str, i: usize, v: u8, e: Option<StateDecodeError>| {
        let mut b = bytes.clone();
        b[at + i] = v;
        assert_eq!(DisplayState::decode(&b).err(), e, "{name}");
    };
    let bad = |n| Some(StateDecodeError::Invalid(n));
    flags("clean", 4, bytes[at + 4], None);
    // screenblocks is 10 or 11 and the flags byte has two bits, each refused alone.
    flags("screenblocks 12", 0, 12, bad("display flags"));
    flags("screenblocks 9", 0, 9, bad("display flags"));
    flags(
        "display flag bit 2",
        1,
        bytes[at + 1] | 4,
        bad("display flags"),
    );
    // owned bit 9 alone (the byte after the mask's low byte).
    flags("owned bit 9", 4, bytes[at + 4] | 2, bad("display flags"));
    // status flag bit 3 alone, HUD flag bit 4 alone.
    flags("status bit 3", 43, bytes[at + 43] | 8, bad("status flags"));
    flags("hud bit 4", 44, bytes[at + 44] | 16, bad("status flags"));
    // ready: 3 admitted (as the index 3), 0xff admitted (no ammunition), 4 refused as flags.
    flags("ready 3", 42, 3, None);
    flags("ready 0xff", 42, 0xff, None);
    flags("ready 4", 42, 4, bad("status flags"));
    flags("ready 0xfe", 42, 0xfe, bad("status flags"));
    // The same through encode: `Some(4)` writes a 4.
    let mut s = good.clone();
    s.status.ready_ammo = Some(4);
    assert_eq!(decoded(&s).err(), bad("status flags"));
    s.status.ready_ammo = None;
    assert!(decoded(&s).is_ok());
}

// ---------------------------------------------------------------------------------------------
// the heads-up line (hu_lib.c:99-139, hu_stuff.c:496-503)
// ---------------------------------------------------------------------------------------------

/// `HUlib_eraseSText` (hu_lib.c:254-267): `needsupdate` is set to 4 only when `laston && !on`,
/// and `HUlib_eraseTextLine` then decrements it (hu_lib.c:170). Kills hu_stuff.rs:230:24 `&&`->`||`.
#[test]
fn erase_sets_needsupdate_only_when_the_message_just_went_off() {
    let run = |laston: bool, on: bool| {
        let mut h = HudState::level_start(PickupMessage::None);
        (h.laston, h.message_on, h.line.needsupdate) = (laston, on, 0);
        h.erase();
        (h.line.needsupdate, h.laston)
    };
    assert_eq!(run(true, false), (3, false), "just went off: 4, then 3");
    assert_eq!(run(false, false), (0, false));
    assert_eq!(run(true, true), (0, true));
    assert_eq!(run(false, true), (0, true));
}

/// The HUD font: the genuine `STCFN` glyphs of `m5b_gfx.wad` with leftoffset 8, so that a glyph
/// the C refuses to draw (`x + w > SCREENWIDTH`, hu_lib.c:119-120) would still lie inside the
/// screen if drawn at `x - 8`; the C advances by the width and ignores the offsets.
fn font_base() -> (Base, Vec<i32>) {
    let loaded = load_edited(&item("M5STDIG"), &|n| {
        let glyph = n.strip_prefix("STCFN").and_then(|d| d.parse::<u32>().ok());
        glyph.filter(|g| (33..=95).contains(g)).map(|_| (4, 8))
    });
    let b = base(loaded, 2);
    let widths = (b'!'..=b'_')
        .map(|c| header(&b.res, &format!("STCFN{c:03}"))[0])
        .collect();
    (b, widths)
}

fn spaces(n: usize) -> Vec<u8> {
    vec![b' '; n]
}

/// hu_lib.c:112-130: a character outside `'!'..='_'` after `toupper` (here `~`) draws nothing
/// and advances 4, exactly like a space; the glyph table is not indexed with it (kills
/// hu_lib.rs:45:27 `&&`->`||`, which indexes `hu_font` with `'~' - '!'` and panics).
#[test]
fn a_character_without_a_glyph_advances_like_a_space() {
    let (b, _) = font_base();
    assert_eq!(b.with_message(b"~"), b.with_message(b" "));
    assert_eq!(b.with_message(b"A~A"), b.with_message(b"A A"));
}

/// hu_lib.c:119-120: `x + w > SCREENWIDTH` breaks BEFORE drawing, so a glyph ending exactly at
/// column 320 is drawn (kills hu_lib.rs:48:26 `>`->`>=` and `>`->`==`). The text is spaces
/// (advance 4) up to `x = 320 - w`, optionally one glyph to fix the residue, then the glyph.
#[test]
fn a_glyph_ending_exactly_at_the_right_edge_is_drawn() {
    let (b, w) = font_base();
    let glyphs = || (b'!'..=b'_').filter(|c| *c != b' ');
    let mut found = None;
    'search: for a in glyphs() {
        let wa = w[usize::from(a - b'!')];
        for extra in std::iter::once(None).chain(glyphs().map(Some)) {
            let wb = extra.map_or(0, |e| w[usize::from(e - b'!')]);
            let rest = 320 - wa - wb;
            if rest >= 8 && rest % 4 == 0 {
                let s = (rest / 4) as usize;
                if s + usize::from(extra.is_some()) < 80 {
                    found = Some((a, extra, s));
                    break 'search;
                }
            }
        }
    }
    let (a, extra, s) = found.expect("a combination of widths that ends at 320");
    let mut prefix = spaces(s);
    prefix.extend(extra);
    let mut text = prefix.clone();
    text.push(a);
    assert_ne!(
        b.with_message(&text),
        b.with_message(&prefix),
        "the last glyph is drawn"
    );
}

/// hu_lib.c:119-120: a glyph that would end past column 320 stops the line and is not drawn,
/// even where its patch (with a leftoffset of 8) would still lie inside the screen (kills
/// hu_lib.rs:48:22 `+`->`-` and 48:26 `>`->`==`). Spaces up to `x` with `x + w` in 321..=324,
/// then the widest glyph.
#[test]
fn a_glyph_ending_past_the_right_edge_ends_the_line() {
    let (b, w) = font_base();
    let (idx, wa) = w
        .iter()
        .enumerate()
        .max_by_key(|(_, v)| **v)
        .map(|(i, v)| (i, *v))
        .expect("glyphs");
    let a = b'!' + idx as u8;
    let s = ((321 - wa + 3) / 4) as usize;
    assert!((321..=324).contains(&(4 * s as i32 + wa)) && (2..80).contains(&s));
    let mut text = spaces(s);
    text.push(a);
    assert_eq!(b.with_message(&text), b.with_message(&spaces(s)));
}
