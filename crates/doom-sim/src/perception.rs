//! Bounded source sight and explicit perception profile composition.
use crate::*;
use std::{error::Error, fmt};
/// Admission/identity/work rejection before authoritative perception changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PerceptionError {
    /// Explicit profile 8 required.
    Profile,
    /// Invalid, pending, retired or stale actor.
    Reference,
    /// Unsupported source/fixture metadata or action handoff.
    Definition,
    /// Source-compatible arithmetic slice exceeded.
    Arithmetic,
    /// Actor or expanded traversal bound exceeded.
    WorkLimit,
    /// Perception roster configuration is invalid.
    Roster,
    /// Retained sound target is pending, retired or stale.
    SoundTarget,
}
impl fmt::Display for PerceptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "perception: {self:?}")
    }
}
impl Error for PerceptionError {}
impl Simulation {
    /// Explicit profile 8; no roster, weapon player, chase or attack is implicit.
    pub fn monster_perception(map: Map) -> Result<Self, SimError> {
        Self::with_perception_registry(map, 64, ActorRegistry::vanilla(), ActorOptions::default())
    }
    /// Immutable original definitions; fixture admission is explicitly constrained.
    pub fn with_perception_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if limit > 64 {
            return Err(PerceptionError::WorkLimit.into());
        }
        crate::sight::validate_map(&map)?;
        let source = ActorRegistry::vanilla();
        validate_registry(&registry, &source)?;
        let mut sim = Self::with_actor_registry(map, limit, registry, options)?;
        sim.profile = Profile::Perception;
        sim.sound_targets = vec![None; sim.sectors.len()];
        Ok(sim)
    }
    pub(crate) fn perception_actor(&self, id: ActorId) -> Result<Actor, SimError> {
        self.perception_actor_ref(id).copied()
    }
    /// [`Self::perception_actor`] without the copy: the hot validators read the
    /// stored actor in place and copy it once, when they return it.
    pub(crate) fn perception_actor_ref(&self, id: ActorId) -> Result<&Actor, SimError> {
        if !self.profile.uses_perception() {
            return Err(PerceptionError::Profile.into());
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(PerceptionError::Reference.into());
        }
        let a = self.actor(id).ok_or(PerceptionError::Reference)?;
        crate::sight::actor_bounds_within(a, self.sight_margin())?;
        Ok(a)
    }
    /// Original read-only REJECT/BSP sight, independent of roster/RNG/presentation.
    pub fn check_sight(&self, observer: ActorId, target: ActorId) -> Result<SightResult, SimError> {
        if self.profile == Profile::MapExtent || self.profile.uses_checked_demons() {
            return crate::sight::try_extent_query(
                &self.world,
                self.sector_view(),
                if self.profile.uses_perception() {
                    self.perception_actor(observer)?
                } else {
                    self.map_extent_sight_actor(observer)?
                },
                if self.profile.uses_perception() {
                    self.perception_actor(target)?
                } else {
                    self.map_extent_sight_actor(target)?
                },
            );
        }
        let (observer, target) = if self.profile == Profile::MapScale {
            (
                self.map_scale_sight_actor(observer)?,
                self.map_scale_sight_actor(target)?,
            )
        } else {
            (
                self.perception_actor(observer)?,
                self.perception_actor(target)?,
            )
        };
        Ok(crate::sight::query(
            &self.world,
            self.sector_view(),
            observer,
            target,
        ))
    }
}

