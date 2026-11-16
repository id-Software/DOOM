//! The E1M1 policy's static decorations (M3h9 note §2.5–§2.7 and §5.1, decision
//! E2): the explicit seven-type list, the static predicate every listed type
//! satisfies, the per-map bound, the live-actor witness and the linked-thing
//! budget. Admission is the list; the predicate records why no body beyond the
//! existing ones is needed. Every failure is a field-less error.
use crate::identity::{OriginalActorType, OriginalState};
use crate::original_player::{MF_NOGRAVITY, MF_SHOOTABLE, MF_SOLID};
use crate::world_pickups::{MF_AMBUSH, MF_COUNTITEM, MF_COUNTKILL, MF_SPECIAL};
use crate::{
    Actor, ActorError, ActorTypeId, MAX_PLAYER_MONSTERS, MAX_WORLD_PICKUPS,
    OriginalExecutionProfile, PlayerBodyError, SimError, Solidity,
};
use doom_types::Fixed;

/// p_mobj.h `MF_NOSECTOR`: not linked to the sector thing list.
pub(crate) const MF_NOSECTOR: u32 = 8;
/// p_mobj.h `MF_NOBLOCKMAP`: not linked to the BLOCKMAP.
pub(crate) const MF_NOBLOCKMAP: u32 = 16;
/// p_mobj.h `MF_SPAWNCEILING`: `P_SpawnMapThing` spawns it at `ONCEILINGZ`.
pub(crate) const MF_SPAWNCEILING: u32 = 256;

/// The seven static decoration types of E1M1 (info.c; M3h9 note §2.5, decision
/// E2), by `mobjinfo` entry:
///
/// | DoomEd | type (info.c line) | spawn state (line) | radius / height | flags (line) |
/// | --- | --- | --- | --- | --- |
/// | 2028 | `MT_MISC31`, floor lamp (3215) | `S_COLU` (1022) | 16 / 16 | `MF_SOLID` (3236) |
/// | 48 | `MT_MISC48`, tall techno pillar (3657) | `S_TECHPILLAR` (1052) | 16 / 16 | `MF_SOLID` (3678) |
/// | 35 | `MT_MISC50`, candelabra (3709) | `S_CANDELABRA` (1048) | 16 / 16 | `MF_SOLID` (3730) |
/// | 24 | `MT_MISC71`, pool of blood and flesh (4255) | `S_GIBS` (1031) | 20 / 16 | 0 (4276) |
/// | 15 | `MT_MISC62`, dead player (4021) | `S_PLAY_DIE7` (300) | 20 / 16 | 0 (4042) |
/// | 10 | `MT_MISC68`, bloody mess (4177) | `S_PLAY_XDIE9` (309) | 20 / 16 | 0 (4198) |
/// | 12 | `MT_MISC69`, bloody mess (4203) | `S_PLAY_XDIE9` (309) | 20 / 16 | 0 (4224) |
///
/// Every spawn state has `tics` −1 and no action, every spawn health is 1000 and
/// none is `MF_SHOOTABLE`, `MF_SPECIAL`, `MF_COUNTKILL`, `MF_COUNTITEM`,
/// `MF_NOBLOCKMAP`, `MF_NOSECTOR`, `MF_SPAWNCEILING` or `MF_NOGRAVITY`
/// (`static_decoration`). Admission is this list under wire 20 and its barrels
/// superset (wire 21, which keeps the list unchanged); a later
/// policy that admits every static type (decision E2 option ii) is a list change
/// with its own evidence (hanging corpses are `MF_SPAWNCEILING|MF_NOGRAVITY`, a
/// new height path).
pub const E1M1_DECORATIONS: [OriginalActorType; 7] = [
    OriginalActorType::MT_MISC31,
    OriginalActorType::MT_MISC48,
    OriginalActorType::MT_MISC50,
    OriginalActorType::MT_MISC71,
    OriginalActorType::MT_MISC62,
    OriginalActorType::MT_MISC68,
    OriginalActorType::MT_MISC69,
];
/// The static things the Episode 1 policy admits beyond [`E1M1_DECORATIONS`]
/// (M4a plan §E.6): the candle (`MT_MISC49`, DoomEd 34, info.c:3682–3706: spawn
/// state `S_CANDLESTIK` with `tics` −1, flags 0, radius 20) and the tall red
/// firestick (`MT_MISC43`, DoomEd 46, info.c:3526–3550: `MF_SOLID`, radius 16, the
/// four-state `S_REDTORCH` loop of 4 tics each with no action, info.c:1070–1073).
/// The firestick is the one animated decoration: the general thinker countdown
/// cycles it (`P_MobjThinker`, p_mobj.c:441–449) and nothing else ever changes it.
pub const EPISODE1_DECORATIONS: [OriginalActorType; 2] =
    [OriginalActorType::MT_MISC49, OriginalActorType::MT_MISC43];
