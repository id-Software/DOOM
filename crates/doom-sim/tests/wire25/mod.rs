//! Shared by `tests/wire25_projection.rs` and `examples/sound_records_run.rs`: one
//! demo run under wire 24 and wire 25 side by side, with the two Rust-against-Rust
//! checks of M5c note §4.4.4 at every capture (tic 0 and every admitted tic):
//!
//! 1. the wire-25 capture with every `map.*` and `events.*` field removed and its
//!    `profile` and `context.schema` set back to wire 24's equals the wire-24
//!    capture so stripped, field for field (the projection M4b used for DSIM40);
//! 2. wire 25's `events()` equal wire 24's with every `sfx_pdiehi` (58) replaced by
//!    `sfx_pldeth` (57) and nothing else, and the pending events are equal.
//!
//! Both runs must also refuse at the same tic with the same error, if at all.
#![allow(dead_code)]

use doom_sim::{sound::SoundRecord, *};

/// `sfx_pldeth` and `sfx_pdiehi` (sounds.h:236-237, counting from `sfx_None` 0).
pub const SFX_PLDETH: u16 = 57;
/// See [`SFX_PLDETH`].
pub const SFX_PDIEHI: u16 = 58;

/// What one paired run observed.
#[derive(Debug, Default)]
pub struct Outcome {
    /// Captures compared (tic 0 included).
    pub captures: usize,
    /// Admitted tics.
    pub tics: usize,
    /// `sfx_pdiehi` events of wire 24 seen as `sfx_pldeth` under wire 25.
    pub screams: usize,
    /// Sound records of wire 25 (starts, placed starts, stops).
    pub starts: usize,
    /// Placed starts among them.
    pub placed: usize,
    /// Stops.
    pub stops: usize,
    /// The refused tic and its error, the same under both wires.
    pub refused: Option<(usize, String)>,
}

fn stripped(s: &Snapshot, wire25: bool) -> Vec<(String, Vec<u8>)> {
    s.fields()
        .filter(|f| !f.path().starts_with("map.") && !f.path().starts_with("events."))
        .map(|f| {
            let value = match (wire25, f.path()) {
                (true, "profile") => {
                    assert_eq!(f.value(), 25_u32.to_le_bytes(), "wire 25's profile");
                    24_u32.to_le_bytes().to_vec()
                }
                (true, "context.schema") => {
                    assert_eq!(f.value(), PLAYER_EPISODE1_V3_CONTEXT_SCHEMA);
                    PLAYER_EPISODE1_V2_CONTEXT_SCHEMA.to_vec()
                }
                _ => f.value().to_vec(),
            };
            (f.path().to_owned(), value)
        })
        .collect()
}

fn check(tic: usize, a: &Simulation, b: &Simulation, out: &mut Outcome) -> Result<(), String> {
    let (sa, sb) = (a.snapshot(), b.snapshot());
    if sa.version() != 40 || sb.version() != 41 {
        return Err(format!(
            "tic {tic}: versions {} and {}",
            sa.version(),
            sb.version()
        ));
    }
    let (fa, fb) = (stripped(&sa, false), stripped(&sb, true));
    if fa != fb {
        let first = fa
            .iter()
            .zip(&fb)
            .find(|(x, y)| x != y)
            .map(|(x, _)| x.0.clone())
            .unwrap_or_else(|| "field count".into());
        return Err(format!("tic {tic}: check 1 differs at {first}"));
    }
    out.captures += 1;
    let (ea, eb) = (a.events(), b.events());
    if ea.len() != eb.len() {
        return Err(format!(
            "tic {tic}: check 2: {} and {} events",
            ea.len(),
            eb.len()
        ));
    }
    for (x, y) in ea.iter().zip(eb) {
        let fixed = match x {
            Event::ActorState(ActorStateEvent::Sound {
                actor,
                sound: SFX_PDIEHI,
            }) => {
                out.screams += 1;
                Event::ActorState(ActorStateEvent::Sound {
                    actor: *actor,
                    sound: SFX_PLDETH,
                })
            }
            other => other.clone(),
        };
        if fixed != *y {
            return Err(format!("tic {tic}: check 2: {x:?} against {y:?}"));
        }
    }
    if a.pending_events() != b.pending_events() {
        return Err(format!("tic {tic}: check 2: pending events differ"));
    }
    if !a.sound_records().is_empty() || !a.pending_sound_records().is_empty() {
        return Err(format!("tic {tic}: wire 24 holds sound records"));
    }
    for r in b.sound_records() {
        match r {
            SoundRecord::Start { at, .. } => {
                out.starts += 1;
                out.placed += usize::from(at.is_some());
            }
            SoundRecord::Stop { .. } => out.stops += 1,
        }
    }
    Ok(())
}