/// Source roster view; activity, health and actor identity are distinct.
/// Historical profiles register fixtures; the genuine-player policy derives this
/// view from its existing canonical owner instead of maintaining a second record.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PerceptionPlayer {
    /// Corresponds to `playeringame[slot]`, not a network connection.
    pub active: bool,
    /// Source player.health used by scanning, independent of mobj health.
    pub health: i32,
    /// Generational body identity, subject to the selected role/lifetime contract.
    /// Inactive historical records may retain old IDs.
    pub actor: Option<ActorId>,
}
/// Source perception decisions, in the order actually evaluated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PerceptionEvent {
    /// Before the source threshold reset.
    LookStart {
        /// Observer.
        actor: ActorId,
        /// Previous threshold.
        threshold: i32,
    },
    /// Source sound target before assignment, ambush sight or player fallback.
    LookSound {
        /// Observer.
        actor: ActorId,
        /// Retained sector target.
        target: Option<ActorId>,
        /// Current shootability bit.
        shootable: bool,
        /// Source AMBUSH bit.
        ambush: bool,
    },
    /// Source wake sound RNG, after detection and before sound/state entry.
    Random {
        /// Observer.
        actor: ActorId,
        /// Original lookup result.
        value: u8,
    },
    /// Immediately before the validated see-state entry.
    Wake {
        /// Observer.
        actor: ActorId,
        /// Metadata see state.
        state: StateId,
    },
    /// Start at current lastlook; no nearest-target sorting.
    ScanStart {
        /// Observer.
        actor: ActorId,
        /// Source allaround request.
        allaround: bool,
        /// Initial slot.
        lastlook: u8,
        /// Source (lastlook-1)&3.
        stop: u8,
    },
    /// Slot before the active filter and c++ stopping condition.
    Slot {
        /// Observer.
        actor: ActorId,
        /// Slot considered.
        slot: u8,
        /// playeringame value.
        active: bool,
        /// Active encounters before this slot.
        count: u8,
    },
    /// Player-record health before sight; not the actor's health.
    Health {
        /// Observer.
        actor: ActorId,
        /// Active slot.
        slot: u8,
        /// Signed source health.
        health: i32,
    },
    /// Directional test after successful sight; distance evaluated only in rear.
    Facing {
        /// Observer.
        actor: ActorId,
        /// Candidate slot.
        slot: u8,
        /// Wrapping BAM angle relative to observer.
        angle: u32,
        /// Approximate rear distance, absent for front candidates.
        distance: Option<Fixed>,
        /// Source directional/close-range decision.
        accepted: bool,
    },
    /// Safe empty-roster termination where original C would loop forever.
    EmptyRoster(ActorId),
    /// Return with lastlook at the exact current source slot.
    ScanEnd {
        /// Observer.
        actor: ActorId,
        /// Whether target assignment occurred.
        found: bool,
        /// Final slot.
        lastlook: u8,
    },
    /// Local sight evidence for a mutating source perception operation.
    Sight {
        /// Observer.
        observer: ActorId,
        /// Candidate or sound target.
        target: ActorId,
        /// Complete read-only result/decision order.
        result: SightResult,
    },
}
pub(crate) struct ScanPlan {
    pub lastlook: u8,
    pub target: Option<ActorId>,
    pub events: Vec<Event>,
}
fn perception_event(events: &mut Vec<Event>, e: PerceptionEvent) {
    events.push(Event::Perception(e));
}
pub(crate) fn scan(
    world: &StaticWorld,
    sectors: crate::staging::Sectors<'_>,
    roster: &[PerceptionPlayer; 4],
    get: &impl Fn(ActorId) -> Option<Actor>,
    id: ActorId,
    allaround: bool,
    checked: Option<&crate::demon_expanded_map::SightControl>,
) -> Result<ScanPlan, SimError> {
    let observer = get(id).ok_or(PerceptionError::Reference)?;
    if observer.lastlook >= 4 {
        return Err(PerceptionError::Roster.into());
    }
    let mut lastlook = observer.lastlook;
    let stop = lastlook.wrapping_sub(1) & 3;
    let mut plan = ScanPlan {
        lastlook,
        target: None,
        events: Vec::new(),
    };
    perception_event(
        &mut plan.events,
        PerceptionEvent::ScanStart {
            actor: id,
            allaround,
            lastlook,
            stop,
        },
    );
    if !roster.iter().any(|p| p.active) {
        perception_event(&mut plan.events, PerceptionEvent::EmptyRoster(id));
        return Ok(plan);
    }
    let mut count = 0_u8;
    for _ in 0..12 {
        let p = roster[usize::from(lastlook)];
        perception_event(
            &mut plan.events,
            PerceptionEvent::Slot {
                actor: id,
                slot: lastlook,
                active: p.active,
                count,
            },
        );
        if p.active {
            let prior = count;
            count += 1;
            if prior == 2 || lastlook == stop {
                plan.lastlook = lastlook;
                return Ok(plan);
            }
            perception_event(
                &mut plan.events,
                PerceptionEvent::Health {
                    actor: id,
                    slot: lastlook,
                    health: p.health,
                },
            );
            if p.health > 0 {
                let target = p.actor.ok_or(PerceptionError::Roster)?;
                let a = get(target).ok_or(PerceptionError::Reference)?;
                let result =
                    crate::demon_expanded_map::action_sight(checked, world, sectors, observer, a)?;
                let visible = result.visible;
                perception_event(
                    &mut plan.events,
                    PerceptionEvent::Sight {
                        observer: id,
                        target,
                        result,
                    },
                );
                if visible {
                    let mut accepted = true;
                    if !allaround {
                        let delta = [
                            a.position()[0] - observer.position()[0],
                            a.position()[1] - observer.position()[1],
                        ];
                        let angle = movement::point_angle(delta).wrapping_sub(observer.angle());
                        let rear = angle > 0x40000000 && angle < 0xc0000000;
                        let distance = rear.then(|| movement::approx_distance(delta));
                        accepted = distance.is_none_or(|d| d <= Fixed::from_map_units(64));
                        perception_event(
                            &mut plan.events,
                            PerceptionEvent::Facing {
                                actor: id,
                                slot: lastlook,
                                angle,
                                distance,
                                accepted,
                            },
                        );
                    }
                    if accepted {
                        plan.lastlook = lastlook;
                        plan.target = Some(target);
                        return Ok(plan);
                    }
                }
            }
        }
        lastlook = (lastlook + 1) & 3;
    }
    Err(PerceptionError::WorkLimit.into())
}
pub(crate) fn commit_scan<A: crate::lifecycle::ActorAccess>(
    host: &mut crate::lifecycle::LifeHost<'_, A>,
    id: ActorId,
    plan: ScanPlan,
) -> bool {
    host.events.extend(plan.events);
    host.access.get_mut(id).unwrap().lastlook = plan.lastlook;
    if let Some(target) = plan.target {
        host.target(id, Some(target));
    }
    let found = plan.target.is_some();
    host.events
        .push(Event::Perception(PerceptionEvent::ScanEnd {
            actor: id,
            found,
            lastlook: plan.lastlook,
        }));
    found
}
impl Simulation {
    /// Read the historical fixture roster. For the genuine fixed-active policy,
    /// use [`Self::original_player_roster`] to observe current player fields.
    pub fn perception_players(&self) -> &[PerceptionPlayer; 4] {
        &self.perception_players
    }
    /// Definition-role policy shared by read-only reference observation and
    /// acquisition. Physical/action validation remains in the latter below.
    pub(crate) fn validate_perception_target_type(&self, ty: ActorTypeId) -> Result<(), SimError> {
        let expected = if self.uses_active_player_roster() {
            ActorTypeId(crate::identity::OriginalActorType::MT_PLAYER.ordinal())
        } else {
            self.demon_body_type()
        };
        if self.profile.uses_demon_map() && ty != expected {
            return Err(MapSpawnError::Target.into());
        }
        Ok(())
    }
    pub(crate) fn validate_perception_reference(&self, id: ActorId) -> Result<Actor, SimError> {
        if self.uses_active_player_roster() && !self.owns_player_body(id) {
            // P_DamageMobj infighting may retarget a roster monster to another.
            if self.player_policy().has_monsters()
                && self
                    .actor(id)
                    .and_then(Actor::life)
                    .is_some_and(|l| self.is_roster_monster(l.actor_type))
            {
                return self.monster_actor(id, 1008);
            }
            return Err(PerceptionError::Roster.into());
        }
        let a = self.perception_actor(id)?;
        let life = a.life.unwrap();
        let registry = self.registry.as_deref().unwrap();
        self.validate_perception_target_type(life.actor_type)?;
        if self.profile.uses_monsters() {
            return self.monster_reference(id);
        }
        let fixture = [19, 21, 136].contains(&life.actor_type.0)
            && registry.actor_type(life.actor_type).unwrap().states()[..2]
                == [StateId(1), StateId(2)];
        if life.actor_type != ActorTypeId(0) && !fixture {
            return Err(PerceptionError::Roster.into());
        }
        let valid = life
            .state
            .is_some_and(|s| (149..=173).contains(&s.0) || fixture && [1, 2].contains(&s.0));
        if !valid {
            return Err(PerceptionError::Roster.into());
        }
        Ok(a)
    }
    pub(crate) fn preflight_perception_roster(&self) -> Result<(), SimError> {
        if self.uses_active_player_roster() {
            return self.preflight_original_roster();
        }
        for p in self.perception_players {
            if p.active {
                self.validate_perception_reference(p.actor.ok_or(PerceptionError::Roster)?)?;
            }
        }
        Ok(())
    }
    /// Explicit fixture/player registration; no full player gameplay or damage.
    pub fn set_perception_player(
        &mut self,
        slot: usize,
        p: PerceptionPlayer,
    ) -> Result<(), SimError> {
        if self.original_player.is_some() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        if !self.profile.uses_perception() {
            return Err(PerceptionError::Profile.into());
        }
        if slot >= 4 || p.active && p.actor.is_none() {
            return Err(PerceptionError::Roster.into());
        }
        if let Some(id) = p.actor {
            self.validate_perception_reference(id)?;
        }
        self.perception_players[slot] = p;
        Ok(())
    }
    /// Between-tic source observer angle/threshold setup, leaving lastlook untouched.
    pub fn configure_perception_observer(
        &mut self,
        id: ActorId,
        angle: u32,
        threshold: i32,
    ) -> Result<(), SimError> {
        if self.owns_player_body(id) {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.perception_actor(id)?;
        if (self.profile == Profile::Pursuit || self.profile.uses_monsters())
            && threshold == i32::MIN
        {
            return Err(PursuitError::Arithmetic.into());
        }
        let a = self.actors.get_mut(id).unwrap();
        a.spec.angle = angle;
        a.life.as_mut().unwrap().threshold = threshold;
        Ok(())
    }
    /// Exact P_LookForPlayers selection. Ok(false) may still advance lastlook.
    pub fn look_for_players(&mut self, id: ActorId, allaround: bool) -> Result<bool, SimError> {
        if self.uses_active_player_roster() {
            self.preflight_monster_callback(id)?;
            return self.checked_demon_operation(|host| {
                let plan = crate::player_roster::scan_current(host, id, allaround)?;
                Ok(commit_scan(host, id, plan))
            })?;
        }
        if self.original_player.is_some() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.perception_actor(id)?;
        self.preflight_perception_roster()?;
        let plan = scan(
            &self.world,
            self.sector_view(),
            &self.perception_players,
            &|id| self.actor(id).copied(),
            id,
            allaround,
            None,
        )?;
        Ok(commit_scan(
            &mut crate::lifecycle::LifeHost {
                specials: None,
                buttons: None,
                player: None,
                statistics: self.statistics.as_mut(),
                perception: None,
                access: &mut self.actors,
                registry: self.registry.as_deref().unwrap(),
                world: &self.world,
                sectors: crate::staging::Sectors::committed(&self.sectors),
                options: self.actor_options,
                rng: &mut self.rng,
                spatial: &mut self.spatial,
                bindings: &mut self.bindings,
                events: &mut self.pending,
            },
            id,
            plan,
        ))
    }
    pub(crate) fn preflight_perception_step(&self) -> Result<(), SimError> {
        self.preflight_perception_roster()?;
        if self.bindings.iter().any(Option::is_some) || self.weapon_player.is_some() {
            return Err(PerceptionError::Roster.into());
        }
        let mut has_looker = false;
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            let a = self.actor(id).unwrap();
            self.validate_perception_type(a.life.unwrap().actor_type)?;
            let mut state = a.life.unwrap().state.unwrap();
            let mut seen = std::collections::BTreeSet::new();
            let mut looking = false;
            while state.0 != 0 && seen.insert(state) {
                let def = self.registry.as_deref().unwrap().state(state).unwrap();
                looking |= matches!(def.action,ActorAction::Original(action) if self.registry.as_deref().unwrap().action_name(action)==Some("A_Look"));
                if def.tics == -1 {
                    break;
                }
                state = def.next;
            }
            if looking {
                has_looker = true;
                self.preflight_perception_look(id)?;
            }
            crate::sight::actor_bounds(a)?;
            for axis in 0..2 {
                if a.momentum()[axis] != Fixed::ZERO
                    && !crate::sight::within(a.position()[axis], 994)
                {
                    return Err(PerceptionError::Arithmetic.into());
                }
            }
            if !crate::sight::within(a.momz, 64) || !crate::sight::within(a.z + a.momz, 1024) {
                return Err(PerceptionError::Arithmetic.into());
            }
        }
        // XY can change a looker's sector before its action. Validate every
        // retained sound identity before the tic, not just the old sector.
        if has_looker {
            for target in self.sound_targets.iter().flatten() {
                self.validate_perception_reference(*target)
                    .map_err(|_| PerceptionError::SoundTarget)?;
            }
        }
        Ok(())
    }
}

