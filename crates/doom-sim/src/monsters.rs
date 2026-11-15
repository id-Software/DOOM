//! Bounded source attack selection; one stationary thinker-head native owner.
use crate::{
    lifecycle::{ActorAccess, LifeHost},
    *,
};
use std::{error::Error, fmt};
const OWNERS: [u16; 6] = [3, 5, 18, 19, 21, 136];
const OWNER_FLAGS: u32 = 2 | 32 | 64 | 128 | 1024 | 2048 | 2097152 | 33554432;
const TARGET_FLAGS: u32 = 2 | 4 | 32 | 64 | 1024 | 2048 | 262144 | 524288 | 1048576 | 33554432;
/// Bounded monster-action rejection, before authoritative action effects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MonsterError {
    /// Requires the explicit bounded attack profile.
    Profile,
    /// Unsupported metadata, state, callback or actor role.
    Definition,
    /// The single unshootable owner must be stationary and thinker head.
    Ownership,
    /// Missing, pending, retired, stale or removable target.
    Target,
    /// Arithmetic or movement proof is outside the existing bounds.
    Arithmetic,
}
impl fmt::Display for MonsterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "monster attacks: {self:?}")
    }
}
impl Error for MonsterError {}
/// Source range outcome, including exactly the information actually evaluated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangeDecision {
    /// Original boolean result.
    pub accepted: bool,
    /// 0=accepted,1=null,2=distance,3=sight,4=JUSTHIT,5=reaction,
    /// 6=VILE far,7=UNDEAD near,8=random rejection.
    pub reason: u8,
    /// Melee raw fixed distance or missile signed integer distance, if reached.
    pub distance: Option<i32>,
    /// Melee raw fixed threshold or missile final capped probability threshold.
    pub threshold: Option<i32>,
    /// Sight only when source short-circuit evaluation reached it.
    pub sight: Option<SightResult>,
}
/// Transient source observations; actor fields carry persistent attack state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MonsterEvent {
    /// Evaluated original range branch (sight is separately observed with tag16).
    Range {
        /// Observer.
        actor: ActorId,
        /// 0=melee,1=missile.
        kind: u8,
        /// Source result.
        decision: RangeDecision,
    },
    /// Ordered gameplay draw: 0/1=shadow,2/3=spread,4=damage,5=missile chance,6=refire.
    Random {
        /// Owner.
        actor: ActorId,
        /// Source draw category.
        kind: u8,
        /// Lookup result.
        value: u8,
    },
    /// A_FaceTarget completion, including the source null-target early return.
    Face {
        /// Owner.
        actor: ActorId,
        /// Retained target.
        target: Option<ActorId>,
        /// Before/after BAM angle.
        angles: [u32; 2],
        /// Before/after source flags.
        flags: [u32; 2],
    },
    /// Chosen state, before melee sound or immediate entry.
    Selection {
        /// Owner.
        actor: ActorId,
        /// 0=melee,1=missile.
        kind: u8,
        /// Admitted native handoff.
        state: StateId,
    },
    /// Missile JUSTATTACKED assignment, after immediate state action.
    JustAttacked {
        /// Owner.
        actor: ActorId,
        /// Resulting source flags.
        flags: u32,
    },
    /// Pure source aim completion, before spread draws (sound order is action-specific).
    Aim {
        /// Owner.
        actor: ActorId,
        /// Complete autoaim evidence.
        result: Box<AimResult>,
    },
}
#[derive(Clone, Debug)]
pub(crate) struct MonsterShotPlan {
    pub owner: ActorId,
    pub rng: DoomRng,
    pub aim: AimResult,
    pub attack: crate::combat::PreparedAttack,
}
fn observe<A: ActorAccess>(h: &mut LifeHost<'_, A>, e: MonsterEvent) {
    h.events.push(Event::Monster(e));
}
pub(crate) fn draw<A: ActorAccess>(h: &mut LifeHost<'_, A>, actor: ActorId, kind: u8) -> u8 {
    let value = h.rng.p_random();
    observe(h, MonsterEvent::Random { actor, kind, value });
    value
}
fn decision(accepted: bool, reason: u8) -> RangeDecision {
    RangeDecision {
        accepted,
        reason,
        distance: None,
        threshold: None,
        sight: None,
    }
}
fn approx(a: Actor, b: Actor) -> i32 {
    movement::approx_distance([
        a.position()[0] - b.position()[0],
        a.position()[1] - b.position()[1],
    ])
    .bits()
}