/// Builds one wire's simulation for `map` with the setup calls of every level run.
pub fn build(
    wire25: bool,
    map: doom_map::Map,
    map_number: u8,
    skill: Skill,
) -> Result<Simulation, String> {
    let mut s = if wire25 {
        Simulation::original_player_episode1_v3_map(
            map,
            map_number,
            skill,
            MapContextOptions::default(),
        )
    } else {
        Simulation::original_player_episode1_v2_map(
            map,
            map_number,
            skill,
            MapContextOptions::default(),
        )
    }
    .map_err(|e| format!("load: {e:?}"))?;
    s.spawn_map_things().map_err(|e| format!("things: {e:?}"))?;
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .map_err(|e| format!("weapons: {e:?}"))?;
    s.spawn_level_specials()
        .map_err(|e| format!("specials: {e:?}"))?;
    Ok(s)
}

/// Runs `commands` under both wires, checking every capture; `each` sees the
/// wire-25 simulation after every admitted tic (and once before tic 1 with tic 0).
pub fn run_pair(
    map: &doom_map::Map,
    map_number: u8,
    skill: Skill,
    commands: &[TicCommand],
    mut each: impl FnMut(usize, &Simulation),
) -> Result<Outcome, String> {
    let mut a = build(false, map.clone(), map_number, skill)?;
    let mut b = build(true, map.clone(), map_number, skill)?;
    let mut out = Outcome::default();
    check(0, &a, &b, &mut out)?;
    each(0, &b);
    for (i, c) in commands.iter().enumerate() {
        let tic = i + 1;
        let cmd = [
            *c,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ];
        match (a.advance(cmd), b.advance(cmd)) {
            (Ok(()), Ok(())) => {}
            (Err(x), Err(y)) if x == y => {
                out.refused = Some((tic, format!("{x:?}")));
                break;
            }
            (x, y) => return Err(format!("tic {tic}: wire 24 {x:?}, wire 25 {y:?}")),
        }
        out.tics += 1;
        check(tic, &a, &b, &mut out)?;
        each(tic, &b);
    }
    Ok(out)
}