fn identity(b: &mut Vec<u8>, id: ActorId) {
    b.extend(id.slot().to_le_bytes());
    b.extend(id.generation().to_le_bytes());
}
pub(crate) fn event_bytes(b: &mut Vec<u8>, e: &PerceptionEvent) {
    match e {
        PerceptionEvent::LookStart { actor, threshold } => {
            b.push(0);
            identity(b, *actor);
            b.extend(threshold.to_le_bytes());
        }
        PerceptionEvent::LookSound {
            actor,
            target,
            shootable,
            ambush,
        } => {
            b.push(1);
            identity(b, *actor);
            if let Some(t) = target {
                b.push(1);
                identity(b, *t);
            } else {
                b.extend([0; 13]);
            }
            b.extend([u8::from(*shootable), u8::from(*ambush)]);
        }
        PerceptionEvent::Wake { actor, state } => {
            b.push(8);
            identity(b, *actor);
            b.extend(state.0.to_le_bytes());
        }
        PerceptionEvent::Random { actor, value } => {
            b.push(9);
            identity(b, *actor);
            b.push(*value);
        }
        PerceptionEvent::ScanStart {
            actor,
            allaround,
            lastlook,
            stop,
        } => {
            b.push(2);
            identity(b, *actor);
            b.extend([u8::from(*allaround), *lastlook, *stop]);
        }
        PerceptionEvent::Slot {
            actor,
            slot,
            active,
            count,
        } => {
            b.push(3);
            identity(b, *actor);
            b.extend([*slot, u8::from(*active), *count]);
        }
        PerceptionEvent::Health {
            actor,
            slot,
            health,
        } => {
            b.push(4);
            identity(b, *actor);
            b.push(*slot);
            b.extend(health.to_le_bytes());
        }
        PerceptionEvent::Facing {
            actor,
            slot,
            angle,
            distance,
            accepted,
        } => {
            b.push(5);
            identity(b, *actor);
            b.push(*slot);
            b.extend(angle.to_le_bytes());
            b.push(u8::from(distance.is_some()));
            if let Some(d) = distance {
                b.extend(d.bits().to_le_bytes());
            }
            b.push(u8::from(*accepted));
        }
        PerceptionEvent::EmptyRoster(actor) => {
            b.push(6);
            identity(b, *actor);
        }
        PerceptionEvent::ScanEnd {
            actor,
            found,
            lastlook,
        } => {
            b.push(7);
            identity(b, *actor);
            b.extend([u8::from(*found), *lastlook]);
        }
        PerceptionEvent::Sight {
            observer,
            target,
            result: r,
        } => {
            b.push(10);
            identity(b, *observer);
            identity(b, *target);
            b.push(u8::from(r.visible));
            b.extend(r.reject_index.to_le_bytes());
            b.extend([u8::from(r.rejected), u8::from(r.eye.is_some())]);
            if let Some(eye) = r.eye {
                b.extend(eye.bits().to_le_bytes());
                for s in r.slopes.unwrap() {
                    b.extend(s.bits().to_le_bytes());
                }
            }
            b.extend((r.visits.len() as u32).to_le_bytes());
            for v in &r.visits {
                match v {
                    SightVisit::NodeStart { node, side } | SightVisit::NodeEnd { node, side } => {
                        b.push(if matches!(v, SightVisit::NodeStart { .. }) {
                            1
                        } else {
                            2
                        });
                        b.extend(node.to_le_bytes());
                        b.push(*side);
                    }
                    SightVisit::Subsector(i) => {
                        b.push(3);
                        b.extend(i.to_le_bytes());
                    }
                    SightVisit::Seg {
                        subsector,
                        seg,
                        line,
                        duplicate,
                    } => {
                        b.push(4);
                        b.extend(subsector.to_le_bytes());
                        b.extend(seg.to_le_bytes());
                        b.extend(line.to_le_bytes());
                        b.push(u8::from(*duplicate));
                    }
                    SightVisit::Sides { line, stage, sides } => {
                        b.push(5);
                        b.extend(line.to_le_bytes());
                        b.extend([*stage, sides[0], sides[1]]);
                    }
                    SightVisit::Opening { line, heights } => {
                        b.push(6);
                        b.extend(line.to_le_bytes());
                        for h in heights {
                            b.extend(h.bits().to_le_bytes());
                        }
                    }
                    SightVisit::Clip {
                        line,
                        fraction,
                        slopes,
                    } => {
                        b.push(7);
                        b.extend(line.to_le_bytes());
                        for v in [*fraction, slopes[0], slopes[1]] {
                            b.extend(v.bits().to_le_bytes());
                        }
                    }
                    SightVisit::Blocked { line, reason } => {
                        b.push(8);
                        b.extend(line.to_le_bytes());
                        b.push(*reason);
                    }
                }
            }
        }
    }
}