pub(crate) fn melee(
    world: &StaticWorld,
    sectors: crate::staging::Sectors<'_>,
    r: &ActorRegistry,
    a: Actor,
    target: Option<Actor>,
) -> RangeDecision {
    checked_melee(world, sectors, r, a, target, None)
        .expect("historical melee has an admitted infallible sight domain")
}
fn checked_melee(
    world: &StaticWorld,
    sectors: crate::staging::Sectors<'_>,
    r: &ActorRegistry,
    a: Actor,
    target: Option<Actor>,
    checked: Option<&crate::demon_expanded_map::SightControl>,
) -> Result<RangeDecision, SimError> {
    let Some(t) = target else {
        return Ok(decision(false, 1));
    };
    let distance = approx(a, t);
    let threshold = 44 * 65536
        + r.actor_type(t.life.unwrap().actor_type)
            .unwrap()
            .radius()
            .bits();
    let mut d = decision(false, 2);
    d.distance = Some(distance);
    d.threshold = Some(threshold);
    if distance >= threshold {
        return Ok(d);
    }
    let sight = crate::demon_expanded_map::action_sight(checked, world, sectors, a, t)?;
    d.accepted = sight.visible;
    d.reason = if d.accepted { 0 } else { 3 };
    d.sight = Some(sight);
    Ok(d)
}
fn range_observation<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    actor: ActorId,
    kind: u8,
    mut d: RangeDecision,
) -> bool {
    if let Some(result) = d.sight.take() {
        h.events.push(Event::Perception(PerceptionEvent::Sight {
            observer: actor,
            target: h.access.get(actor).unwrap().life.unwrap().target.unwrap(),
            result,
        }));
    }
    let accepted = d.accepted;
    observe(
        h,
        MonsterEvent::Range {
            actor,
            kind,
            decision: d,
        },
    );
    accepted
}
pub(crate) fn melee_host<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) -> bool {
    let a = *h.access.get(id).unwrap();
    let target = a.life.unwrap().target.map(|t| *h.access.get(t).unwrap());
    let d = match checked_melee(
        h.world,
        h.sectors,
        h.registry,
        a,
        target,
        h.perception.and_then(|p| p.checked),
    ) {
        Ok(d) => d,
        Err(error) => {
            h.fail_sight(error);
            return false;
        }
    };
    range_observation(h, id, 0, d)
}
fn missile<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) -> bool {
    let a = *h.access.get(id).unwrap();
    let life = a.life.unwrap();
    let t = *h.access.get(life.target.unwrap()).unwrap();
    // Checked worlds use the bounded fallible query; historical fixtures keep
    // their infallible query. Only PlayerMonstersV1 reaches this with control.
    let sight = match h.perception.and_then(|p| p.checked) {
        Some(control) => {
            match crate::demon_expanded_map::action_sight(Some(control), h.world, h.sectors, a, t) {
                Ok(sight) => sight,
                Err(error) => {
                    h.fail_sight(error);
                    return false;
                }
            }
        }
        None => crate::sight::query(h.world, h.sectors, a, t),
    };
    let visible = sight.visible;
    h.events.push(Event::Perception(PerceptionEvent::Sight {
        observer: id,
        target: life.target.unwrap(),
        result: sight,
    }));
    let d = if !visible {
        decision(false, 3)
    } else if life.flags & 64 != 0 {
        h.access.get_mut(id).unwrap().life.as_mut().unwrap().flags &= !64;
        decision(true, 4)
    } else if life.reaction_time != 0 {
        decision(false, 5)
    } else {
        let info = h.registry.actor_type(life.actor_type).unwrap();
        let mut distance = (approx(a, t)
            - 64 * 65536
            - if info.states()[3].0 == 0 {
                128 * 65536
            } else {
                0
            })
            >> 16;
        if life.actor_type.0 == 3 && distance > 896 {
            let mut d = decision(false, 6);
            d.distance = Some(distance);
            d
        } else if life.actor_type.0 == 5 && distance < 196 {
            let mut d = decision(false, 7);
            d.distance = Some(distance);
            d
        } else {
            if [5, 18, 19, 21].contains(&life.actor_type.0) {
                distance >>= 1;
            }
            distance = distance.min(if life.actor_type.0 == 21 { 160 } else { 200 });
            let accepted = i32::from(draw(h, id, 5)) >= distance;
            let mut d = decision(accepted, if accepted { 0 } else { 8 });
            d.distance = Some(distance);
            d.threshold = Some(distance);
            d
        }
    };
    range_observation(h, id, 1, d)
}
pub(crate) fn select<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    work: &mut usize,
) -> bool {
    let life = h.access.get(id).unwrap().life.unwrap();
    let info = h.registry.actor_type(life.actor_type).unwrap();
    let states = info.states();
    let sound = info.sounds()[1];
    if states[3].0 != 0 && melee_host(h, id) {
        if h.perception.is_some_and(|input| {
            matches!(
                input.players,
                crate::player_roster::PerceptionRoster::OriginalPlayer(_)
            ) && !input.players.allows_player_melee()
        }) {
            // The historical late melee plan assumes a synthetic unbound body.
            // Preserve the real source branch boundary; commit no partial chase.
            h.fail_sight(PlayerBodyError::Unsupported.into());
            return false;
        }
        observe(
            h,
            MonsterEvent::Selection {
                actor: id,
                kind: 0,
                state: states[3],
            },
        );
        if sound != 0 {
            h.sound(Some(id), sound);
        }
        h.enter_counted(id, states[3], work);
        return true;
    }
    if states[4].0 != 0
        && (h.options.skill == Skill::Nightmare || life.movecount == 0)
        && missile(h, id)
    {
        observe(
            h,
            MonsterEvent::Selection {
                actor: id,
                kind: 1,
                state: states[4],
            },
        );
        h.enter_counted(id, states[4], work);
        let l = h.access.get_mut(id).unwrap().life.as_mut().unwrap();
        l.flags |= 128;
        let flags = l.flags;
        observe(h, MonsterEvent::JustAttacked { actor: id, flags });
        return true;
    }
    false
}
pub(crate) fn face_angle(a: Actor, t: Actor, rng: &mut DoomRng) -> u32 {
    let angle = movement::point_angle([
        t.position()[0] - a.position()[0],
        t.position()[1] - a.position()[1],
    ]);
    if t.life.unwrap().flags & 262144 == 0 {
        angle
    } else {
        let first = i32::from(rng.p_random());
        let second = i32::from(rng.p_random());
        angle.wrapping_add(((first - second) * 2097152) as u32)
    }
}
pub(crate) fn face<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) {
    let a = *h.access.get(id).unwrap();
    let life = a.life.unwrap();
    if let Some(t) = life.target {
        let t = *h.access.get(t).unwrap();
        let mut angle = movement::point_angle([
            t.position()[0] - a.position()[0],
            t.position()[1] - a.position()[1],
        ]);
        h.access.get_mut(id).unwrap().life.as_mut().unwrap().flags &= !32;
        if t.life.unwrap().flags & 262144 != 0 {
            let first = i32::from(draw(h, id, 0));
            let second = i32::from(draw(h, id, 1));
            angle = angle.wrapping_add(((first - second) * 2097152) as u32);
        }
        h.access.get_mut(id).unwrap().spec.angle = angle;
    }
    let after = h.access.get(id).unwrap();
    observe(
        h,
        MonsterEvent::Face {
            actor: id,
            target: life.target,
            angles: [a.angle(), after.angle()],
            flags: [life.flags, after.life.unwrap().flags],
        },
    );
}
pub(crate) fn pos_attack<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) {
    single_attack(h, id, false);
}
pub(crate) fn single_attack<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    sound_before_face: bool,
) {
    if h.access.get(id).unwrap().life.unwrap().target.is_none() {
        return;
    }
    let plan = h
        .perception
        .unwrap()
        .shot
        .expect("shot prepared before authoritative mutation")
        .clone();
    assert_eq!(plan.owner, id);
    assert_eq!(plan.rng, *h.rng);
    if sound_before_face {
        h.sound(Some(id), lifecycle::source_sound("sfx_shotgn").unwrap());
    }
    face(h, id);
    observe(
        h,
        MonsterEvent::Aim {
            actor: id,
            result: Box::new(plan.aim.clone()),
        },
    );
    if !sound_before_face {
        h.sound(Some(id), lifecycle::source_sound("sfx_pistol").unwrap());
    }
    draw(h, id, 2);
    draw(h, id, 3);
    draw(h, id, 4);
    crate::combat::execute_attack(h, plan.attack);
}

