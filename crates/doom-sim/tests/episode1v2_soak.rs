//! Wire 24 (`PlayerMonstersEpisode1V2`, DSIM40) soaks over the sixteen committed M4b
//! synthetic WADs (no IWAD), M4b plan step 9. Rust-only safety properties through the
//! public API; no expectation is a captured digest. Each run loads one WAD at skill 2
//! or 3 under its map number (BOSS8 is map 8, every other WAD map 1), plays the
//! committed script of that WAD (the genuine-engine scenario that reaches its
//! mechanism: teleports, telefrags, Baron attacks, the boss floor, hard-skill
//! things) and then a seeded pseudo-random command stream, and asserts:
//!
//! - every tic is admitted or refused only by a declared wire-24 witness (the
//!   mover family, checked arithmetic, the store-full, work-limit and invariant
//!   witnesses, the exit and reborn tics: M4B_REFERENCE_INVESTIGATION.md §5
//!   decisions I3, I7 and I9 and the M4a witnesses wire 24 inherits), or by the
//!   position check refusing a Baron fireball that tunnelled out of the vertex
//!   rectangle in the tic before, recognised by [`bruiser_tunnel`] and nothing
//!   else; anything else fails the test;
//! - a refused tic keeps the complete canonical state, the RNG and the tic, and
//!   refuses again on a retry (AGENTS.md: recoverable errors preserve state);
//! - after the level's exit tic every later tic is `LevelExited`, as in M4a;
//! - every `ROUND_TRIP` admitted tics, and at the end, the DSIM40 capture decodes
//!   with `decode_episode1_v2`, re-encodes to the same bytes, and is refused by the
//!   DSIM39 reader with `Version(40)` (the frozen-reader rule);
//! - determinism: a second simulation loaded afresh and fed the same commands yields
//!   the same per-tic results, the same digests at the round-trip points and the
//!   same final snapshot.
use doom_sim::{demo::parse_demo_episode1_v2, *};
use doom_types::Fixed;

const ROUND_TRIP: usize = 100;
/// Random commands appended after a script.
const TAIL: usize = 300;

/// (WAD file, map number, the committed script of its mechanism).
const RUNS: [(&str, u8, &str); 18] = [
    ("tele", 1, "TELE1"),
    ("telemove", 1, "TSPLIT"),
    ("tfrag", 1, "TFRAG1"),
    ("montele", 1, "MTELE1"),
    ("slideb", 1, "SLIDEB"),
    ("slidec", 1, "SLIDEC"),
    ("baron", 1, "BARON1"),
    ("infight", 1, "BINF1"),
    ("bvb", 1, "BVB1"),
    ("boss8", 8, "BOSS1"),
    ("bossm1", 1, "BOSSM1"),
    ("spech8", 1, "SPECH8"),
    ("spech9", 1, "SPECH9"),
    ("skill", 1, "SKILL3"),
    ("tele2x", 1, "TELE2X"),
    ("sawbite", 1, "SAWBITE"),
    ("btunnel", 1, "BTUNNEL"),
    ("telez", 1, "TELEZ"),
];

fn reference(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/reference")
            .join(name),
    )
    .expect("reference file")
}

/// xorshift32 command stream: walking, turning, firing, using and weapon changes in
/// bursts, inside the admitted command domain (movement within +-50). A body that is
/// not live gets the empty command (BT_USE alone would request the reborn).
struct Stream(u32);
impl Stream {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn command(&mut self, live: bool) -> TicCommand {
        let r = self.next();
        if !live {
            return TicCommand::default();
        }
        let mut buttons = 0;
        if r % 5 == 0 {
            buttons |= BT_ATTACK;
        }
        if (r >> 3) % 7 == 0 {
            buttons |= BT_USE;
        }
        if (r >> 6) % 23 == 0 {
            buttons |= BT_CHANGE | (((r >> 11) % 4) as u8) << 3;
        }
        TicCommand {
            forward: ((r >> 8) % 101) as i8 - 40,
            side: ((r >> 15) % 61) as i8 - 30,
            angle_turn: (((r >> 21) % 11) as i16 - 5) * 900,
            buttons,
        }
    }
}

/// The closed vertex rectangle (minimum and maximum x and y over all VERTEXES, in
/// fixed point): the room every actor centre is checked against under wire 24.
type Room = [[i32; 2]; 2];

fn room_of(m: &doom_map::Map) -> Room {
    let v = m.vertices();
    let fixed = |f: fn(&doom_map::Vertex) -> i16| -> [i32; 2] {
        let (lo, hi) = (v.iter().map(f).min(), v.iter().map(f).max());
        [lo.unwrap(), hi.unwrap()].map(|u| Fixed::from_map_units(u).bits())
    };
    let (x, y) = (fixed(|v| v.x), fixed(|v| v.y));
    [[x[0], y[0]], [x[1], y[1]]]
}