impl Simulation {
    /// Original P_NoiseAlert(target, emitter), sharing profile-6 propagation.
    /// This does not fire a weapon or require the target to be a roster player.
    pub fn noise_alert(&mut self, target: ActorId, emitter: ActorId) -> Result<(), SimError> {
        if self.original_player.is_some() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.validate_perception_reference(target)?;
        self.perception_actor(emitter)?;
        self.emit_noise(target, emitter);
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PerceptionInputs<'a> {
    pub checked: Option<&'a crate::demon_expanded_map::SightControl>,
    pub players: crate::player_roster::PerceptionRoster<'a>,
    pub pursuit: bool,
    pub monsters: bool,
    pub shotgunner: bool,
    pub chaingunner: bool,
    pub demon: bool,
    pub map_demons: bool,
    pub melee: Option<&'a crate::demon::MeleePlan>,
    pub shotgun: Option<&'a crate::shotgunner::ShotgunPlan>,
    pub shot: Option<&'a crate::monsters::MonsterShotPlan>,
    pub sound_targets: &'a [Option<ActorId>],
}
/// The original A_Look body, reached only after complete admission/preflight.
pub(crate) fn look<A: crate::lifecycle::ActorAccess>(
    host: &mut crate::lifecycle::LifeHost<'_, A>,
    id: ActorId,
    work: &mut usize,
) -> bool {
    let input = host.perception.expect("profile-8 perception context");
    let observer = *host.access.get(id).unwrap();
    let life = observer.life.unwrap();
    host.events
        .push(Event::Perception(PerceptionEvent::LookStart {
            actor: id,
            threshold: life.threshold,
        }));
    host.access
        .get_mut(id)
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .threshold = 0;
    let sound = input.sound_targets[usize::from(observer.location.unwrap().sector)];
    let shootable = sound.is_some_and(|t| host.access.get(t).unwrap().life.unwrap().flags & 4 != 0);
    let ambush = life.flags & 32 != 0;
    host.events
        .push(Event::Perception(PerceptionEvent::LookSound {
            actor: id,
            target: sound,
            shootable,
            ambush,
        }));
    let mut heard = false;
    if shootable {
        let target = sound.unwrap();
        host.target(id, Some(target));
        if ambush {
            let result = match crate::demon_expanded_map::action_sight(
                input.checked,
                host.world,
                host.sectors,
                observer,
                *host.access.get(target).unwrap(),
            ) {
                Ok(result) => result,
                Err(error) => {
                    host.fail_sight(error);
                    return false;
                }
            };
            heard = result.visible;
            host.events.push(Event::Perception(PerceptionEvent::Sight {
                observer: id,
                target,
                result,
            }));
        } else {
            heard = true
        }
    }
    if !heard {
        let plan = crate::player_roster::scan_current(host, id, false);
        let plan = match plan {
            Ok(plan) => plan,
            Err(error) => {
                host.fail_sight(error);
                return false;
            }
        };
        if !commit_scan(host, id, plan) {
            return false;
        }
    }
    let info = host.registry.actor_type(life.actor_type).unwrap();
    let state = info.states()[1];
    let sound = info.sounds()[0];
    if sound != 0 {
        let posit = crate::lifecycle::source_sound_id("sfx_posit1");
        let bgsit = crate::lifecycle::source_sound_id("sfx_bgsit1");
        let group = if (posit..=posit + 2).contains(&sound) {
            Some((posit, 3))
        } else if (bgsit..=bgsit + 1).contains(&sound) {
            Some((bgsit, 2))
        } else {
            None
        };
        let chosen = if let Some((base, modulus)) = group {
            let value = host.rng.p_random();
            host.events.push(Event::Perception(PerceptionEvent::Random {
                actor: id,
                value,
            }));
            base + u16::from(value % modulus)
        } else {
            sound
        };
        let global = [19, 21].contains(&life.actor_type.0);
        host.sound(if global { None } else { Some(id) }, chosen);
    }
    host.events.push(Event::Perception(PerceptionEvent::Wake {
        actor: id,
        state,
    }));
    host.enter_counted(id, state, work);
    true
}
impl Simulation {
    pub(crate) fn validate_perception_type(&self, ty: ActorTypeId) -> Result<(), SimError> {
        if self.profile.uses_monsters() {
            return self.validate_monster_type(ty);
        }
        if self.profile == Profile::Pursuit {
            return self.validate_pursuit_type(ty);
        }
        self.registry
            .as_deref()
            .unwrap()
            .validate_type_perception(ty, &self.sectors)?;
        Ok(())
    }
    fn preflight_perception_look(&self, id: ActorId) -> Result<(), SimError> {
        if self.profile.uses_monsters() {
            return self.preflight_monster_callback(id);
        }
        if self.profile == Profile::Pursuit {
            self.preflight_pursuit_callback(id)?;
        }
        let a = self.perception_actor(id)?;
        let ty = a.life.unwrap().actor_type;
        let info = self.registry.as_deref().unwrap().actor_type(ty).unwrap();
        if ![19, 21, 136].contains(&ty.0) || info.states()[..2] != [StateId(1), StateId(2)] {
            return Err(PerceptionError::Definition.into());
        }
        self.validate_perception_type(ty)?;
        self.preflight_perception_roster()?;
        if let Some(target) = self.sound_targets[usize::from(a.location.unwrap().sector)] {
            self.validate_perception_reference(target)
                .map_err(|_| PerceptionError::SoundTarget)?;
        }
        Ok(())
    }
    pub(crate) fn preflight_perception_entry(
        &self,
        id: ActorId,
        state: StateId,
    ) -> Result<(), SimError> {
        self.perception_actor(id)?;
        self.preflight_perception_roster()?;
        let registry = self.registry.as_deref().unwrap();
        let mut cursor = state;
        let mut seen = std::collections::BTreeSet::new();
        while cursor.0 != 0 && seen.insert(cursor) {
            let def = registry.state(cursor).ok_or(ActorError::InvalidState)?;
            if matches!(def.action,ActorAction::Original(a) if registry.action_name(a)==Some("A_Look"))
            {
                self.preflight_perception_look(id)?;
            }
            cursor = def.next;
        }
        let referenced = self
            .perception_players
            .iter()
            .any(|p| p.active && p.actor == Some(id))
            || self.sound_targets.contains(&Some(id));
        if referenced {
            let a = self.actor(id).unwrap();
            let fixture = a.life.unwrap().actor_type != ActorTypeId(0);
            if !(149..=173).contains(&state.0) && !(fixture && [1, 2].contains(&state.0)) {
                return Err(PerceptionError::Roster.into());
            }
        }
        Ok(())
    }
    /// Execute admitted original A_Look between tics. Ok(false) is a normal
    /// no-wake decision and may still change threshold/target/lastlook/events.
    pub fn look(&mut self, id: ActorId) -> Result<bool, SimError> {
        if self.original_player.is_some() && !self.uses_active_player_roster() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.preflight_perception_look(id)?;
        if self.profile.uses_checked_demons() {
            return self.checked_demon_operation(|host| host.invoke_look(id));
        }
        Ok(crate::lifecycle::LifeHost {
            specials: None,
            buttons: None,
            player: None,
            statistics: self.statistics.as_mut(),
            perception: Some(PerceptionInputs {
                pursuit: (self.profile == Profile::Pursuit || self.profile.uses_monsters()),
                monsters: self.profile.uses_monsters(),
                shotgunner: self.profile == Profile::Shotgunner,
                chaingunner: self.profile == Profile::Chaingunner,
                demon: self.profile.uses_demon(),
                map_demons: self.profile.uses_demon_map(),
                checked: None,
                melee: None,
                shotgun: None,
                shot: None,
                players: crate::player_roster::PerceptionRoster::Historical(
                    &self.perception_players,
                ),
                sound_targets: &self.sound_targets,
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
        .invoke_look(id))
    }
}

pub(crate) fn validate_registry(r: &ActorRegistry, source: &ActorRegistry) -> Result<(), SimError> {
    if r.states() == source.states() && r.types() == source.types() {
        return Ok(());
    }
    if r.states().len() != source.states().len() || r.types().len() != source.types().len() {
        return Err(PerceptionError::Definition.into());
    }
    for (i, s) in r.states().iter().enumerate() {
        if (i > 2 || i == 0) && s != &source.states()[i] {
            return Err(PerceptionError::Definition.into());
        }
    }
    let a = &r.states()[1];
    let b = &r.states()[2];
    if a.name != "FIXTURE_LOOK"
        || a.sprite != 29
        || a.frame != 0
        || !(1..=35).contains(&a.tics)
        || a.action != ActorAction::Original(crate::lifecycle::source_action_id("A_Look"))
        || a.next != StateId(1)
        || a.misc != [0, 0]
        || b.name != "FIXTURE_SEE"
        || b.sprite != 29
        || b.frame != 1
        || b.tics != -1
        || b.action != ActorAction::None
        || b.next != StateId(0)
        || b.misc != [0, 0]
    {
        return Err(PerceptionError::Definition.into());
    }
    for (i, t) in r.types().iter().enumerate() {
        if ![19, 21, 136].contains(&i) {
            if t != &source.types()[i] {
                return Err(PerceptionError::Definition.into());
            }
            continue;
        }
        if t.name != format!("FIXTURE_LOOKER_{i}")
            || t.properties[0] != -1
            || t.properties[1] != 1
            || t.properties[3] != 2
            || t.properties[5] != 8
        {
            return Err(PerceptionError::Definition.into());
        }
        for (j, (value, base)) in t
            .properties
            .iter()
            .zip(source.types()[0].properties.iter())
            .enumerate()
        {
            if ![0, 1, 3, 4, 5, 17, 21].contains(&j) && value != base {
                return Err(PerceptionError::Definition.into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "perception_tests.rs"]
mod tests;
