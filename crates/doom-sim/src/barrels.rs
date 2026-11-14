//! The exploding barrel of the barrels policy (wire 21; M3h9b note §2 and §5,
//! decisions F4, F5, F6 and F9): its role predicate, the compile-time definition
//! check, the live witness every tic preflight reads, and `A_Explode` with
//! `P_RadiusAttack` and `PIT_RadiusAttack` (p_enemy.c:1598–1601, p_map.c:1164–1233).
//!
//! A barrel is neither a roster monster nor an item: it is spawned by
//! `P_SpawnMapThing` like any THING, counted against [`crate::MAX_BARRELS`] alone,
//! never counted for kills or items, never a drop source and never a monster's
//! target. Everything it does goes through existing bodies: `P_DamageMobj` (the
//! shared [`crate::combat::DamagePlan`] and `LifeHost::apply_damage`), the
//! thinker's movement, the state machine and `PIT_ChangeSector`. What is new is
//! the role gates and the radius attack below. An explosion adds no event kind
//! (decision F9): the `A_Explode` action record, then per candidate reaching
//! `P_CheckSight` the existing sight record, then each victim's damage records.
use crate::combat::{DamagePlan, DamageRequest};
use crate::identity::{OriginalActorType, OriginalState};
use crate::lifecycle::{ActorAccess, LifeHost};
use crate::original_player::{
    BASETHRESHOLD, MF_CORPSE, MF_DROPOFF, MF_NOBLOOD, MF_SHOOTABLE, MF_SOLID,
};
use crate::world_pickups::MF_AMBUSH;
use crate::{
    Actor, ActorError, ActorId, ActorTypeId, CombatError, MAX_BARRELS, MAX_PLAYER_MONSTERS,
    OriginalExecutionProfile, PerceptionError, PlayerBodyError, SimError, Solidity,
};
use doom_types::Fixed;

/// `A_Explode`'s damage (p_enemy.c:1600: `P_RadiusAttack (thingy, thingy->target, 128)`).
pub(crate) const BOMB_DAMAGE: i32 = 128;
/// p_local.h:51 `#define MAXRADIUS 32*FRACUNIT`, unparenthesized.
const MAXRADIUS: i32 = 32 << 16;
/// p_local.h:40 `MAPBLOCKSHIFT` (`FRACBITS + 7`): one BLOCKMAP cell is 128 units.
const MAPBLOCKSHIFT: u32 = 23;
/// p_map.c:1221 `dist = (damage+MAXRADIUS)<<FRACBITS;` as the shipped executable
/// computes it (decision F5, a declared reproduced quirk). The macro is not
/// parenthesized, so the expression is `(128 + 32*65536) << 16`: `0x0020_0080 <<
/// 16` is `0x20_0080_0000`, whose low 32 bits are `0x0080_0000`, `128 << 16`. The
/// `MAXRADIUS` term shifts out entirely (`32 << 32 ≡ 0 mod 2³²`) and the block
/// window is the spot ± 128 units: always exactly the 3 × 3 cells around the
/// spot's cell (`⌊(v + 128)/128⌋ − ⌊(v − 128)/128⌋ = 2` for every `v`), so a thing
/// in a fourth cell is never visited even when its distance minus its radius is
/// below 128. ISO C leaves the signed overflow undefined; this is the
/// two's-complement wrap of the original build, computed in `u32`.
pub(crate) const BLAST_WINDOW: i32 = (BOMB_DAMAGE as u32)
    .wrapping_add(MAXRADIUS as u32)
    .wrapping_shl(16) as i32;