/// The most decorations one map may spawn under the E1M1 policy (E1M1 spawns 27 on
/// every skill, M3h9 note §3); the next is `MapSpawnError::DecorationLimit` and the
/// batch spawns nothing.
pub const MAX_DECORATIONS: usize = 32;
/// The linked things `N` of the E1M1 policy (M3h9 note §5.3): the player, the
/// monsters, the pickups, the drops and the decorations, every one BLOCKMAP- and
/// sector-linked (none of the seven types is `MF_NOBLOCKMAP` or `MF_NOSECTOR`).
/// The §5.3 budgets this value feeds, with the values of this policy
/// (`D` = decorations ≤ 32, `K` = monsters ≤ 8, `P` = pickups ≤ 16, `R` = drops
/// ≤ 8, `k` = Imps ≤ `K`):
///
/// | Budget | Formula | Value | Store or certificate |
/// | --- | --- | --- | --- |
/// | Linked things `N` | `1 + K + P + R + D` | 65 (was 33) | `P_ChangeSector` work, intercept reasoning |
/// | Live actors | `1 + K + P + 26k + (K−k) + 8(K−k) + R + D` | ≤ 273 at `k = 8` | `MAP_SCALE_ACTOR_SLOTS` 320 |
/// | Spawns in one pass | `39 − 3k` | ≤ 39 | 273 + 15 = 288 ≤ 320 at `k = 8`; 137 + 39 = 176 at `k = 0`; crude 273 + 39 = 312 ≤ 320 |
/// | Thinker visits per pass | `320 + L + 2M` | unchanged | the visit budget (decorations are actors inside the 320) |
/// | Sector-thinker slots | `L + 2M` | unchanged | `sector_thinkers.slot_count` |
/// | `P_ChangeSector` per tic | `Σ_movers 4·(B(s) + N·Q)` | `N` 33 → 65 | re-evaluated, not re-proved |
/// | `spechit` | ≤ 8 per position check, every radius < 64 | radii 16, 20 | rule 6 unchanged |
/// | Hitscan intercepts | lines of every cell along the trace + things in those cells, against 128 | E1M1 estimate 28 + 65 = 93 (28 is E1M1's 2 × 2 slide window; a 2,048-unit hitscan visits every cell along its line) | not a bound: the intercept witness (refuse the tic) stays reachable on admitted maps, unchanged |
/// | Slide intercepts | window lines ≤ 128 | 28 | unchanged (lines only) |
/// | Sight queries | 64 per operation, monsters only | unchanged | decorations are never queried |
/// | Scrollers | ≤ 64 | 8 | rule 5 unchanged |
/// | Blockmap links, sector thing lists | `N` per cell at most | 65 | linked lists, no fixed-capacity store |
/// | `MAX_BONUS` | `6 × P` | 96 | unchanged |
///
/// No fixed-capacity store grows and no arithmetic certificate has a roster term,
/// so the older policies' private `MAX_LINKED_THINGS` (33) and its 95-linedef
/// static assert are left as they are: this constant is the E1M1 value alone.
pub const E1M1_LINKED_THINGS: usize =
    1 + MAX_PLAYER_MONSTERS + MAX_WORLD_PICKUPS + MAX_PLAYER_MONSTERS + MAX_DECORATIONS;