/// The 64 synthetic items of the genuine M5c sound streams
/// (tests/reference/m5c_sound/LUMP.sound): every item of
/// tools/reference/m5b_reference.py (the M5a rooms, the M4b scripts, the M5b rooms
/// and the bespoke M5CHEAT, whose cheat keys the simulation never sees) and the
/// rooms of tests/reference/m5c_manifest.json, as (lump, WAD, demo) under
/// tests/reference. The map slot is `E1M<n>` of the demo header.
pub const ITEMS: [(&str, &str, &str); 64] = [
    ("M5PEG", "m5a_wads/peg.wad", "m5a_scripts/M5PEG.lmp"),
    ("M5MASK", "m5a_wads/mask.wad", "m5a_scripts/M5MASK.lmp"),
    ("M5PLANE", "m5a_wads/planes.wad", "m5a_scripts/M5PLANE.lmp"),
    ("M5SKY", "m5a_wads/sky.wad", "m5a_scripts/M5SKY.lmp"),
    ("M5LIGHT", "m5a_wads/light.wad", "m5a_scripts/M5LIGHT.lmp"),
    (
        "M5CONTR",
        "m5a_wads/contrast.wad",
        "m5a_scripts/M5CONTR.lmp",
    ),
    ("M5DIST", "m5a_wads/dist.wad", "m5a_scripts/M5DIST.lmp"),
    ("M5SPRIT", "m5a_wads/sprites.wad", "m5a_scripts/M5SPRIT.lmp"),
    ("M5CLIP", "m5a_wads/clip.wad", "m5a_scripts/M5CLIP.lmp"),
    ("M5WEAP", "m5a_wads/weapon.wad", "m5a_scripts/M5WEAP.lmp"),
    ("M5FUZZ", "m5a_wads/fuzz.wad", "m5a_scripts/M5FUZZ.lmp"),
    ("M5AMP", "m5a_wads/amp.wad", "m5a_scripts/M5AMP.lmp"),
    ("M5HOM", "m5a_wads/hom.wad", "m5a_scripts/M5HOM.lmp"),
    ("M5SHORT", "m5a_wads/short.wad", "m5a_scripts/M5SHORT.lmp"),
    ("M5VIEW", "m5a_wads/view.wad", "m5a_scripts/M5VIEW.lmp"),
    ("M5VPLAN", "m5a_wads/vplanes.wad", "m5a_scripts/M5VPLAN.lmp"),
    ("M5DSEG", "m5a_wads/dsegs.wad", "m5a_scripts/M5DSEG.lmp"),
    ("M5VSPR", "m5a_wads/vsprites.wad", "m5a_scripts/M5VSPR.lmp"),
    ("M5OPEN", "m5a_wads/openings.wad", "m5a_scripts/M5OPEN.lmp"),
    ("M5SSEG", "m5a_wads/solidsegs.wad", "m5a_scripts/M5SSEG.lmp"),
    (
        "M5AMPFL",
        "m5a_wads/ampflick.wad",
        "m5a_scripts/M5AMPFL.lmp",
    ),
    ("M5VIEWZ", "m5a_wads/viewz.wad", "m5a_scripts/M5VIEWZ.lmp"),
    ("M5SWTCH", "m5a_wads/switch.wad", "m5a_scripts/M5SWTCH.lmp"),
    ("M5LAND", "m5a_wads/landing.wad", "m5a_scripts/M5LAND.lmp"),
    (
        "M5SEGANG",
        "m5a_wads/segangle.wad",
        "m5a_scripts/M5SEGANG.lmp",
    ),
    (
        "M5MSKPAD",
        "m5a_wads/maskpad.wad",
        "m5a_scripts/M5MSKPAD.lmp",
    ),
    ("TELE1", "m4b_wads/tele.wad", "m4b_scripts/TELE1.lmp"),
    ("TELE2", "m4b_wads/tele.wad", "m4b_scripts/TELE2.lmp"),
    ("TSPLIT", "m4b_wads/telemove.wad", "m4b_scripts/TSPLIT.lmp"),
    ("TSLIDE", "m4b_wads/telemove.wad", "m4b_scripts/TSLIDE.lmp"),
    ("TFRAG1", "m4b_wads/tfrag.wad", "m4b_scripts/TFRAG1.lmp"),
    ("MTELE1", "m4b_wads/montele.wad", "m4b_scripts/MTELE1.lmp"),
    ("SLIDEB", "m4b_wads/slideb.wad", "m4b_scripts/SLIDEB.lmp"),
    ("SLIDEC", "m4b_wads/slidec.wad", "m4b_scripts/SLIDEC.lmp"),
    ("BARON1", "m4b_wads/baron.wad", "m4b_scripts/BARON1.lmp"),
    ("BINF1", "m4b_wads/infight.wad", "m4b_scripts/BINF1.lmp"),
    ("BVB1", "m4b_wads/bvb.wad", "m4b_scripts/BVB1.lmp"),
    ("BOSS1", "m4b_wads/boss8.wad", "m4b_scripts/BOSS1.lmp"),
    ("BOSS2", "m4b_wads/boss8.wad", "m4b_scripts/BOSS2.lmp"),
    ("BOSSM1", "m4b_wads/bossm1.wad", "m4b_scripts/BOSSM1.lmp"),
    ("SPECH8", "m4b_wads/spech8.wad", "m4b_scripts/SPECH8.lmp"),
    ("SPECH9", "m4b_wads/spech9.wad", "m4b_scripts/SPECH9.lmp"),
    ("SKILL2", "m4b_wads/skill.wad", "m4b_scripts/SKILL2.lmp"),
    ("SKILL3", "m4b_wads/skill.wad", "m4b_scripts/SKILL3.lmp"),
    ("TELE2X", "m4b_wads/tele2x.wad", "m4b_scripts/TELE2X.lmp"),
    ("SAWBITE", "m4b_wads/sawbite.wad", "m4b_scripts/SAWBITE.lmp"),
    ("BTUNNEL", "m4b_wads/btunnel.wad", "m4b_scripts/BTUNNEL.lmp"),
    ("TELEZ", "m4b_wads/telez.wad", "m4b_scripts/TELEZ.lmp"),
    ("M5STDIG", "m5b_wads/stdigit.wad", "m5b_scripts/M5STDIG.lmp"),
    ("M5STARM", "m5b_wads/starms.wad", "m5b_scripts/M5STARM.lmp"),
    ("M5EVIL", "m5b_wads/faceevil.wad", "m5b_scripts/M5EVIL.lmp"),
    ("M5OUCH", "m5b_wads/faceouch.wad", "m5b_scripts/M5OUCH.lmp"),
    ("M5LOOK", "m5b_wads/facelook.wad", "m5b_scripts/M5LOOK.lmp"),
    ("M5FACED", "m5b_wads/facedir.wad", "m5b_scripts/M5FACED.lmp"),
    ("M5PALRD", "m5b_wads/palred.wad", "m5b_scripts/M5PALRD.lmp"),
    (
        "M5PALBN",
        "m5b_wads/palbonus.wad",
        "m5b_scripts/M5PALBN.lmp",
    ),
    ("M5PALST", "m5b_wads/palsuit.wad", "m5b_scripts/M5PALST.lmp"),
    ("M5MSG", "m5b_wads/message.wad", "m5b_scripts/M5MSG.lmp"),
    ("M5CHEAT", "m5b_wads/cheats.wad", "m5b_scripts/M5CHEAT.lmp"),
    ("SNDDIST", "m5c_wads/snddist.wad", "m5c_scripts/SNDDIST.lmp"),
    ("SNDBUSY", "m5c_wads/sndbusy.wad", "m5c_scripts/SNDBUSY.lmp"),
    (
        "SNDMONST",
        "m5c_wads/sndmonst.wad",
        "m5c_scripts/SNDMONST.lmp",
    ),
    ("SNDBTN1", "m5c_wads/sndbtn.wad", "m5c_scripts/SNDBTN1.lmp"),
    ("SNDBTN2", "m5c_wads/sndbtn.wad", "m5c_scripts/SNDBTN2.lmp"),
];