const _: () = assert!(BLAST_WINDOW == 128 << 16);
/// The distinct shootable candidates one explosion's 3 × 3 window can hold
/// (decision F6): at most the player, the `K` monsters and the other `B − 1`
/// barrels (`1 + K + B − 1` = 24).
pub(crate) const EXPLOSION_SIGHT_CANDIDATES: usize = 1 + MAX_PLAYER_MONSTERS + MAX_BARRELS - 1;
const _: () = assert!(EXPLOSION_SIGHT_CANDIDATES == 24);
/// The sight queries one explosion may make (decision F6, widened for mid-walk
/// relinks): `24 × (1 + K)` = 216. A walk without relinks queries each distinct
/// candidate at most once, at most 24. A victim whose
/// wound runs its seestate `A_Chase` (p_inter.c:912–914) is unlinked and relinked
/// at the head of its cell (p_maputl.c:436–442), and the walk then follows its new
/// `bnext` (p_maputl.c:529–535), so things already walked can be met and queried
/// again: one relink makes the remaining walk revisit each candidate at most once
/// more, at most 24 queries. Only a monster in its spawn state with threshold 0
/// enters its seestate from `P_DamageMobj`, and afterwards it is in neither, so
/// each of the `K` monsters relinks at most once per explosion (the player and
/// barrels never relink inside the walk; a kill or a pain state does not move).
/// Hence at most `24 · (1 + K)` queries and work at most that many times one
/// query's bound `S`. Separate from the 64-query per-tic control the monsters
/// share; exhaustion is `PerceptionError::WorkLimit` and refuses the tic, which
/// this argument makes unreachable on an admitted map.
pub const EXPLOSION_SIGHT_QUERIES: usize = EXPLOSION_SIGHT_CANDIDATES * (1 + MAX_PLAYER_MONSTERS);
const _: () = assert!(EXPLOSION_SIGHT_QUERIES == 216);

/// Whether `ty` is the barrel under `policy`: `MT_BARREL` (DoomEd 2035) and the
/// policy has `has_barrels` (wire 21 only).
pub(crate) fn is_barrel(policy: OriginalExecutionProfile, ty: ActorTypeId) -> bool {
    policy.has_barrels() && is_barrel_type(ty)
}
/// `MT_BARREL`, policy-free (the policy gate is [`is_barrel`]).
pub(crate) const fn is_barrel_type(ty: ActorTypeId) -> bool {
    ty.0 == OriginalActorType::MT_BARREL.ordinal()
}
/// The live states of a barrel (`S_BAR1`/`S_BAR2`, info.c:942–943).
pub(crate) fn live_state(s: OriginalState) -> bool {
    matches!(s, OriginalState::S_BAR1 | OriginalState::S_BAR2)
}
/// The death chain `S_BEXP..S_BEXP5` (info.c:944–948), entered by `P_KillMobj`.
pub(crate) fn death_state(s: OriginalState) -> bool {
    matches!(
        s,
        OriginalState::S_BEXP
            | OriginalState::S_BEXP2
            | OriginalState::S_BEXP3
            | OriginalState::S_BEXP4
            | OriginalState::S_BEXP5
    )
}
/// Every state a barrel holds through its role (`S_GIBS` after a crunch is
/// type-keyed by the gibs rule, never by the role).
pub(crate) fn state_allowed(s: OriginalState) -> bool {
    live_state(s) || death_state(s)
}

/// The info.c record the barrel's admission relies on (info.c:1888–1912 and the
/// states 942–948; M3h9b note §2.1): DoomEd 2035, spawn `S_BAR1`, health 20, no
/// see or pain state, painchance 0, reaction time 8, death `S_BEXP`, no extreme
/// death, radius 10, height 42, mass 100, flags `MF_SOLID|MF_SHOOTABLE|MF_NOBLOOD`,
/// and the seven states with exactly the tics, actions and successors listed.
pub(crate) fn source_definition() -> bool {
    use OriginalState as S;
    let d = OriginalActorType::MT_BARREL.descriptor();
    let chain = [
        (S::S_BAR1, 6, None, S::S_BAR2),
        (S::S_BAR2, 6, None, S::S_BAR1),
        (S::S_BEXP, 5, None, S::S_BEXP2),
        (S::S_BEXP2, 5, Some("A_Scream"), S::S_BEXP3),
        (S::S_BEXP3, 5, None, S::S_BEXP4),
        (S::S_BEXP4, 10, Some("A_Explode"), S::S_BEXP5),
        (S::S_BEXP5, 10, None, S::S_NULL),
    ];
    d.doomed_number == 2035
        && d.states.spawn == S::S_BAR1
        && d.states.see == S::S_NULL
        && d.states.pain == S::S_NULL
        && d.states.melee == S::S_NULL
        && d.states.missile == S::S_NULL
        && d.states.death == S::S_BEXP
        && d.states.extreme_death == S::S_NULL
        && d.states.raise == S::S_NULL
        && d.spawn_health == 20
        && d.reaction_time == 8
        && d.pain_chance == 0
        && d.radius == 10 << 16
        && d.height == 42 << 16
        && d.mass == 100
        && d.flags == MF_SOLID | MF_SHOOTABLE | MF_NOBLOOD
        && chain.iter().all(|(s, tics, action, next)| {
            let x = s.descriptor();
            x.tics == *tics && x.action == *action && x.next == *next
        })
}
/// `ty` is `MT_BARREL` and its compile-time definition is [`source_definition`].
pub(crate) fn validate_definition(ty: ActorTypeId) -> Result<(), ActorError> {
    static MEMO: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if is_barrel_type(ty) && *MEMO.get_or_init(source_definition) {
        Ok(())
    } else {
        Err(ActorError::InvalidType)
    }
}