const _: () = assert!(E1M1_LINKED_THINGS == 65);

/// The most exploding barrels (`MT_BARREL`, DoomEd 2035) one map may spawn under
/// the barrels policy (decision F2: E1M1 spawns 6 at medium).
/// The M3h9b note §5.3 budgets with the values of this policy (`K` = monsters ≤ 8,
/// `k` = Imps ≤ `K`, `P` = pickups ≤ 64, `R` = drops ≤ 8, `D` = decorations ≤ 32,
/// `B` = barrels ≤ 16, `L`/`M` light and mover sectors):
///
/// | Budget | Formula | Value | Store or certificate |
/// | --- | --- | --- | --- |
/// | Linked things `N` | `1 + K + P + R + D + B` | 129 (was 65) | `P_ChangeSector` work, intercept reasoning |
/// | Live actors | `1 + K + P + 26k + (K−k) + 8(K−k) + R + D + B` | ≤ 337 at `k = 8` (201 at `k = 0`) | `BARRELS_ACTOR_SLOTS` 384 |
/// | Spawns in one pass | `39 − 3k` | ≤ 39 | 337 + 15 = 352 at `k = 8`; 201 + 39 = 240 at `k = 0`; crude 337 + 39 = 376 ≤ 384 |
/// | Thinker visits per pass | `384 + L + 2M` | | the visit budget |
/// | `P_ChangeSector` per tic | `Σ_movers 4·(B(s) + N·Q)` | `N` 65 → 129 | re-evaluated, not re-proved |
/// | `spechit` | ≤ 8 per position check, every radius < 64 | radii 10, 16, 20 | rule 6 unchanged |
/// | Hitscan intercepts | lines (with multiplicity) + things of the ≤ 18 cells of a trace | E1M1 ≤ 103 against 128 | not a bound: the intercept witness stays (decision F7) |
/// | Explosions per tic | ≤ `B` | 16 | |
/// | Sight queries per explosion | ≤ `24 × (1 + K)`, one revisit pass per woken monster | 216 = `EXPLOSION_SIGHT_QUERIES` | per-explosion control (decision F6) |
/// | `P_DamageMobj` calls per tic | ≤ `B·(K + B)` + the attack bodies | ≤ 384 | each a bounded body |
/// | `MAX_BONUS` | `6 × P` | 384 | pickup state range |
/// | Barrel removals per tic | ≤ `B` | 16 | retirement list |
///
/// The only grown stores are the actor slots (384, encoded as `store.max_slots`)
/// and the per-policy counters; the arithmetic certificate has no roster term.
pub const MAX_BARRELS: usize = 16;
/// The linked things `N` of the barrels policy (M3h9b note §5.3):
/// `1 + K + P + R + D + B` = 1 + 8 + 64 + 8 + 32 + 16.
pub const BARRELS_LINKED_THINGS: usize = 1
    + MAX_PLAYER_MONSTERS
    + crate::world_pickups::BARRELS_MAX_PICKUPS
    + MAX_PLAYER_MONSTERS
    + MAX_DECORATIONS
    + MAX_BARRELS;
const _: () = assert!(BARRELS_LINKED_THINGS == 129);