/// The one declared case of `Monster(Arithmetic)`: the state the refused tic kept
/// holds at least one actor centre outside the vertex rectangle, and every such
/// actor is a Baron fireball (`MT_BRUISERSHOT`, speed 15, radius 6: info.c:1524–1548)
/// that is still a flying missile (`MF_MISSILE`, cleared by `P_ExplodeMissile`,
/// p_mobj.c:101), whose momentum is at most 15 per axis (one unsplit
/// `P_XYMovement` step: p_mobj.c:150–164), whose centre one momentum step back lay
/// inside the rectangle, and whose box now lies wholly beyond a rectangle edge.
/// That is the genuine engine's wall tunnel: `PIT_CheckLine` ignores a line whose
/// bbox the destination box does not strictly overlap (p_map.c:191–195), so a
/// one-sided wall never blocks a step that carries the 12-unit box clean across it
/// (`P_CheckPosition` tests only the destination, p_map.c:374–442); the fireball
/// then keeps flying through the void (observed with the genuine engine on INFIGHT,
/// M4b step 9: a fireball at x = -776.25 still flying at x = -6664 393 tics later).
/// Rust admits the tunnelling tic, matching C, and the position check
/// of the next tic's preflight refuses with the state kept.
fn bruiser_tunnel(s: &Simulation, room: Room) -> bool {
    let ball = ActorTypeId(identity::OriginalActorType::MT_BRUISERSHOT.ordinal());
    let inside = |p: [i32; 2]| (0..2).all(|i| room[0][i] <= p[i] && p[i] <= room[1][i]);
    let mut tunnels = 0;
    for id in s.actor_order() {
        let a = s.actor(id).unwrap();
        let p = a.position().map(Fixed::bits);
        if inside(p) {
            continue;
        }
        let m = a.momentum().map(Fixed::bits);
        let r = a.radius().bits();
        let flying = a
            .life()
            .is_some_and(|l| l.actor_type == ball && l.flags & 0x1_0000 != 0);
        let step = m.iter().all(|v| v.unsigned_abs() <= 15 << 16);
        let before = [0, 1].map(|i| p[i].checked_sub(m[i]));
        let came_from_inside = before[0].zip(before[1]).is_some_and(|b| inside([b.0, b.1]));
        let clear = (0..2).any(|i| {
            p[i].checked_add(r).is_some_and(|e| e <= room[0][i])
                || p[i].checked_sub(r).is_some_and(|e| e >= room[1][i])
        });
        if !(flying && step && came_from_inside && clear) {
            return false;
        }
        tunnels += 1;
    }
    tunnels > 0
}

fn load(name: &str, map: u8, skill: Skill) -> (Simulation, Room) {
    let bytes = reference(&format!("m4b_wads/{name}.wad"));
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
    let m = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default())
        .or_else(|_| doom_map::Map::from_wad(&wad, "E1M8", doom_map::MapLimits::default()))
        .unwrap();
    let room = room_of(&m);
    let mut s =
        Simulation::original_player_episode1_v2_map(m, map, skill, MapContextOptions::default())
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    (s, room)
}

/// The declared refusals of an admitted tic, except the Baron fireball tunnel
/// ([`bruiser_tunnel`]), which needs the kept state.
fn witness(e: &SimError) -> bool {
    matches!(
        e,
        SimError::MapScale(
            MapScaleError::PlatBlocked
                | MapScaleError::StairBlocked
                | MapScaleError::SecondDoor
                | MapScaleError::DonutBusy
                | MapScaleError::MoverFamily
                | MapScaleError::Arithmetic
                | MapScaleError::Escape
                | MapScaleError::Intercepts
                | MapScaleError::SpecialCross
                | MapScaleError::Vertical
        ) | SimError::Store(StoreError::Capacity)
            | SimError::Perception(PerceptionError::WorkLimit)
            | SimError::PlayerBody(PlayerBodyError::Invariant)
            | SimError::LevelExited
            | SimError::PlayerReborn
    )
}

#[derive(Default, Debug)]
struct Totals {
    runs: usize,
    tics: usize,
    refused: usize,
    exits: usize,
    round_trips: usize,
    teleports: usize,
    tunnels: usize,
}

type Trace = Vec<(Result<(), SimError>, Option<u64>)>;