/// The motion access of a barrel's `P_XYMovement` under the barrels policy: the
/// shared non-player movement plus the barrel escape witness.
///
/// The M3h5 Invariant Ω proof (M3h5 note §4.3) covers an accepted move from `c` to
/// `c′` by the two boxes at `c` and `c′`, which needs the per-axis step below the
/// box width `2r`. `P_XYMovement` clamps momentum to ±30 per axis and halves only
/// a positive step above 15 (p_mobj.c:150–164), and `P_TryMove` tests only the
/// destination box (p_map.c:451–517), so a negative step reaches 30 units. That is
/// below `2r` for the player (32) and every admitted monster (40) but not for a
/// barrel (radius 10, `2r` = 20): a fast barrel can clear a wall corner in one
/// accepted step and leave the room exactly as the source lets it. After every
/// accepted attempt the barrel's centre is tested against the one-sided boundary
/// (`Boundary::inside`, exact integer ray parity); even parity refuses the tic with
/// `MapScaleError::Escape` and the checked transaction keeps the state before it.
/// The test is exact, so it never refuses a move that stays in Ω. The player keeps
/// its own slide witness (`world_pickups::Contacts::after_slide`); monsters pay no
/// check.
pub(crate) struct BarrelMotion<'a, G> {
    pub get: G,
    pub boundary: &'a crate::player_map_scale::Boundary,
}
impl<G: Fn(ActorId) -> Option<Actor>> crate::movement::MotionAccess for BarrelMotion<'_, G> {
    type Error = SimError;
    fn get(&self, id: ActorId) -> Option<Actor> {
        (self.get)(id)
    }
    fn after_move(&mut self, actor: &Actor) -> Result<(), SimError> {
        if self.boundary.inside(actor.position()) {
            Ok(())
        } else {
            Err(crate::MapScaleError::Escape.into())
        }
    }
}

