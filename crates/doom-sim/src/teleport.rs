//! Teleportation under wire 24 (`PlayerMonstersEpisode1V2`, M4b step 4; decisions
//! I3, I7 and I8): line special 97 (`EV_Teleport`, p_telept.c:47–131), the landing
//! `MT_TELEPORTMAN` (doomednum 14), `P_TeleportMove` and `PIT_StompThing`
//! (p_map.c:81–177) and the two `MT_TFOG` spawns.
//!
//! The teleport runs inside the move, as `P_TryMove`'s crossing loop runs it
//! (p_map.c:500–514): [`cross`] is called for each crossing in the loop's order, and
//! its result tells the caller whether `P_TeleportMove` was reached, which sets
//! `numspechit = 0` (p_map.c:152) and so ends the loop, success or failure. The
//! caller then continues the motion from wherever the thing now stands: the second
//! half of a split move (p_mobj.c:147–191) and the rest of a slide (p_map.c:
//! 762–787) carry on from the landing.
//!
//! Every mutation here happens inside the checked transaction of the tic: a refusal
//! (the store-full witness, a damage plan, a refused fog spawn) latches its error
//! and the whole staged tic is discarded, as for every other mid-tic refusal.
use crate::lifecycle::{ActorAccess, LifeHost};
use crate::{
    ActorId, ActorTypeId, DamageRequest, Event, MoverEvent, PlayerBodyError, SimError, SpawnHeight,
    StoreError,
    identity::OriginalActorType,
    line_triggers::{Crosser, Crossing},
    original_player::{MF_MISSILE, MF_SHOOTABLE},
};
use doom_types::Fixed;

/// Line special 97, WR teleport (p_spec.c:904–907): `EV_Teleport`, the special
/// kept.
pub(crate) const LINE_TELEPORT: i16 = 97;
/// `PIT_StompThing`'s damage (p_map.c:105).
pub(crate) const TELEFRAG_DAMAGE: i32 = 10_000;
/// `thing->reactiontime = 18` for a player (p_telept.c:121–122).
pub(crate) const PLAYER_TELEPORT_FREEZE: i32 = 18;
/// The fog-2 offset from the landing, in map units (p_telept.c:114).
const FOG_OFFSET: i32 = 20;

/// `MT_TELEPORTMAN` under the teleport policy.
pub(crate) fn is_landing(policy: crate::OriginalExecutionProfile, ty: ActorTypeId) -> bool {
    policy.has_teleport() && ty.0 == OriginalActorType::MT_TELEPORTMAN.ordinal()
}
/// `MT_TFOG` under the teleport policy.
pub(crate) fn is_fog(policy: crate::OriginalExecutionProfile, ty: ActorTypeId) -> bool {
    policy.has_teleport() && ty.0 == OriginalActorType::MT_TFOG.ordinal()
}

/// The tic-boundary witness of a landing: its `MT_TELEPORTMAN` record as
/// `P_SpawnMapThing` left it (info.c:2174–2198; p_mobj.c:479–534, 708–797):
/// state `S_NULL` with `tics` −1, health 1,000, the info flags plus at most
/// `MF_AMBUSH`, no momentum, no target, inside the admitted room.
pub(crate) fn validate_landing(
    room: Option<crate::player_map_scale::RoomBounds>,
    a: crate::Actor,
) -> Result<(), SimError> {
    let l = a.life.ok_or(PlayerBodyError::Invariant)?;
    let d = OriginalActorType::MT_TELEPORTMAN.descriptor();
    if l.state != Some(crate::StateId(0))
        || l.tics != -1
        || l.health != d.spawn_health
        || l.flags & !crate::world_pickups::MF_AMBUSH != d.flags
        || l.target.is_some()
        || a.spec.momentum != [Fixed::ZERO; 2]
        || a.momz != Fixed::ZERO
        || !crate::player_map_scale::position_admitted(room, a.spec.position, 994)
    {
        return Err(PlayerBodyError::Invariant.into());
    }
    Ok(())
}

/// What one `EV_Teleport` call did (DSIM40 sub-tags 15 and 16).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Outcome {
    /// `side == 1` (p_telept.c:70–71): returned before the search.
    BackSide,
    /// No sector with the line's tag holds a landing (p_telept.c:130).
    NoLanding,
    /// `P_TeleportMove` refused (a non-player met a shootable thing, p_map.c:102–103):
    /// nothing moved, no fog, but the loop ends (`numspechit = 0` ran first).
    Blocked,
    /// The thing stands on the landing.
    Teleported,
}
impl Outcome {
    /// Whether `P_TeleportMove` was called, which ends `P_TryMove`'s crossing loop
    /// (p_map.c:152 sets `numspechit = 0` before the stomp loop).
    pub(crate) const fn ends_crossings(self) -> bool {
        matches!(self, Self::Blocked | Self::Teleported)
    }
}