/// One run. With `checks` the per-tic and round-trip properties are asserted.
fn run(
    s: &mut Simulation,
    room: Room,
    commands: &[TicCommand],
    label: &str,
    checks: bool,
    t: &mut Totals,
) -> Trace {
    let limits = DiagnosticLimits::default();
    let mut trace = Trace::new();
    let mut admitted = 0;
    let mut exited = false;
    for (i, &c) in commands.iter().enumerate() {
        let before = checks.then(|| (s.snapshot().canonical_bytes(), s.rng(), s.tic()));
        let tic = [
            c,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ];
        let result = s.advance(tic);
        match result {
            Ok(()) => {
                assert!(!exited, "{label} tic {i}: ran after the exit");
                admitted += 1;
                if checks {
                    t.tics += 1;
                    t.teleports += s
                        .events()
                        .iter()
                        .filter(|e| matches!(e, Event::Mover(MoverEvent::Teleported { .. })))
                        .count();
                    exited |= s
                        .events()
                        .iter()
                        .any(|e| matches!(e, Event::Mover(MoverEvent::Exit { .. })));
                    if admitted % ROUND_TRIP == 0 {
                        round_trip(s, label, limits);
                        t.round_trips += 1;
                    }
                }
                trace.push((
                    result,
                    (admitted % ROUND_TRIP == 0).then(|| s.snapshot().digest()),
                ));
            }
            Err(e) => {
                if let Some(before) = before {
                    let tunnel =
                        e == SimError::Monster(MonsterError::Arithmetic) && bruiser_tunnel(s, room);
                    assert!(
                        witness(&e) || tunnel,
                        "{label} tic {i}: {c:?} refused with {e:?}"
                    );
                    t.tunnels += usize::from(tunnel);
                    assert_eq!(
                        s.snapshot().canonical_bytes(),
                        before.0,
                        "{label} tic {i}: {e:?}"
                    );
                    assert_eq!((s.rng(), s.tic()), (before.1, before.2), "{label} tic {i}");
                    assert_eq!(s.advance(tic), Err(e), "{label} tic {i}: retry");
                    assert_eq!(s.snapshot().canonical_bytes(), before.0, "{label} retry");
                    if exited {
                        assert_eq!(e, SimError::LevelExited, "{label} tic {i}");
                    }
                    t.refused += 1;
                    t.exits += usize::from(e == SimError::LevelExited);
                }
                trace.push((result, Some(s.snapshot().digest())));
                break;
            }
        }
    }
    if checks {
        round_trip(s, label, limits);
        t.round_trips += 1;
    }
    trace.push((Ok(()), Some(s.snapshot().digest())));
    trace
}

fn round_trip(s: &Simulation, label: &str, limits: DiagnosticLimits) {
    let snap = s.snapshot();
    assert_eq!(snap.version(), 40);
    let bytes = snap.canonical_bytes();
    let view = Diagnostic::decode_episode1_v2(&bytes, limits)
        .unwrap_or_else(|e| panic!("{label}: the DSIM40 reader refused a capture: {e:?}"));
    assert_eq!(view.encode(limits).unwrap(), bytes, "{label}");
    assert_eq!(
        Diagnostic::decode_episode1(&bytes, limits).err(),
        Some(DiagnosticError::Version(40)),
        "{label}"
    );
}

#[test]
fn wire_24_soaks_over_the_m4b_wads_hold_the_safety_properties() {
    let mut t = Totals::default();
    for (name, map, script) in RUNS {
        let (header, demo) =
            parse_demo_episode1_v2(&reference(&format!("m4b_scripts/{script}.lmp")), map).unwrap();
        assert!(header.skill == 2 || header.skill == 3);
        for skill in [Skill::Medium, Skill::Hard] {
            for seed in [0x2545_f491_u32, 0x9e37_79b9] {
                let label = format!("{name} {skill:?} seed {seed:#x}");
                let mut stream = Stream(seed ^ u32::from(map));
                let mut commands = demo.commands.clone();
                // The tail is drawn without knowing whether the body is live: a dead
                // body's BT_USE requests the reborn, which is the declared `PlayerReborn`.
                let (mut s, room) = load(name, map, skill);
                let (mut again, _) = load(name, map, skill);
                t.runs += 1;
                let mut tail = Vec::new();
                for _ in 0..TAIL {
                    tail.push(stream.command(true));
                }
                commands.extend(tail);
                let first = run(&mut s, room, &commands, &label, true, &mut t);
                let second = run(&mut again, room, &commands, &label, false, &mut t);
                assert_eq!(first, second, "{label}: determinism");
                assert_eq!(s.snapshot(), again.snapshot(), "{label}: final state");
            }
        }
    }
    println!("wire 24 soaks: {t:?}");
    // The soak is not vacuous: it ran thousands of tics, round-tripped captures and
    // reached the teleport on the teleport WADs.
    assert_eq!(t.runs, 18 * 2 * 2);
    assert!(t.tics > 10_000, "{t:?}");
    assert!(t.round_trips > 100, "{t:?}");
    assert!(t.teleports > 0, "{t:?}");
}