/// The live witness of a barrel at a tic boundary (M3h9b note §2.1–§2.6): its
/// definition; a role state (or `S_GIBS` after a crunch, `gibbed`) with the
/// state's sprite and frame and a countdown inside the state's tics; health at
/// most 20, positive exactly in `S_BAR1`/`S_BAR2`; the info flags plus at most
/// `MF_AMBUSH` while alive, `MF_SHOOTABLE` cleared and `MF_CORPSE|MF_DROPOFF` set
/// once dead (`P_KillMobj`, p_inter.c:675–680), `MF_SOLID` kept until a crunch
/// clears it (p_map.c:1274); solidity from `MF_SOLID`; radius 10 (0 gibbed);
/// height 42 alive, 10.5 dead (`height >>= 2`, 681), 0 gibbed; dropoff exactly
/// when dead; reaction time 8 or 0 (`P_DamageMobj` zeroes it for a survivor,
/// p_inter.c:902); threshold 0, or 100 with a target (the retarget block,
/// 904–915, never lowered for a barrel), and threshold 100 only with reaction
/// time 0 (902 zeroes it before that block runs; threshold 0 keeps either, as a
/// Rust-only `damage_actor` of amount 0 zeroes it without retargeting); no
/// `movedir`, `movecount` or death
/// request; `lastlook` from one `P_Random() % 4`; a 2035 spawnpoint; a position in
/// the admitted room. The target handle itself is not resolved here: a stale one
/// is the explosion's own witness (decision F4(iv)).
pub(crate) fn validate_barrel(
    room: Option<crate::player_map_scale::RoomBounds>,
    a: Actor,
    gibbed: bool,
) -> Result<(), SimError> {
    let l = a.life().ok_or(ActorError::InvalidType)?;
    validate_definition(l.actor_type)?;
    let d = OriginalActorType::MT_BARREL.descriptor();
    let state = l
        .state
        .and_then(|s| OriginalState::try_from(s.0).ok())
        .ok_or(PlayerBodyError::Invariant)?;
    let alive = live_state(state);
    let dead_flags = (d.flags & !MF_SHOOTABLE) | MF_CORPSE | MF_DROPOFF;
    let (flags, radius, height) = if gibbed {
        (dead_flags & !MF_SOLID, 0, 0)
    } else if alive {
        (d.flags, d.radius, d.height)
    } else {
        (dead_flags, d.radius, d.height >> 2)
    };
    let s = state.descriptor();
    let shape_ok = gibbed
        || (state_allowed(state)
            && l.sprite == s.sprite.ordinal()
            && l.frame == s.frame
            && (1..=s.tics).contains(&l.tics));
    if !shape_ok
        || l.health > d.spawn_health
        || (l.health > 0) != alive
        || l.flags & !MF_AMBUSH != flags
        || (a.solidity == Solidity::Solid) != (l.flags & MF_SOLID != 0)
        || a.radius() != Fixed::from_bits(radius)
        || a.height() != Fixed::from_bits(height)
        || a.allows_dropoff() == alive
        || !(l.reaction_time == d.reaction_time || l.reaction_time == 0)
        || !(l.threshold == 0 || l.threshold == BASETHRESHOLD)
        || l.target.is_some() != (l.threshold == BASETHRESHOLD)
        || (l.threshold == BASETHRESHOLD && l.reaction_time != 0)
        || l.movedir != 0
        || l.movecount != 0
        || l.death_requested
        || a.lastlook() > 3
        || l.spawnpoint
            .is_none_or(|p| i32::from(p.kind) != d.doomed_number)
        || !crate::player_map_scale::position_admitted(room, a.position(), 978)
    {
        return Err(PlayerBodyError::Invariant.into());
    }
    Ok(())
}

/// The per-explosion sight allowance (decision F6): at most
/// [`EXPLOSION_SIGHT_QUERIES`] (216) queries and that many times one query's work bound
/// `S` (`extent_sight_work + 5·lines`, the per-query charge of the shared per-tic
/// control). Transient, never canonical state.
struct ExplosionSight {
    queries: usize,
    work: usize,
}
impl ExplosionSight {
    fn charge(&mut self, world: &crate::StaticWorld) -> Result<(), SimError> {
        let one = world
            .extent_sight_work
            .and_then(|w| w.checked_add(world.lines.len().checked_mul(5)?))
            .ok_or(PerceptionError::WorkLimit)?;
        // The Episode 1 policy derives the allowance from the map's own `K` and
        // `B` (decision H4): `(1 + K + B)·(1 + K)`.
        let allowance = world
            .episode1
            .map_or(EXPLOSION_SIGHT_QUERIES, |b| b.explosion_sight_queries());
        let budget = one
            .checked_mul(allowance)
            .ok_or(PerceptionError::WorkLimit)?;
        let queries = self.queries + 1;
        let work = self
            .work
            .checked_add(one)
            .ok_or(PerceptionError::WorkLimit)?;
        if queries > allowance || work > budget {
            return Err(PerceptionError::WorkLimit.into());
        }
        self.queries = queries;
        self.work = work;
        Ok(())
    }
}

