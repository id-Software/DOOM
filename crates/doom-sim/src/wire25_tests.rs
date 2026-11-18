//! M5c wire 25 (`PlayerMonstersEpisode1V3`, DSIM41), step 3a: unit tests of what
//! wire 25 adds, each from a cited C fact (M5c note §2, §4.4.4, §4.4.5). Rust-only
//! tests: the genuine comparison of the records is M5c step 3b's.
use crate::barrels_replay::{cases, room};
use crate::diagonal_room::Lumps;
use crate::sound::{SoundOrigin, SoundRecord};
use crate::*;

const V2: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V2;
const V3: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V3;
/// sounds.h:236-237, counting from `sfx_None` 0.
const SFX_PLDETH: u16 = 57;
const SFX_PDIEHI: u16 = 58;

fn lump<'a>(l: &'a mut Lumps, name: &str) -> &'a mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == name).unwrap().1
}
fn record(x: i16, y: i16, angle: i16, kind: i16, options: i16) -> Vec<u8> {
    [x, y, angle, kind, options]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
fn with_start(mut l: Lumps, at: usize, position: [i16; 2], angle: i16) -> Lumps {
    let t = lump(&mut l, "THINGS");
    let at = at.min(t.len() / 10);
    t.splice(
        at * 10..at * 10,
        record(position[0], position[1], angle, 1, 7),
    );
    l
}
fn load(l: &Lumps, policy: OriginalExecutionProfile) -> Result<Simulation, SimError> {
    let options = MapContextOptions::default();
    let result = if policy == V3 {
        Simulation::original_player_episode1_v3_map(l.map(), 1, Skill::Medium, options)
    } else {
        Simulation::original_player_episode1_v2_map(l.map(), 1, Skill::Medium, options)
    };
    result.map_err(|e| match e {
        OriginalRegistryError::Runtime(e) => e,
        other => panic!("{other:?}"),
    })
}
fn setup(l: &Lumps, policy: OriginalExecutionProfile) -> Simulation {
    let mut s = load(l, policy).unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
fn base() -> Lumps {
    let c = &cases()[0];
    with_start(room(c.map), 0, c.position, 0)
}
fn body(s: &Simulation) -> ActorId {
    s.original_player.as_ref().unwrap().mo.unwrap()
}

/// The sound starts of a list of events, as (origin, sfx).
fn sounds(events: &[Event]) -> Vec<(Option<ActorId>, u16)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::ActorState(ActorStateEvent::Sound { actor, sound }) => Some((*actor, *sound)),
            _ => None,
        })
        .collect()
}

/// `A_PlayerScream` (p_enemy.c:1994-2007) plays `sfx_pdiehi` only in commercial
/// mode with the health below -50: wire 25 (non-commercial) plays `sfx_pldeth` at
/// any health, with the listener's own body as its unplaced origin; wire 24 keeps
/// its frozen `sfx_pdiehi` below -50. The player is killed by an explicit
/// unsourced damage of 100 - health (no gib: the health stays at or above -100,
/// the player's spawn health, p_inter.c:726-729), and the scream comes with
/// `S_PLAY_DIE2` (info.c) a few tics later.
#[test]
fn hard_death_scream_is_pldeth_under_wire_25_only() {
    for (policy, health, expected) in [
        (V2, -51, SFX_PDIEHI),
        (V2, -50, SFX_PLDETH),
        (V3, -51, SFX_PLDETH),
        (V3, -100, SFX_PLDETH),
        (V3, -50, SFX_PLDETH),
    ] {
        let mut s = setup(&base(), policy);
        let mo = body(&s);
        s.damage_actor(DamageRequest {
            target: mo,
            inflictor: None,
            source: None,
            amount: 100 - health,
        })
        .unwrap();
        let mut heard = Vec::new();
        let mut records = Vec::new();
        for _ in 0..20 {
            s.advance([TicCommand::default(); 4]).unwrap();
            heard.extend(sounds(s.events()));
            records.extend_from_slice(s.sound_records());
        }
        let screams: Vec<_> = heard
            .iter()
            .filter(|(_, sfx)| matches!(*sfx, SFX_PLDETH | SFX_PDIEHI))
            .copied()
            .collect();
        assert_eq!(screams, [(Some(mo), expected)], "{policy:?} {health}");
        if policy == V3 {
            assert!(records.contains(&SoundRecord::Start {
                sfx: SFX_PLDETH,
                origin: SoundOrigin::Actor(mo),
                at: None,
            }));
        } else {
            assert!(records.is_empty());
        }
    }
}