/// The listed decoration type of `ty`, policy-free (the policy gate is
/// [`is_decoration`]).
pub(crate) fn decoration(ty: ActorTypeId) -> Option<OriginalActorType> {
    E1M1_DECORATIONS.into_iter().find(|d| d.ordinal() == ty.0)
}
/// [`decoration`] widened by the Episode 1 policy's [`EPISODE1_DECORATIONS`],
/// policy-free (the policy gate is [`is_decoration`]).
pub(crate) fn listed_decoration(ty: ActorTypeId) -> Option<OriginalActorType> {
    decoration(ty).or_else(|| {
        EPISODE1_DECORATIONS
            .into_iter()
            .find(|d| d.ordinal() == ty.0)
    })
}
/// Whether `ty` is an admitted decoration under `policy`: one of
/// [`E1M1_DECORATIONS`] and the policy has `has_e1m1`, or one of
/// [`EPISODE1_DECORATIONS`] and the policy has `has_episode1`.
pub(crate) fn is_decoration(policy: OriginalExecutionProfile, ty: ActorTypeId) -> bool {
    policy.has_e1m1()
        && (E1M1_DECORATIONS.iter().any(|d| d.ordinal() == ty.0)
            || (policy.has_episode1() && EPISODE1_DECORATIONS.iter().any(|d| d.ordinal() == ty.0)))
}
/// The states a decoration may hold: its spawn state, or for an animated one
/// ([`animated_decoration`]) every state of its spawn loop.
pub(crate) fn decoration_states(ty: OriginalActorType) -> DecorationStates {
    let root = ty.descriptor().states.spawn;
    let mut states = DecorationStates {
        states: [root; 8],
        len: 1,
    };
    let mut state = root.descriptor().next;
    while state != root && state != OriginalState::S_NULL && states.len < 8 {
        states.states[states.len] = state;
        states.len += 1;
        state = state.descriptor().next;
    }
    states
}
/// The states of [`decoration_states`], at most eight in spawn-loop order, held
/// inline instead of in a `Vec`; read as a slice of its first `len` entries.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DecorationStates {
    states: [OriginalState; 8],
    len: usize,
}
impl std::ops::Deref for DecorationStates {
    type Target = [OriginalState];
    fn deref(&self) -> &[OriginalState] {
        &self.states[..self.len]
    }
}
/// Whether a decoration in `state` may hold the countdown `tics`: −1 in a static
/// spawn state, `1..=` the state's tics in an animated loop.
pub(crate) fn decoration_tics(state: OriginalState, tics: i32) -> bool {
    let limit = state.descriptor().tics;
    if limit == -1 {
        tics == -1
    } else {
        (1..=limit).contains(&tics)
    }
}
/// The static predicate every listed type satisfies (M3h9 note §2.5, decision E2):
/// a DoomEd number, spawn state with `tics` −1 and no action (so `P_MobjThinker`
/// never cycles it, p_mobj.c:441–449), flags 0 or `MF_SOLID` alone (so
/// `PIT_CheckThing` either passes it at 258–259 or blocks it at 345 and the missile
/// branch at 304–305, and `PIT_ChangeSector` returns at 1290–1294), none of
/// `MF_SHOOTABLE` (never damaged, thrust or gibbed), `MF_SPECIAL` (never touched),
/// `MF_COUNTKILL`/`MF_COUNTITEM` (no counter), `MF_NOBLOCKMAP`/`MF_NOSECTOR` (linked
/// like any thing) or `MF_SPAWNCEILING`/`MF_NOGRAVITY` (spawned at `ONFLOORZ`,
/// p_mobj.c:770–777), spawn health 1000, radius 16 or 20 and height 16.
pub(crate) fn static_decoration(ty: OriginalActorType) -> bool {
    let d = ty.descriptor();
    let s = d.states.spawn.descriptor();
    d.doomed_number > 0
        && s.tics == -1
        && s.action.is_none()
        && d.flags & !MF_SOLID == 0
        && d.flags
            & (MF_SHOOTABLE
                | MF_SPECIAL
                | MF_COUNTKILL
                | MF_COUNTITEM
                | MF_NOBLOCKMAP
                | MF_NOSECTOR
                | MF_SPAWNCEILING
                | MF_NOGRAVITY)
            == 0
        && d.spawn_health == 1000
        && (d.radius == 16 * 65536 || d.radius == 20 * 65536)
        && d.height == 16 * 65536
}
/// The animated variant of [`static_decoration`] (the Episode 1 firestick, M4a
/// plan §E.6): the same flags, health, radius and height, but a spawn loop of at
/// most eight timed states with the spawn sprite and no action that closes on the
/// spawn state.
pub(crate) fn animated_decoration(ty: OriginalActorType) -> bool {
    let d = ty.descriptor();
    let root = d.states.spawn;
    let s = root.descriptor();
    let states = decoration_states(ty);
    d.doomed_number > 0
        && states.len() > 1
        && states.len() < 8
        && states.last().is_some_and(|l| l.descriptor().next == root)
        && states.iter().all(|state| {
            let x = state.descriptor();
            x.tics > 0 && x.action.is_none() && x.sprite == s.sprite
        })
        && d.flags & !MF_SOLID == 0
        && d.spawn_health == 1000
        && (d.radius == 16 * 65536 || d.radius == 20 * 65536)
        && d.height == 16 * 65536
}
/// `ty` is a listed decoration whose compile-time definition satisfies
/// [`static_decoration`]; computed once per type ordinal.
pub(crate) fn validate_definition(ty: ActorTypeId) -> Result<(), ActorError> {
    static MEMO: [std::sync::OnceLock<Result<(), ActorError>>; 256] =
        [const { std::sync::OnceLock::new() }; 256];
    let compute = || {
        decoration(ty)
            .filter(|d| static_decoration(*d))
            .map(|_| ())
            .ok_or(ActorError::InvalidType)
    };
    match MEMO.get(usize::from(ty.0)) {
        Some(cell) => *cell.get_or_init(compute),
        None => compute(),
    }
}
/// [`validate_definition`] over [`listed_decoration`]: an Episode 1 decoration
/// satisfies [`static_decoration`] (the candle) or [`animated_decoration`] (the
/// firestick).
pub(crate) fn validate_listed_definition(ty: ActorTypeId) -> Result<(), ActorError> {
    if decoration(ty).is_some() {
        return validate_definition(ty);
    }
    listed_decoration(ty)
        .filter(|d| static_decoration(*d) || animated_decoration(*d))
        .map(|_| ())
        .ok_or(ActorError::InvalidType)
}
/// `mobj->angle = ANG45 * (mthing->angle/45)` (p_mobj.c:794): the THING's `short`
/// angle divided by 45 with C's truncation toward zero, times `ANG45`
/// (0x20000000, tables.h) in unsigned 32-bit arithmetic (the `int` quotient is
/// converted to `unsigned` and the product wraps).
pub(crate) fn spawn_angle(thing_angle: i16) -> u32 {
    ((i32::from(thing_angle) / 45) as u32).wrapping_mul(0x2000_0000)
}
/// `MF_AMBUSH` is set on a spawned thing exactly when its THING record carries
/// `MTF_AMBUSH` (8): `if (mthing->options & MTF_AMBUSH) mobj->flags |= MF_AMBUSH;`
/// (p_mobj.c:795–796), and nothing else sets or clears it on a decoration.
pub(crate) fn spawn_ambush(options: u16) -> bool {
    options & 8 != 0
}
/// The live witness of a spawned decoration (`P_SpawnMapThing` p_mobj.c:708–797
/// through `P_SpawnMobj` 480–534, then nothing per tic, §2.5): its listed type and
/// definition, the spawn state with the state's sprite and frame and `tics` −1,
/// spawn health 1000, the type's reaction time, no target, threshold, `movedir`
/// or `movecount`, `lastlook` from one `P_Random() % 4`, the type's radius and
/// height, no momentum, `z == floorz` (spawned at `ONFLOORZ`, kept there by
/// `P_ThingHeightClip`, §2.7), the type's flags with `MF_AMBUSH` added exactly when
/// the THING record carries `MTF_AMBUSH` (p_mobj.c:795–796), no pending death,
/// solidity from `MF_SOLID`, and the THING record it was spawned from at its own
/// position (the spawnpoint, 785) with the angle `ANG45 * (angle/45)` (794).
pub(crate) fn validate_decoration(
    room: Option<crate::player_map_scale::RoomBounds>,
    a: Actor,
) -> Result<(), SimError> {
    let l = a.life().ok_or(ActorError::InvalidType)?;
    let ty = listed_decoration(l.actor_type).ok_or(ActorError::InvalidType)?;
    validate_listed_definition(l.actor_type)?;
    let d = ty.descriptor();
    let state = l
        .state
        .and_then(|v| OriginalState::try_from(v.0).ok())
        .ok_or(ActorError::InvalidState)?;
    let s = state.descriptor();
    if !decoration_states(ty).contains(&state)
        || l.sprite != s.sprite.ordinal()
        || l.frame != s.frame
        || !decoration_tics(state, l.tics)
        || l.health != d.spawn_health
        || l.reaction_time != d.reaction_time
        || l.target.is_some()
        || l.threshold != 0
        || l.movedir != 0
        || l.movecount != 0
        || l.death_requested
        || a.lastlook() > 3
        || a.radius() != Fixed::from_bits(d.radius)
        || a.height() != Fixed::from_bits(d.height)
        || a.momentum() != [Fixed::ZERO; 2]
        || a.momz != Fixed::ZERO
        || a.z != a.floor
        || l.flags & !(d.flags | MF_AMBUSH) != 0
        || l.flags & d.flags != d.flags
        || (a.solidity == Solidity::Solid) != (d.flags & MF_SOLID != 0)
        || !crate::player_map_scale::position_admitted(room, a.position(), 978)
        || l.spawnpoint.is_none_or(|p| {
            i32::from(p.kind) != d.doomed_number
                || a.position()
                    != [
                        Fixed::from_map_units(p.position.x),
                        Fixed::from_map_units(p.position.y),
                    ]
                || a.spec.angle != spawn_angle(p.angle)
                || (l.flags & MF_AMBUSH != 0) != spawn_ambush(p.options)
        })
    {
        return Err(PlayerBodyError::Invariant.into());
    }
    Ok(())
}
/// Whether `state` is a state the listed decoration `ty` may hold: its spawn state
/// (`tics` −1, no successor entered, never killed), or a state of the Episode 1
/// firestick's spawn loop.
pub(crate) fn spawn_state(ty: ActorTypeId, state: OriginalState) -> bool {
    listed_decoration(ty).is_some_and(|d| decoration_states(d).contains(&state))
}