/// `A_Explode` (p_enemy.c:1598–1601), run by the state entry of `S_BEXP4` inside
/// the barrel's own thinker (p_mobj.c:441–449) on the staged tic. Any error is
/// latched and refuses the whole tic with the canonical state preserved.
pub(crate) fn explode<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) {
    if let Err(error) = radius_attack(h, id) {
        // Keep the first latched cause; the staged tic is discarded either way.
        if !h.sight_failed() {
            h.fail_sight(error);
        }
    }
}

/// `P_RadiusAttack (spot, spot->target, 128)` (p_map.c:1206–1233).
fn radius_attack<A: ActorAccess>(h: &mut LifeHost<'_, A>, spot: ActorId) -> Result<(), SimError> {
    let bomb = *h.access.get(spot).ok_or(CombatError::Reference)?;
    let life = bomb.life().ok_or(ActorError::InvalidType)?;
    // Admission names A_Explode for MT_BARREL's S_BEXP4 and, under the Episode 1
    // policy, MT_ROCKET's S_EXPLODE1 (M4a plan §E.4): `P_RadiusAttack (rocket,
    // rocket->target, 128)` with the shooter as the source (self damage included).
    let rocket = crate::original_registry::is_rocket(life.actor_type)
        && h.player.as_deref().is_some_and(|p| p.policy.has_episode1());
    if !is_barrel_type(life.actor_type) && !rocket {
        return Err(
            ActorError::UnsupportedAction(crate::lifecycle::source_action_id("A_Explode")).into(),
        );
    }
    // `bombsource = source` (1227) is the barrel's `target`: its first non-lethal
    // wounder with a non-NULL source (p_inter.c:904–915), or NULL. A handle that
    // no longer resolves would be a dangling pointer in C (undefined); the tic is
    // refused before any victim is touched (decision F4(iv)). Pending storage stays
    // readable, so a source removed earlier in this tic is still the source.
    let source = life.target;
    if source.is_some_and(|s| h.access.get(s).is_none()) {
        return Err(PlayerBodyError::Invariant.into());
    }
    let origin = h.world.map().blockmap().origin();
    let origin = [
        Fixed::from_map_units(origin.x).bits(),
        Fixed::from_map_units(origin.y).bits(),
    ];
    let [width, height] = h.world.map().blockmap().dimensions().map(i32::from);
    // p_map.c:1222–1225: arithmetic shifts of fixed values, from the BLOCKMAP
    // origin. Positions lie in the admitted room (below 2¹⁵ units), so no sum
    // overflows; the checked forms make that explicit.
    let block = |v: i32, o: i32, sign: i32| -> Result<i32, SimError> {
        v.checked_add(sign * BLAST_WINDOW)
            .and_then(|v| v.checked_sub(o))
            .map(|v| v >> MAPBLOCKSHIFT)
            .ok_or_else(|| CombatError::Coordinates.into())
    };
    let [x, y] = bomb.position().map(Fixed::bits);
    let (yh, yl) = (block(y, origin[1], 1)?, block(y, origin[1], -1)?);
    let (xh, xl) = (block(x, origin[0], 1)?, block(x, origin[0], -1)?);
    let mut sight = ExplosionSight {
        queries: 0,
        work: 0,
    };
    #[cfg(test)]
    walk_trace::push(walk_trace::Row::Attack {
        spot,
        source,
        window: [xl, xh, yl, yh],
        position: [x, y, bomb.z().bits()],
        dist: BLAST_WINDOW,
    });
    // A defensive visit bound: every linked thing at most once per cell, plus one
    // more pass per relinking victim (only a monster entering its seestate
    // `A_Chase` relinks, at most once per explosion since its threshold becomes
    // 100). Unreachable on an admitted map; exhaustion refuses the tic.
    //
    // Why the walk never comes near it (M4b step 8; this also shows the mutant
    // `9 + order·(R + 2)` refuses nothing). The walk is a run of segments, each
    // following `bnext` from a cell head without revisiting: one per base cell
    // walk, and one per relink, from the relinked monster's new successor
    // (p_maputl.c:436–442, 529–535); a wire-24 teleport inside that move
    // relinks the same monster again, and the walk follows only its final link
    // (p_map.c:115–175, 451–517; p_telept.c:48–131). Nothing unlinks, and the
    // only things linked during the walk are drops (p_inter.c:745–769; the
    // teleport fogs are NOBLOCKMAP, info.c:2122–2144): nothing else the
    // callback reaches moves or spawns a linked thing (p_map.c:1164–1197;
    // p_inter.c:668–769, 775–913; p_enemy.c:672–767).
    // A monster relinks at most once, at its first visit: one not hit then is
    // never hit (fixed position and sight, p_map.c:1179–1191); a hit with pain,
    // a melee/missile state or an unmoved chase leaves its spawn state for the
    // rest of the walk (p_inter.c:894–913); a chase at a dead source that falls
    // back to the spawn state repeats with the same outcome on every later hit,
    // since one call of `P_LookForPlayers` leaves `lastlook` at the one player's
    // index for good and the sound target does not change (p_enemy.c:499–553,
    // 604–630, 701–712). So with order = n, L relinks and D′ drops of monsters
    // that never relink, counting the segments that can reach each thing (its
    // cell's base walk, then the relinks into its cell while it is there): a
    // thing that never relinks is visited at most 1 + L times, the j-th
    // relinker at most 2 + L − j (its relinking visit, then its new cell), the
    // drop of the i-th relinker (spawned after relink i) at most 1 + L − i, any
    // other drop at most 1 + L. Summed, visits ≤
    // n(1 + L) + L + D′(1 + L). With K monsters (L + D′ ≤ K, K ≤ R by the
    // roster limit, `map_demon_limit`; n ≥ K + 2 for the player and the exploding
    // barrel or, under episode1, the exploding rocket, which `spatial.order` holds
    // although it is NOBLOCKMAP: `LifeHost::spawn` links every thing) that is at
    // most n(R + 2) − 2.
    let roster = h
        .world
        .episode1
        .map_or(MAX_PLAYER_MONSTERS, |b| b.visit_roster());
    let limit = 9 * h.spatial.order.len().max(1) * (roster + 2);
    let mut visits = 0usize;
    // `y` outer, `x` inner (1230–1231), unlike P_CheckPosition and P_ChangeSector.
    for by in yl..=yh {
        for bx in xl..=xh {
            // P_BlockThingsIterator skips cells off the grid (p_maputl.c:520–526).
            if bx < 0 || by < 0 || bx >= width || by >= height {
                continue;
            }
            let mut next = first_in_cell(h, [bx, by]);
            #[cfg(test)]
            let mut ordinal = 0;
            while let Some(thing) = next {
                visits += 1;
                if visits > limit {
                    return Err(CombatError::WorkLimit.into());
                }
                #[cfg(test)]
                {
                    walk_trace::visit(by * width + bx, ordinal);
                    ordinal += 1;
                }
                pit_radius_attack(h, thing, spot, source, &mut sight)?;
                if h.sight_failed() {
                    return Ok(());
                }
                // `for (mobj = blocklinks[..]; mobj; mobj = mobj->bnext)`
                // (p_maputl.c:529–535) reads `bnext` after the callback returns.
                // `P_DamageMobj` may have run a monster's seestate `A_Chase`
                // (p_inter.c:912–914), whose `P_TryMove` unlinked and relinked it at
                // the head of its (possibly new) cell (p_maputl.c:436–442): the walk
                // then continues from its new successor in its new cell, revisiting
                // or skipping things exactly as the source does (M3h9b note §5.2).
                next = next_in_cell(h, thing)?;
            }
        }
    }
    Ok(())
}