impl Simulation {
    /// Profile10 using strictly labelled native fixtures and original impact data.
    pub fn with_monster_attack_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if limit > 64 {
            return Err(PerceptionError::WorkLimit.into());
        }
        if map.linedefs().iter().any(|l| l.special != 0)
            || map.sectors().iter().any(|s| s.special != 0)
        {
            return Err(PursuitError::Specials.into());
        }
        crate::sight::validate_map(&map)?;
        validate_registry(&registry)?;
        let mut sim = Self::with_combat_registry(map, limit, registry, options)?;
        sim.profile = Profile::MonsterAttacks;
        sim.sound_targets = vec![None; sim.sectors.len()];
        Ok(sim)
    }
    pub(crate) fn monster_is_owner_type(&self, ty: ActorTypeId) -> bool {
        if self.profile == Profile::OriginalDemon {
            ty == original_registry::owner_slot()
        } else {
            OWNERS.contains(&ty.0)
        }
    }
    pub(crate) fn monster_is_owner(&self, id: ActorId) -> bool {
        self.actor(id)
            .and_then(Actor::life)
            .is_some_and(|l| self.monster_is_owner_type(l.actor_type))
    }
    pub(crate) fn validate_monster_type(&self, ty: ActorTypeId) -> Result<(), SimError> {
        if self.profile.uses_demon_lifecycle() {
            return self.validate_demon_lifecycle_type(ty);
        }
        if self.profile == Profile::DemonMelee {
            return self.validate_demon_type(ty);
        }
        if self.profile == Profile::Chaingunner {
            return self.validate_chaingun_type(ty);
        }
        if self.profile == Profile::Shotgunner {
            return self.validate_shotgun_type(ty);
        }
        if !self.monster_is_owner_type(ty) && ![37, 38, 135].contains(&ty.0) {
            return Err(MonsterError::Definition.into());
        }
        let info = self
            .registry
            .as_deref()
            .unwrap()
            .actor_type(ty)
            .ok_or(MonsterError::Definition)?;
        if self.monster_is_owner_type(ty)
            && (info.name != format!("FIXTURE_ATTACKER_{}", ty.0)
                || info.states()[..2] != [StateId(1), StateId(2)])
            || ty.0 == 135 && info.name != "FIXTURE_DAMAGE_TARGET"
        {
            return Err(MonsterError::Definition.into());
        }
        self.registry
            .as_deref()
            .unwrap()
            .validate_type_monsters(ty, &self.sectors)?;
        Ok(())
    }
    pub(crate) fn monster_actor(&self, id: ActorId, margin: u32) -> Result<Actor, SimError> {
        if !self.profile.uses_monsters() {
            return Err(MonsterError::Profile.into());
        }
        let a = self.perception_actor_ref(id)?;
        let l = a.life.unwrap();
        self.validate_monster_type(l.actor_type)?;
        if MoveDirection::from_raw(l.movedir).is_none()
            || [l.movecount, l.reaction_time, l.threshold].contains(&i32::MIN)
            || l.reaction_time == i32::MIN + 1
        {
            return Err(MonsterError::Arithmetic.into());
        }
        if self.monster_is_owner_type(l.actor_type) {
            if l.flags
                & !(OWNER_FLAGS
                    | if self.profile.uses_map_statistics() {
                        4194304
                    } else {
                        0
                    }
                    | if self.profile.uses_demon_lifecycle() {
                        4 | 1048576
                    } else {
                        0
                    })
                != 0
                || !self.profile.uses_demon_lifecycle()
                    && (a.momentum() != [Fixed::ZERO; 2] || a.momz != Fixed::ZERO || a.z != a.floor)
                || !self.profile.uses_demon_map() && self.actors.order().next() != Some(id)
            {
                return Err(MonsterError::Ownership.into());
            }
            if !self.position_admitted(a.position(), margin) {
                return Err(MonsterError::Arithmetic.into());
            }
        } else if l.actor_type == self.demon_body_type() && l.flags & !TARGET_FLAGS != 0 {
            return Err(MonsterError::Definition.into());
        }
        if self.profile.uses_demon_lifecycle() && l.actor_type == self.demon_owner_type() {
            self.lifecycle_owner(a)?;
        }
        let state = l.state.ok_or(MonsterError::Definition)?;
        // p_map.c:1269–1279: under the movers policy a corpse a mover crunches
        // enters S_GIBS, outside every monster type's own state graph. A static
        // decoration of the E1M1 policy passes the role gates above in its spawn
        // state alone; its full record is witnessed by `preflight_map_demons`.
        if !self.gibbed(a) {
            self.monster_state_role(l.actor_type, state)?;
        }
        Ok(*a)
    }
    pub(crate) fn monster_state_role(
        &self,
        ty: ActorTypeId,
        state: StateId,
    ) -> Result<(), SimError> {
        if self.profile.uses_demon_lifecycle() {
            return self.lifecycle_state_role(ty, state);
        }
        if self.profile == Profile::DemonMelee {
            return self.demon_state_role(ty, state);
        }
        if self.profile == Profile::Chaingunner {
            return self.chaingun_state_role(ty, state);
        }
        if self.profile == Profile::Shotgunner {
            return self.shotgun_state_role(ty, state);
        }
        let valid = if self.monster_is_owner_type(ty) {
            (149..=173).contains(&state.0)
                || [1, 2, 3, 7].contains(&state.0)
                || ty.0 == 136 && (4..=6).contains(&state.0)
        } else if ty.0 == 135 {
            (149..=173).contains(&state.0)
        } else {
            (90..=96).contains(&state.0)
        };
        if !valid {
            return Err(MonsterError::Definition.into());
        }
        Ok(())
    }
    pub(crate) fn monster_reference(&self, id: ActorId) -> Result<Actor, SimError> {
        let a = self.monster_actor(id, 1008)?;
        if [37, 38].contains(&a.life.unwrap().actor_type.0)
            || original_registry::is_missile_type(a.life.unwrap().actor_type)
            || (self.player_policy().has_world_pickups()
                && world_pickups::kind_for(self.player_policy(), a.life.unwrap().actor_type)
                    .is_some())
        {
            return Err(MonsterError::Target.into());
        }
        Ok(a)
    }
    pub(crate) fn preflight_monster_callback(&self, id: ActorId) -> Result<(), SimError> {
        self.preflight_map_demons()?;
        if !self.monster_is_owner(id) {
            return Err(MonsterError::Ownership.into());
        }
        let a = self.monster_actor(id, 1008)?;
        if self.profile.uses_demon_lifecycle() && a.life.unwrap().health <= 0 {
            return Err(MonsterError::Ownership.into());
        }
        self.preflight_perception_roster()?;
        self.monster_callback_references(&a, true)
    }
    /// The callback's last step: the owner's target and every sound target, in
    /// order. `preflight_monster_owner` calls it alone, because every earlier step
    /// of the callback already returned `Ok` on the same `&self` (M4p note §3 O1, O4).
    ///
    /// `monster_reference` depends only on `(self, id)`, so within one `&self`
    /// borrow a repeat of the id it last accepted is skipped, and the sound targets
    /// are walked only when `sounds` (the owner loop walks them once): a
    /// skipped call could only return the `Ok` already observed, and the first
    /// failing id is still checked first (M4p note §3 O2).
    fn monster_callback_references(&self, a: &Actor, sounds: bool) -> Result<(), SimError> {
        if let Some(t) = a.life.unwrap().target {
            self.monster_reference(t)?;
        }
        if sounds {
            for t in self.sound_targets.iter().flatten() {
                self.monster_reference(*t)?;
            }
        }
        Ok(())
    }
    pub(crate) fn preflight_monster_spawn(
        &self,
        ty: ActorTypeId,
        position: [Fixed; 2],
        height: SpawnHeight,
    ) -> Result<(), SimError> {
        if self.monster_is_owner_type(ty) {
            let a = lifecycle::make_actor(
                &self.world,
                self.sector_view(),
                self.registry.as_deref().unwrap(),
                self.actor_options,
                ty,
                position,
                height,
            );
            if a.z != a.floor {
                return Err(MonsterError::Ownership.into());
            }
        }
        if self.monster_is_owner_type(ty) && self.actors.order().next().is_some() {
            return Err(MonsterError::Ownership.into());
        }
        let radius = self
            .registry
            .as_deref()
            .and_then(|r| r.actor_type(ty))
            .map_or(Fixed::ZERO, ActorTypeDef::radius);
        if !self.spawn_admitted(position, radius, 1008, true)
            || matches!(height,SpawnHeight::Absolute(z) if !crate::sight::within(z,1018))
        {
            return Err(MonsterError::Arithmetic.into());
        }
        Ok(())
    }
    pub(crate) fn configure_monster_actor(
        &mut self,
        id: ActorId,
        p: PursuitSetup,
    ) -> Result<(), SimError> {
        if self.owns_player_body(id) {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.monster_actor(id, 1008)?;
        let owner = self.monster_is_owner(id);
        if !owner && self.actor(id).unwrap().life.unwrap().actor_type != self.demon_body_type() {
            return Err(MonsterError::Definition.into());
        }
        if MoveDirection::from_raw(p.movedir).is_none()
            || [p.movecount, p.reaction_time, p.threshold].contains(&i32::MIN)
            || p.reaction_time == i32::MIN + 1
        {
            return Err(MonsterError::Arithmetic.into());
        }
        if self.profile.uses_demon_lifecycle() {
            let current = self.actor(id).unwrap().life.unwrap();
            if current.flags & 1048576 != 0
                || owner
                    && (!(1..=150).contains(&p.health)
                        || p.flags
                            & !(6
                                | 32
                                | 64
                                | 128
                                | if self.profile.uses_map_statistics() {
                                    4194304
                                } else {
                                    0
                                })
                            != 0
                        || self.profile.uses_map_statistics() && owner && p.flags & 4194304 == 0
                        || p.flags & 6 != 6)
            {
                return Err(MonsterError::Definition.into());
            }
        }
        if p.flags
            & !(if owner {
                OWNER_FLAGS
                    | if self.profile.uses_map_statistics() {
                        4194304
                    } else {
                        0
                    }
                    | if self.profile.uses_demon_lifecycle() {
                        4
                    } else {
                        0
                    }
            } else {
                TARGET_FLAGS
            })
            != 0
        {
            return Err(MonsterError::Definition.into());
        }
        let a = self.actors.get_mut(id).unwrap();
        let l = a.life.as_mut().unwrap();
        l.movedir = p.movedir;
        l.movecount = p.movecount;
        l.reaction_time = p.reaction_time;
        l.threshold = p.threshold;
        l.health = p.health;
        l.flags = p.flags;
        a.spec.angle = p.angle;
        a.solidity = if p.flags & 2 != 0 {
            Solidity::Solid
        } else {
            Solidity::NonSolid
        };
        a.allow_dropoff = p.flags & 1024 != 0;
        Ok(())
    }
    pub(crate) fn preflight_monster_step(&mut self) -> Result<Option<MonsterShotPlan>, SimError> {
        self.preflight_monster_owner()?
            .map(|id| self.prepare_monster_shot(id))
            .transpose()
            .map(Option::flatten)
    }
    pub(crate) fn preflight_monster_owner(&self) -> Result<Option<ActorId>, SimError> {
        self.preflight_map_demons()?;
        self.preflight_perception_roster()?;
        let mut shot_owner = None;
        let mut sounds_passed = false;
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            // Under `uses_demon_map`, `preflight_map_demons` above ran
            // `monster_actor(id, 1008)` on every non-pending actor of this `&self` and
            // returned `Ok`; `monster_actor(id, 978)` can then differ only in its one
            // margin-dependent check, kept here (M4p note §3 O4).
            let a = if self.profile.uses_demon_map() {
                let a = *self.actor(id).ok_or(PerceptionError::Reference)?;
                if self.monster_is_owner_type(a.life.unwrap().actor_type)
                    && !self.position_admitted(a.position(), 978)
                {
                    return Err(MonsterError::Arithmetic.into());
                }
                a
            } else {
                self.monster_actor(id, 978)?
            };
            let l = a.life.unwrap();
            if self.monster_is_owner(id) && !(self.profile.uses_demon_lifecycle() && l.health <= 0)
            {
                // The callback's whole-state passes, owner test, `monster_actor(id,
                // 1008)` (implied by the 978 result) and health test all hold here.
                self.monster_callback_references(&a, !sounds_passed)?;
                sounds_passed = true;
                if l.tics == 1
                    && l.state.is_some_and(|s| {
                        if self.profile.uses_demon() {
                            s.0 == 5
                        } else if self.profile == Profile::Chaingunner {
                            [4, 5, 7].contains(&s.0)
                        } else {
                            s.0 == 4
                        }
                    })
                {
                    shot_owner = Some(id);
                }
            } else if !self.position_admitted(a.position(), 994) {
                return Err(MonsterError::Arithmetic.into());
            }
            if !crate::player_map_scale::vertical_witness(self.player_policy(), &a) {
                return Err(MonsterError::Arithmetic.into());
            }
        }
        Ok(shot_owner)
    }
    pub(crate) fn prepare_monster_shot(
        &mut self,
        id: ActorId,
    ) -> Result<Option<MonsterShotPlan>, SimError> {
        self.preflight_monster_callback(id)?;
        let a = *self.actor(id).unwrap();
        let Some(target) = a.life.unwrap().target else {
            return Ok(None);
        };
        let t = *self.actor(target).unwrap();
        let initial = self.rng;
        let mut rng = initial;
        let angle = face_angle(a, t, &mut rng);
        let range = Fixed::from_map_units(2048);
        let aim = crate::combat_trace::aim(self, id, angle, range)?;
        let first = i32::from(rng.p_random());
        let second = i32::from(rng.p_random());
        let angle = angle.wrapping_add(((first - second) * 1048576) as u32);
        let damage = (i32::from(rng.p_random()) % 5 + 1) * 3;
        let attack = self.prepare_attack(
            id,
            LineAttack {
                angle,
                range,
                slope: aim.slope,
                damage,
            },
            None,
        )?;
        if attack.result.position.is_some_and(|p| {
            !crate::sight::within(p[2], 1018)
                || p[..2].iter().any(|v| !crate::sight::within(*v, 994))
        }) {
            return Err(MonsterError::Arithmetic.into());
        }
        self.reserve_actor_spawns(attack.spawn_count())?;
        Ok(Some(MonsterShotPlan {
            owner: id,
            rng: initial,
            aim,
            attack,
        }))
    }
    pub(crate) fn enter_monster_state(
        &mut self,
        id: ActorId,
        state: StateId,
    ) -> Result<(), SimError> {
        if self.profile.uses_demon() {
            return self.enter_demon_state(id, state);
        }
        let a = self.monster_actor(id, 1008)?;
        self.monster_state_role(a.life.unwrap().actor_type, state)?;
        if self.monster_is_owner(id) {
            self.preflight_monster_callback(id)?;
        }
        self.validate_entry(state)?;
        let plan =
            if state == StateId(5) || self.profile == Profile::Chaingunner && state == StateId(6) {
                self.prepare_monster_shot(id)?
            } else {
                None
            };
        self.enter_monster_planned(id, state, plan.as_ref());
        Ok(())
    }
    /// Pure original P_CheckMeleeRange. No events, flags or RNG are changed.
    pub fn check_melee_range(&self, id: ActorId) -> Result<RangeDecision, SimError> {
        self.preflight_monster_callback(id)?;
        let a = *self.actor(id).unwrap();
        let t = a.life.unwrap().target.map(|t| *self.actor(t).unwrap());
        if !self.profile.uses_checked_demons() {
            return Ok(melee(
                &self.world,
                self.sector_view(),
                self.registry.as_deref().unwrap(),
                a,
                t,
            ));
        }
        checked_melee(
            &self.world,
            self.sector_view(),
            self.registry.as_deref().unwrap(),
            a,
            t,
            None,
        )
    }
    /// Original P_CheckMissileRange, including JUSTHIT clearing and ordered RNG.
    pub fn check_missile_range(&mut self, id: ActorId) -> Result<bool, SimError> {
        if self.original_player.is_some() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        if self.profile.uses_demon() {
            return Err(MonsterError::Profile.into());
        }
        self.preflight_monster_callback(id)?;
        if self.actor(id).unwrap().life.unwrap().target.is_none() {
            return Err(MonsterError::Target.into());
        }
        Ok(missile(&mut self.monster_host(None), id))
    }
    /// Direct source facing action, sharing the state-machine callback.
    pub fn face_target(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.original_player.is_some() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.monster_action(id, false)
    }
    /// Direct bounded source A_PosAttack; actual player damage stays unavailable.
    pub fn pos_attack(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.original_player.is_some() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.monster_action(id, true)
    }
    fn monster_action(&mut self, id: ActorId, pos: bool) -> Result<(), SimError> {
        if pos
            && matches!(
                self.profile,
                Profile::Shotgunner
                    | Profile::Chaingunner
                    | Profile::DemonMelee
                    | Profile::DemonLifecycle
                    | Profile::DemonStandRun
                    | Profile::DemonMapSpawn
                    | Profile::DemonMapContext
                    | Profile::DemonExpandedMap
                    | Profile::OriginalDemon
            )
        {
            return Err(MonsterError::Definition.into());
        }
        self.preflight_monster_callback(id)?;
        if pos && self.actor(id).unwrap().life.unwrap().actor_type != self.demon_owner_type() {
            return Err(MonsterError::Definition.into());
        }
        if self.profile.uses_checked_demons() {
            return self.checked_demon_operation(|host| {
                host.events.push(Event::ActorState(ActorStateEvent::Action {
                    actor: id,
                    action: ActorAction::Original(lifecycle::source_action_id("A_FaceTarget")),
                }));
                face(host, id);
            });
        }
        let plan = if pos {
            self.prepare_monster_shot(id)?
        } else {
            None
        };
        let mut host = self.monster_host(plan.as_ref());
        host.events.push(Event::ActorState(ActorStateEvent::Action {
            actor: id,
            action: ActorAction::Original(lifecycle::source_action_id(if pos {
                "A_PosAttack"
            } else {
                "A_FaceTarget"
            })),
        }));
        if pos {
            pos_attack(&mut host, id);
        } else {
            face(&mut host, id);
        }
        Ok(())
    }
    pub(crate) fn monster_host<'a>(
        &'a mut self,
        shot: Option<&'a MonsterShotPlan>,
    ) -> LifeHost<'a, &'a mut ThinkerStore<Actor>> {
        LifeHost {
            specials: None,
            buttons: None,
            player: None,
            statistics: self.statistics.as_mut(),
            perception: Some(crate::perception::PerceptionInputs {
                players: crate::player_roster::PerceptionRoster::Historical(
                    &self.perception_players,
                ),
                sound_targets: &self.sound_targets,
                pursuit: true,
                monsters: true,
                shotgunner: self.profile == Profile::Shotgunner,
                chaingunner: self.profile == Profile::Chaingunner,
                demon: self.profile.uses_demon(),
                map_demons: self.profile.uses_demon_map(),
                checked: None,
                melee: None,
                shotgun: None,
                shot,
            }),
            access: &mut self.actors,
            registry: self.registry.as_deref().unwrap(),
            world: &self.world,
            sectors: crate::staging::Sectors::committed(&self.sectors),
            options: self.actor_options,
            rng: &mut self.rng,
            spatial: &mut self.spatial,
            bindings: &mut self.bindings,
            events: &mut self.pending,
        }
    }
}
fn validate_registry(r: &ActorRegistry) -> Result<(), SimError> {
    let source = ActorRegistry::vanilla();
    if r.states() == source.states() && r.types() == source.types() {
        return Ok(());
    }
    if r.states().len() != source.states().len() || r.types().len() != source.types().len() {
        return Err(MonsterError::Definition.into());
    }
    for (i, s) in r.states().iter().enumerate() {
        if !(1..=7).contains(&i) {
            if s != &source.states()[i] {
                return Err(MonsterError::Definition.into());
            }
            continue;
        }
        let names = [
            "",
            "FIXTURE_LOOK",
            "FIXTURE_CHASE",
            "FIXTURE_MELEE_MARKER",
            "FIXTURE_POSS_ATK1",
            "FIXTURE_POSS_ATK2",
            "FIXTURE_POSS_ATK3",
            "FIXTURE_MISSILE_MARKER",
        ];
        let (frame, tics, action, next) = match i {
            1 => (0, s.tics, Some("A_Look"), 1),
            2 => (1, s.tics, Some("A_Chase"), 2),
            3 | 7 => (0, s.tics, None, 2),
            4 => (4, 10, Some("A_FaceTarget"), 5),
            5 => (5, 8, Some("A_PosAttack"), 6),
            6 => (4, 8, None, 2),
            _ => unreachable!(),
        };
        let action = action.map_or(ActorAction::None, |a| {
            ActorAction::Original(lifecycle::source_action_id(a))
        });
        if s.name != names[i]
            || s.sprite != 29
            || s.frame != frame
            || s.tics != tics
            || !(1..=35).contains(&s.tics)
            || s.action != action
            || s.next != StateId(next)
            || s.misc != [0; 2]
        {
            return Err(MonsterError::Definition.into());
        }
    }
    for (i, t) in r.types().iter().enumerate() {
        if !OWNERS.contains(&(i as u16)) && i != 135 {
            if t != &source.types()[i] {
                return Err(MonsterError::Definition.into());
            }
            continue;
        }
        let owner = i != 135;
        let name = if owner {
            format!("FIXTURE_ATTACKER_{i}")
        } else {
            "FIXTURE_DAMAGE_TARGET".into()
        };
        let mut expected = source.types()[0].clone();
        expected.name = name;
        expected.properties[0] = -1;
        let variable: &[usize] = if owner {
            &[4, 6, 10, 11, 15, 16, 17, 20, 21]
        } else {
            &[2, 8, 16, 17, 21]
        };
        for &j in variable {
            expected.properties[j] = t.properties[j];
        }
        if owner {
            expected.properties[1] = 1;
            expected.properties[3] = 2;
            expected.properties[5] = 8;
        }
        if *t != expected
            || t.radius() < Fixed::ZERO
            || t.radius() > Fixed::from_map_units(32)
            || t.height() <= Fixed::ZERO
            || t.flags()
                & !(if owner {
                    OWNER_FLAGS
                } else {
                    TARGET_FLAGS & !1048576
                })
                != 0
        {
            return Err(MonsterError::Definition.into());
        }
        if owner {
            if !(1..=16).contains(&t.speed())
                || ![0, 3].contains(&t.states()[3].0)
                || !([0, 7].contains(&t.states()[4].0) || i == 136 && t.states()[4] == StateId(4))
            {
                return Err(MonsterError::Definition.into());
            }
        } else if !(1..=1000).contains(&t.spawn_health()) || !(0..=256).contains(&t.pain_chance()) {
            return Err(MonsterError::Definition.into());
        }
    }
    Ok(())
}
pub(crate) fn event_bytes(b: &mut Vec<u8>, e: &MonsterEvent) {
    fn id(b: &mut Vec<u8>, a: ActorId) {
        b.extend(a.slot().to_le_bytes());
        b.extend(a.generation().to_le_bytes());
    }
    fn opt(b: &mut Vec<u8>, a: Option<ActorId>) {
        b.push(u8::from(a.is_some()));
        id(b, a.unwrap_or(ActorId::from_parts(0, 0)));
    }
    match e {
        MonsterEvent::Range {
            actor,
            kind,
            decision: d,
        } => {
            b.push(0);
            id(b, *actor);
            b.extend([*kind, u8::from(d.accepted), d.reason]);
            for v in [d.distance, d.threshold] {
                b.push(u8::from(v.is_some()));
                if let Some(v) = v {
                    b.extend(v.to_le_bytes());
                }
            }
        }
        MonsterEvent::Random { actor, kind, value } => {
            b.push(1);
            id(b, *actor);
            b.extend([*kind, *value]);
        }
        MonsterEvent::Face {
            actor,
            target,
            angles,
            flags,
        } => {
            b.push(2);
            id(b, *actor);
            opt(b, *target);
            for v in angles.iter().chain(flags) {
                b.extend(v.to_le_bytes());
            }
        }
        MonsterEvent::Selection { actor, kind, state } => {
            b.push(3);
            id(b, *actor);
            b.push(*kind);
            b.extend(state.0.to_le_bytes());
        }
        MonsterEvent::JustAttacked { actor, flags } => {
            b.push(4);
            id(b, *actor);
            b.extend(flags.to_le_bytes());
        }
        MonsterEvent::Aim { actor, result } => {
            b.push(5);
            id(b, *actor);
            crate::snapshot::aim_bytes(b, result);
        }
    }
}