#[cfg(test)]
mod decoration_states_tests {
    use super::*;

    /// Pins the eight-state bound of `decoration_states` (`states.len < 8`). A type whose spawn
    /// chain runs on past eight states without returning to its root or `S_NULL` must stop at
    /// exactly eight (the inline array's size) and not index past it. The static data has such
    /// chains (the chain of the type of ordinal 4 is 30 states, of ordinal 39 is 12, of ordinal
    /// 29 exactly 8), so the bound is reachable by the function's contract. Below the public API: the function is
    /// `pub(crate)` and takes any original type, while the public API only reaches it through
    /// the admitted decoration roles.
    #[test]
    fn a_spawn_chain_longer_than_eight_states_holds_exactly_eight() {
        for ordinal in [4_u16, 39] {
            let ty = OriginalActorType::try_from(ordinal).unwrap();
            let chain = chain_length(ty);
            assert!(chain > 8, "ordinal {ordinal}: chain of {chain} states");
            let states = decoration_states(ty);
            assert_eq!(states.len(), 8, "ordinal {ordinal}");
            assert_eq!(states[0], ty.descriptor().states.spawn);
            for pair in states.windows(2) {
                assert_eq!(pair[0].descriptor().next, pair[1]);
            }
        }
        // A chain of exactly eight states fits whole.
        let eight = OriginalActorType::try_from(29_u16).unwrap();
        assert_eq!(chain_length(eight), 8);
        assert_eq!(decoration_states(eight).len(), 8);
    }

    /// The spawn chain's length counted without the bound, from the static data.
    fn chain_length(ty: OriginalActorType) -> usize {
        let root = ty.descriptor().states.spawn;
        let mut n = 1;
        let mut s = root.descriptor().next;
        while s != root && s != OriginalState::S_NULL {
            n += 1;
            s = s.descriptor().next;
        }
        n
    }
}