/// Whether `id` is BLOCKMAP-linked in cell `cell`: present in the link order,
/// not `MF_NOBLOCKMAP`, and its position in that cell.
fn linked_in<A: ActorAccess>(h: &LifeHost<'_, A>, id: ActorId, cell: [i32; 2]) -> bool {
    h.access.get(id).is_some_and(|a| {
        crate::lifecycle::blocks_cell_index(a)
            && crate::movement::cell(h.world, a.position()) == cell
    })
}
/// `blocklinks[cell]`: the newest-linked thing of the cell. The spatial index keeps
/// one global link order, newest first (`SpatialIndex::link` prepends, as
/// `P_SetThingPosition` prepends to its cell, p_maputl.c:436–442), so a cell's list
/// is that order filtered by cell, never re-sorted.
fn first_in_cell<A: ActorAccess>(h: &LifeHost<'_, A>, cell: [i32; 2]) -> Option<ActorId> {
    h.spatial
        .order
        .iter()
        .copied()
        .find(|id| linked_in(h, *id, cell))
}
/// `thing->bnext`, read now: the next thing after `thing` in the link order that is
/// linked in `thing`'s current cell. A thing no longer linked would leave a stale
/// `bnext` in C; no admitted callback unlinks its victim (a death state never
/// reaches `S_NULL` at once), so that case refuses the tic.
fn next_in_cell<A: ActorAccess>(
    h: &LifeHost<'_, A>,
    thing: ActorId,
) -> Result<Option<ActorId>, SimError> {
    let at = h
        .spatial
        .order
        .iter()
        .position(|id| *id == thing)
        .ok_or(PlayerBodyError::Invariant)?;
    let a = h.access.get(thing).ok_or(PlayerBodyError::Invariant)?;
    if !crate::lifecycle::blocks_cell_index(a) {
        return Err(PlayerBodyError::Invariant.into());
    }
    let cell = crate::movement::cell(h.world, a.position());
    Ok(h.spatial.order[at + 1..]
        .iter()
        .copied()
        .find(|id| linked_in(h, *id, cell)))
}