/// `P_CrossSpecialLine` for special 97 (p_spec.c:904–907) and its crossing record
/// (DSIM35 sub-tag 3, after the teleport's records). The crosser is a player or a
/// non-missile thing: `P_CrossSpecialLine` returns for every Episode 1 missile type
/// before its switch (p_spec.c:503–518) and admits 97 for any other non-player
/// (520–534). Returns whether the caller's crossing loop ends.
pub(crate) fn cross<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    actor: ActorId,
    crosser: Crosser,
    crossing: Crossing,
) -> Result<bool, SimError> {
    if !h.world.teleport || crossing.special != LINE_TELEPORT {
        // Defence in depth: rule 1 admits 97 under wire 24 only.
        return Err(crate::PursuitError::Specials.into());
    }
    let player = h.player.as_ref().is_some_and(|p| p.mo == Some(actor));
    if player != (crosser == Crosser::Player) {
        return Err(PlayerBodyError::Binding.into());
    }
    let outcome = ev_teleport(h, actor, crossing.line, crossing.side)?;
    h.events.push(Event::Mover(MoverEvent::Crossed {
        actor,
        line: crossing.line,
        side: crossing.side,
        before: crossing.special,
        after: crossing.special,
    }));
    Ok(outcome.ends_crossings())
}

/// `EV_Teleport (line, side, thing)` (p_telept.c:47–131) with the thing's current
/// record in the host's store.
pub(crate) fn ev_teleport<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    line: u16,
    side: u8,
) -> Result<Outcome, SimError> {
    let thing = *h.access.get(id).ok_or(StoreError::InvalidHandle)?;
    let life = thing.life.ok_or(PlayerBodyError::Invariant)?;
    // p_telept.c:65–66. Unreachable: every Episode 1 missile type returns earlier in
    // `P_CrossSpecialLine` and no crossing of a missile reaches this service.
    if life.flags & MF_MISSILE != 0 {
        return Err(PlayerBodyError::Invariant.into());
    }
    if side == 1 {
        not_taken(h, id, line, 0);
        return Ok(Outcome::BackSide);
    }
    let Some(landing) = find_landing(h, line)? else {
        not_taken(h, id, line, 1);
        return Ok(Outcome::NoLanding);
    };
    let m = *h.access.get(landing).ok_or(StoreError::InvalidHandle)?;
    let old = [thing.spec.position[0], thing.spec.position[1], thing.z];
    let dest = m.spec.position;
    let player = h.player.as_ref().is_some_and(|p| p.mo == Some(id));
    // P_TeleportMove (p_map.c:115–177). Its globals (`tmfloorz` and the rest, and
    // `numspechit = 0`) are set before the stomp loop and are read by nothing after
    // a teleport (M4b note §2 T3.7), so only the thing's own record changes.
    // `tmfloorz = tmdropoffz = floorheight`, `tmceilingz = ceilingheight` of the
    // centre subsector's sector only, read live (p_map.c:144–149): no line pass.
    let location = h.world.locate_validated(dest);
    let sector = h.sectors[usize::from(location.sector)];
    let victims = stomped(h, id, &thing, dest);
    // PIT_StompThing (p_map.c:102–103): on maps 1–9 (`gamemap != 30`) a
    // non-player's move fails at the first shootable thing it overlaps.
    if !victims.is_empty() && !player {
        not_taken(h, id, line, 2);
        return Ok(Outcome::Blocked);
    }
    // Every telefrag is planned before the first mutation, and the store-full
    // witness proves the slots of the drops and the two fogs (decision I7).
    let mut plans = Vec::with_capacity(victims.len());
    let mut spawns = 2_usize;
    for &victim in &victims {
        let plan = crate::combat::DamagePlan::telefrag(
            h,
            DamageRequest {
                target: victim,
                inflictor: Some(id),
                source: Some(id),
                amount: TELEFRAG_DAMAGE,
            },
        )?;
        spawns = spawns
            .checked_add(plan.spawn_count())
            .ok_or(StoreError::Capacity)?;
        plans.push(plan);
    }
    h.access.reserve(spawns)?;
    h.spatial.reserve_spawn()?;
    // `P_DamageMobj (thing, tmthing, tmthing, 10000)` per victim, in the stomp
    // loop's order (p_map.c:105). The inflictor still stands at its pre-teleport
    // position: `x`/`y` are assigned only after the loop (p_map.c:171–172).
    for plan in plans {
        h.apply_damage(plan);
        if let Some(error) = latched(h) {
            return Err(error);
        }
    }
    {
        let a = h.access.get_mut(id).ok_or(StoreError::InvalidHandle)?;
        a.spec.position = dest;
        a.floor = sector.floor;
        a.ceiling = sector.ceiling;
        a.location = Some(location);
        // p_telept.c:106: `thing->z = thing->floorz`.
        a.z = a.floor;
    }
    // P_SetThingPosition (p_map.c:174): the head of the cell and sector lists.
    h.spatial.link(id);
    let telept = crate::lifecycle::source_sound_id("sfx_telept");
    // p_telept.c:111–112: fog 1 at the old position and z.
    let fog = h.spawn(
        ActorTypeId(OriginalActorType::MT_TFOG.ordinal()),
        [old[0], old[1]],
        SpawnHeight::Absolute(old[2]),
    );
    h.sound(Some(fog), telept);
    // p_telept.c:113–118: fog 2 `20*finecosine[an]`, `20*finesine[an]` from the
    // landing (`finecosine = &finesine[FINEANGLES/4]`, r_main.c:113), at the new z.
    // The table's half-step offset makes it not exactly 20 units (M4b note §2 T2.9).
    let offset = fog_offset(m.spec.angle);
    let z = h.access.get(id).ok_or(StoreError::InvalidHandle)?.z;
    let fog = h.spawn(
        ActorTypeId(OriginalActorType::MT_TFOG.ordinal()),
        [dest[0] + offset[0], dest[1] + offset[1]],
        SpawnHeight::Absolute(z),
    );
    h.sound(Some(fog), telept);
    let a = h.access.get_mut(id).ok_or(StoreError::InvalidHandle)?;
    let l = a.life.as_mut().ok_or(PlayerBodyError::Invariant)?;
    // p_telept.c:121–125.
    if player {
        l.reaction_time = PLAYER_TELEPORT_FREEZE;
    }
    a.spec.angle = m.spec.angle;
    a.spec.momentum = [Fixed::ZERO; 2];
    a.momz = Fixed::ZERO;
    h.events.push(Event::Mover(MoverEvent::Teleported {
        actor: id,
        line,
        landing,
        old,
    }));
    Ok(Outcome::Teleported)
}