/// `PIT_RadiusAttack` (p_map.c:1164–1198) for one candidate.
fn pit_radius_attack<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    spot: ActorId,
    source: Option<ActorId>,
    sight: &mut ExplosionSight,
) -> Result<(), SimError> {
    let thing = *h.access.get(id).ok_or(CombatError::Reference)?;
    let Some(life) = thing.life() else {
        #[cfg(test)]
        walk_trace::outcome(id, 0, -1);
        return Ok(());
    };
    // 1170: the exploding barrel itself is skipped here (P_KillMobj cleared it).
    if life.flags & MF_SHOOTABLE == 0 {
        #[cfg(test)]
        walk_trace::outcome(id, 0, -1);
        return Ok(());
    }
    // 1175: never present under this policy; kept for fidelity.
    if life.actor_type.0 == OriginalActorType::MT_CYBORG.ordinal()
        || life.actor_type.0 == OriginalActorType::MT_SPIDER.ordinal()
    {
        #[cfg(test)]
        walk_trace::outcome(id, 1, -1);
        return Ok(());
    }
    let bomb = *h.access.get(spot).ok_or(CombatError::Reference)?;
    // 1179–1185: 2-D Chebyshev distance minus the radius, floored by the
    // arithmetic shift, clamped at 0.
    let delta = |axis: usize| {
        thing.position()[axis]
            .bits()
            .checked_sub(bomb.position()[axis].bits())
            .and_then(i32::checked_abs)
            .ok_or(CombatError::Coordinates)
    };
    let dist = delta(0)?.max(delta(1)?);
    let dist = (dist
        .checked_sub(thing.radius().bits())
        .ok_or(CombatError::Coordinates)?
        >> 16)
        .max(0);
    // 1188.
    if dist >= BOMB_DAMAGE {
        #[cfg(test)]
        walk_trace::outcome(id, 2, dist);
        return Ok(());
    }
    // 1191: P_CheckSight (thing, bombspot), the candidate as looker; the dead
    // barrel's window is its bottom 10.5 units (height >>= 2, p_inter.c:681).
    // Every query is recorded with the existing sight record (observer the
    // candidate, target the barrel), blocked or not, before the victim's damage:
    // the C projection pins one per candidate reaching P_CheckSight (decision F9).
    sight.charge(h.world)?;
    let result = crate::sight::checked_query(h.world, h.sectors, thing, bomb)?;
    let visible = result.visible;
    h.events
        .push(crate::Event::Perception(crate::PerceptionEvent::Sight {
            observer: id,
            target: spot,
            result,
        }));
    if !visible {
        #[cfg(test)]
        walk_trace::outcome(id, 3, dist);
        return Ok(());
    }
    #[cfg(test)]
    walk_trace::outcome(id, 4, dist);
    // 1194: P_DamageMobj (thing, bombspot, bombsource, bombdamage - dist).
    let plan = DamagePlan::current_original(
        h,
        DamageRequest {
            target: id,
            inflictor: Some(spot),
            source,
            amount: BOMB_DAMAGE - dist,
        },
    )?;
    h.access.reserve(plan.spawn_count())?;
    h.apply_damage(plan);
    Ok(())
}

/// A test-only observer of the radius attack (the M3h9b replay's `RA` and `RC`
/// comparison rows, `barrels_replay`): when enabled on the current thread it
/// records each `P_RadiusAttack` call and every `PIT_RadiusAttack` candidate in
/// walk order with its BLOCKMAP list, its ordinal in that list's walk and its
/// outcome. It reads values the walk already computed and never changes state;
/// disabled (the default) it records nothing.
#[cfg(test)]
pub(crate) mod walk_trace {
    use crate::ActorId;
    use std::cell::RefCell;

    /// One observed row.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) enum Row {
        /// `P_RadiusAttack`: spot, source, block window `[xl, xh, yl, yh]`, the
        /// spot's raw position and the wrapped `dist`.
        Attack {
            spot: ActorId,
            source: Option<ActorId>,
            window: [i32; 4],
            position: [i32; 3],
            dist: i32,
        },
        /// `PIT_RadiusAttack`: candidate, block index `y·width + x`, ordinal in the
        /// block's walk, outcome code (0 not shootable, 1 boss, 2 out of range,
        /// 3 sight blocked, 4 damaged) and `dist` (−1 for codes 0 and 1).
        Candidate {
            thing: ActorId,
            block: i32,
            position: usize,
            code: u8,
            dist: i32,
        },
    }
    thread_local! {
        static ROWS: RefCell<Option<Vec<Row>>> = const { RefCell::new(None) };
        static AT: RefCell<(i32, usize)> = const { RefCell::new((0, 0)) };
    }
    /// Starts recording on this thread (discarding anything recorded).
    pub(crate) fn enable() {
        ROWS.with(|r| *r.borrow_mut() = Some(Vec::new()));
    }
    /// Stops recording on this thread.
    pub(crate) fn disable() {
        ROWS.with(|r| *r.borrow_mut() = None);
    }
    /// The rows recorded since the last call (empty when disabled).
    pub(crate) fn take() -> Vec<Row> {
        ROWS.with(|r| {
            r.borrow_mut()
                .as_mut()
                .map(std::mem::take)
                .unwrap_or_default()
        })
    }
    pub(super) fn push(row: Row) {
        ROWS.with(|r| {
            if let Some(rows) = r.borrow_mut().as_mut() {
                rows.push(row);
            }
        });
    }
    pub(super) fn visit(block: i32, position: usize) {
        AT.with(|a| *a.borrow_mut() = (block, position));
    }
    pub(super) fn outcome(thing: ActorId, code: u8, dist: i32) {
        let (block, position) = AT.with(|a| *a.borrow());
        push(Row::Candidate {
            thing,
            block,
            position,
            code,
            dist,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decision F6: one explosion's allowance is exactly `24 × (1 + K)` = 216
    /// queries of one query's work each; the 217th is `PerceptionError::WorkLimit`,
    /// without touching the counters.
    #[test]
    fn the_explosion_sight_allowance_is_216_queries() {
        let map = crate::player_e1m1_tests::room(12).map();
        let sim = crate::Simulation::original_player_barrels_map(
            map,
            crate::player_movers_tests::options(),
        )
        .unwrap();
        let mut sight = ExplosionSight {
            queries: 0,
            work: 0,
        };
        for _ in 0..EXPLOSION_SIGHT_QUERIES {
            sight.charge(&sim.world).unwrap();
        }
        let (queries, work) = (sight.queries, sight.work);
        assert_eq!(queries, 216);
        assert_eq!(
            sight.charge(&sim.world),
            Err(SimError::Perception(PerceptionError::WorkLimit))
        );
        assert_eq!((sight.queries, sight.work), (queries, work));
        let one = sim.world.extent_sight_work.unwrap() + 5 * sim.world.lines.len();
        assert_eq!(work, 216 * one);
    }
}