/// Fog 2's offset from a landing of angle `angle` (p_telept.c:113–115):
/// `an = angle >> ANGLETOFINESHIFT`, then `20*finecosine[an]` and `20*finesine[an]`
/// as `int` products (`finecosine = &finesine[FINEANGLES/4]`, r_main.c:113).
pub(crate) fn fog_offset(angle: u32) -> [Fixed; 2] {
    let an = (angle >> 19) as usize;
    [
        Fixed::from_bits(FOG_OFFSET * crate::FINE_SINE[an + 2048]),
        Fixed::from_bits(FOG_OFFSET * crate::FINE_SINE[an]),
    ]
}

fn not_taken<A: ActorAccess>(h: &mut LifeHost<'_, A>, actor: ActorId, line: u16, reason: u8) {
    h.events.push(Event::Mover(MoverEvent::TeleportNotTaken {
        actor,
        line,
        reason,
    }));
}

fn latched<A: ActorAccess>(h: &LifeHost<'_, A>) -> Option<SimError> {
    h.perception
        .and_then(|p| p.checked)
        .and_then(|c| c.failure.get())
}

/// The landing `EV_Teleport` finds (p_telept.c:74–97): the outer loop runs over the
/// sectors in index order and keeps those whose tag equals the line's (no tag-0
/// guard); for each it walks the whole thinker list for a live `MT_TELEPORTMAN`
/// whose subsector's sector is that sector. The first found wins, whether its move
/// then succeeds or not (103–104 return either way).
///
/// The search is read-only and bounded by construction: one pass over the actor
/// order (at most the store's slots) collects the landings in thinker order, then
/// each tagged sector is matched against them (at most sectors × landings). It runs
/// before any mutation of the teleport. A pending (removed) thinker is skipped, as
/// the source skips a thinker whose function is −1 (p_tick.c:80–84); a landing is
/// never removed.
fn find_landing<A: ActorAccess>(
    h: &LifeHost<'_, A>,
    line: u16,
) -> Result<Option<ActorId>, SimError> {
    let map = h.world.map();
    let tag = map
        .linedefs()
        .get(usize::from(line))
        .ok_or(crate::GeometryError::InvalidReference)?
        .tag;
    let landings: Vec<(ActorId, u16)> = h
        .access
        .live_order()
        .ok_or(StoreError::UnsupportedThinker)?
        .into_iter()
        .filter_map(|id| {
            let a = h.access.get(id)?;
            let l = a.life?;
            (l.actor_type.0 == OriginalActorType::MT_TELEPORTMAN.ordinal())
                .then(|| a.location.map(|loc| (id, loc.sector)))
                .flatten()
        })
        .collect();
    if landings.is_empty() {
        return Ok(None);
    }
    for i in crate::mover_rules::tagged(map, tag) {
        if let Some(&(id, _)) = landings.iter().find(|(_, s)| usize::from(*s) == i) {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// The shootable things `PIT_StompThing` reaches for `thing` moving to `dest`, in
/// the iteration order of `P_TeleportMove` (p_map.c:155–163: cells `bx` outer, `by`
/// inner over `tmbbox ± MAXRADIUS`, each cell by `bnext`), with its tests in order
/// (p_map.c:85–99): `MF_SHOOTABLE`, both axis distances below the summed radii (no
/// z test), not the thing itself. Killing one victim changes no later victim's
/// tests (its first death state has no action, M4b note §2 T3.8, and its drop is
/// linked behind the iterator), so collecting first equals the source's loop.
fn stomped<A: ActorAccess>(
    h: &LifeHost<'_, A>,
    id: ActorId,
    thing: &crate::Actor,
    dest: [Fixed; 2],
) -> Vec<ActorId> {
    let mut victims = Vec::new();
    crate::movement::for_things(
        h.world,
        id,
        thing,
        dest,
        h.spatial,
        &|i| h.access.get(i).copied(),
        |candidate, other| {
            let shootable = other.life.is_some_and(|l| l.flags & MF_SHOOTABLE != 0);
            let reach = i64::from(other.spec.radius.bits()) + i64::from(thing.spec.radius.bits());
            if shootable
                && (0..2).all(|axis| {
                    (i64::from(other.spec.position[axis].bits()) - i64::from(dest[axis].bits()))
                        .abs()
                        < reach
                })
                && candidate != id
            {
                victims.push(candidate);
            }
            true
        },
    );
    victims
}

/// Which escape witness the mover of a [`Motion`] carries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mover {
    /// The player body: pickups, the slide escape witness and the slide intercept
    /// witness of `world_pickups::Contacts`.
    Player,
    /// A barrel on a map with a one-sided boundary: the escape witness of
    /// `barrels::BarrelMotion`.
    Barrel,
    /// Any other non-missile thing with momentum (a monster, a corpse, a barrel on a
    /// map without boundary): no witness, as the plain closure access.
    Other,
}

/// The motion access of wire 24 (`P_XYMovement` of the player and of every sliding
/// non-missile thing): what the older accesses do for the same mover (delegated to
/// `world_pickups::Contacts` or `barrels::BarrelMotion`, built per call), plus the
/// crossings run inside the move, teleports included ([`cross`]; decision I7). The
/// fields are the thinker pass's own staged state, borrowed for one motion.
pub(crate) struct Motion<'a, 'b, 'c> {
    pub mover: Mover,
    pub access: &'b mut crate::ThinkerContext<'c, crate::Actor>,
    pub player: Option<&'b mut crate::original_player::PlayerRuntime>,
    pub statistics: Option<&'b mut crate::LevelStatistics>,
    pub perception: Option<crate::perception::PerceptionInputs<'a>>,
    pub registry: &'a crate::ActorRegistry,
    pub world: &'a crate::StaticWorld,
    pub sectors: crate::staging::Sectors<'a>,
    pub options: crate::ActorOptions,
    pub rng: &'b mut doom_types::DoomRng,
    pub bindings: &'b mut [Option<ActorId>; 4],
    pub specials: Option<&'b mut [crate::sector_thinkers::SectorSpecial]>,
    pub boundary: Option<&'a crate::player_map_scale::Boundary>,
}
impl<'b, 'c> Motion<'_, 'b, 'c> {
    #[allow(clippy::type_complexity)]
    fn contacts(
        &mut self,
    ) -> Result<
        crate::world_pickups::Contacts<'_, &'b mut crate::ThinkerContext<'c, crate::Actor>>,
        SimError,
    > {
        Ok(crate::world_pickups::Contacts {
            access: &mut self.access,
            player: self.player.as_deref_mut().ok_or(PlayerBodyError::Profile)?,
            sectors: self.sectors,
            boundary: self.boundary,
        })
    }
}
impl crate::movement::MotionAccess for Motion<'_, '_, '_> {
    type Error = SimError;
    fn get(&self, id: ActorId) -> Option<crate::Actor> {
        self.access.get(id).copied()
    }
    fn contacts(
        &mut self,
        id: ActorId,
        actor: &mut crate::Actor,
        attempt: &crate::ActorMoveAttempt,
        spatial: &mut crate::movement::SpatialIndex,
        events: &mut Vec<crate::movement::MotionEvent>,
    ) -> Result<(), SimError> {
        match self.mover {
            Mover::Player => self
                .contacts()?
                .contacts(id, actor, attempt, spatial, events),
            Mover::Barrel | Mover::Other => Ok(()),
        }
    }
    fn after_slide(&mut self, actor: &crate::Actor) -> Result<(), SimError> {
        match self.mover {
            Mover::Player => self.contacts()?.after_slide(actor),
            Mover::Barrel | Mover::Other => Ok(()),
        }
    }
    fn after_move(&mut self, actor: &crate::Actor) -> Result<(), SimError> {
        match (self.mover, self.boundary) {
            (Mover::Barrel, Some(boundary)) => crate::barrels::BarrelMotion {
                get: |_| None,
                boundary,
            }
            .after_move(actor),
            _ => Ok(()),
        }
    }
    fn intercept_limit(&self) -> Option<usize> {
        match self.mover {
            Mover::Player => self
                .player
                .as_ref()
                .filter(|p| p.policy.has_real_map())
                .map(|_| crate::player_map_scale::MAX_INTERCEPTS),
            Mover::Barrel | Mover::Other => None,
        }
    }
    fn intercept_overflow(&self) -> SimError {
        crate::CombatError::InterceptLimit.into()
    }
    fn intercept_arithmetic(&self) -> SimError {
        crate::MapScaleError::Arithmetic.into()
    }
    fn crosses_in_move(&self) -> bool {
        true
    }
    fn cross(
        &mut self,
        id: ActorId,
        actor: &mut crate::Actor,
        crosser: Crosser,
        crossing: Crossing,
        sectors: crate::staging::Sectors<'_>,
        spatial: &mut crate::movement::SpatialIndex,
        events: &mut Vec<crate::movement::MotionEvent>,
    ) -> Result<bool, SimError> {
        use crate::movement::MotionEvent;
        let mut out = Vec::new();
        let result = if crossing.special == LINE_TELEPORT {
            // The store holds the mover's record as of the tic's start; the
            // teleport reads and writes it there (the stomp's inflictor position
            // included), so it is synchronised around the call.
            *self.access.get_mut(id).ok_or(StoreError::InvalidHandle)? = *actor;
            let mut host = LifeHost {
                statistics: self.statistics.as_deref_mut(),
                player: self.player.as_deref_mut(),
                perception: self.perception,
                access: &mut *self.access,
                registry: self.registry,
                world: self.world,
                sectors,
                options: self.options,
                rng: &mut *self.rng,
                spatial,
                bindings: &mut *self.bindings,
                events: &mut out,
                buttons: None,
                specials: self.specials.as_deref(),
            };
            let result = cross(&mut host, id, crosser, crossing);
            *actor = *self.access.get(id).ok_or(StoreError::InvalidHandle)?;
            result.map(|ends| (ends, crossing.special))
        } else {
            crate::line_triggers::cross(
                &mut crate::movers::Starter {
                    world: self.world,
                    sectors,
                    store: &mut *self.access,
                    events: &mut out,
                    specials: self.specials.as_deref_mut(),
                    player: None,
                },
                id,
                crosser,
                crossing,
            )
            .map(|after| (false, after))
        };
        events.extend(out.into_iter().map(MotionEvent::Outer));
        let (ends, after) = result?;
        if after != crossing.special {
            events.push(MotionEvent::Stage {
                line: crossing.line,
                after,
            });
        }
        Ok(ends)
    }
}

#[cfg(test)]
#[path = "teleport_tests.rs"]
mod tests;
