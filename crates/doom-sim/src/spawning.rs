//! Profile-4 setup. Recoverable validation always precedes gameplay mutation.
use crate::{
    lifecycle::{Demand, LifeHost},
    *,
};

/// Results for the supported single-player THINGS subset, in source order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MapSpawnReport {
    /// Admitted ordinary actors.
    pub spawned: Vec<ActorId>,
    /// Deferred starts; complete P_SpawnPlayer remains unsupported.
    pub player_starts: Vec<doom_map::Thing>,
    /// First ten deathmatch starts, collected before ordinary filters.
    pub deathmatch_starts: Vec<doom_map::Thing>,
    /// Placements skipped by single-player, skill or nomonsters rules.
    pub filtered: usize,
    /// The player body the level policy spawned at its type-1 record (decision
    /// G4); `None` under every other policy, whose caller spawns the body.
    pub player_body: Option<ActorId>,
}

impl Simulation {
    /// Whether a spawn of `ty` is a mover for the barrels policy's spawn rule S′
    /// (M3h9b note decision F3): every type except the policy's pickups and static
    /// decorations, whose boxes are never read against walls (they have no
    /// momentum, and the API refuses pushes). Read only under the barrels policy.
    pub(crate) fn spawn_mover(&self, ty: ActorTypeId) -> bool {
        let policy = self.player_policy();
        !((policy.has_world_pickups() && world_pickups::kind_for(policy, ty).is_some())
            || e1m1_rules::is_decoration(policy, ty))
    }
    /// Profile 4, all recognized source metadata, medium single-player skill,
    /// 1024 slots. Map THINGS are not spawned implicitly.
    pub fn actor_states(map: Map) -> Result<Self, SimError> {
        Self::with_actor_registry(map, 1024, ActorRegistry::vanilla(), ActorOptions::default())
    }
    /// Explicit immutable definitions and options. Native probe definitions do
    /// not implement a scripting language or mod compatibility layer.
    pub fn with_actor_registry(
        map: Map,
        max_actors: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if map.things().len() > 4096 {
            return Err(ActorError::WorkLimit.into());
        }
        if max_actors > 1024 {
            return Err(StoreError::InvalidLimit.into());
        }
        if options.mode != ActorGameMode::SinglePlayer {
            return Err(ActorError::UnsupportedMode.into());
        }
        let mut sim = Self::create(map, max_actors, Profile::ActorStates)?;
        snapshot::registry_fields(Arc::make_mut(&mut sim.map_fields), &registry);
        sim.registry = Some(Arc::new(registry));
        sim.actor_options = options;
        Ok(sim)
    }
    /// Read-only registry, absent in earlier physics profiles.
    pub fn actor_registry(&self) -> Option<&ActorRegistry> {
        self.registry.as_deref()
    }
    /// Immutable placement options.
    pub const fn actor_options(&self) -> ActorOptions {
        self.actor_options
    }
    fn registry_required(&self) -> Result<&ActorRegistry, SimError> {
        self.registry
            .as_deref()
            .ok_or(ActorError::UnsupportedMode.into())
    }
    pub(crate) fn active_typed(&self, id: ActorId) -> Result<ActorLife, SimError> {
        self.registry_required()?;
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        self.actor(id)
            .and_then(Actor::life)
            .ok_or(ActorError::InvalidType.into())
    }
    pub(crate) fn reserve_actor_spawns(&mut self, count: usize) -> Result<(), SimError> {
        self.actors.reserve_spawns(count)?;
        self.spatial
            .order
            .try_reserve(count)
            .map_err(|_| StoreError::Capacity)?;
        Ok(())
    }
    /// Supported P_SpawnMobj: assign the spawn state directly (no action), set
    /// type properties/height, draw lastlook once, link and append the thinker.
    pub fn spawn_type(
        &mut self,
        ty: ActorTypeId,
        position: [Fixed; 2],
        height: SpawnHeight,
    ) -> Result<ActorId, SimError> {
        self.spawn_type_checks(ty, position, height)?;
        self.reserve_actor_spawns(1)?;
        Ok(LifeHost {
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
        }
        .spawn(ty, position, height))
    }
    /// Every check [`Simulation::spawn_type`] makes before reserving the slot, also
    /// run by the level policy for the body it spawns in the THINGS batch.
    fn spawn_type_checks(
        &self,
        ty: ActorTypeId,
        position: [Fixed; 2],
        height: SpawnHeight,
    ) -> Result<(), SimError> {
        if self.player_policy().has_world_pickups()
            && world_pickups::kind_for(self.player_policy(), ty).is_some()
        {
            return Err(MapSpawnError::DirectSpawn.into());
        }
        // A static decoration exists only as a THING spawned by `P_SpawnMapThing`
        // (p_mobj.c:775–796: spawnpoint, angle, `MF_AMBUSH`); no action or body of
        // the source spawns one, and the decoration witness requires the THING
        // record. Under the E1M1 policy a direct spawn is refused like a pickup's.
        if e1m1_rules::is_decoration(self.player_policy(), ty) {
            return Err(MapSpawnError::DirectSpawn.into());
        }
        // The barrel likewise exists only as a THING (M3h9b decision F4(v)): no
        // admitted body spawns one, and its witness requires the 2035 spawnpoint.
        if barrels::is_barrel(self.player_policy(), ty) {
            return Err(MapSpawnError::DirectSpawn.into());
        }
        self.player_spawn_guard(ty, position, height)?;
        if self.profile.uses_demon_map() && ty == self.demon_owner_type() {
            return Err(MapSpawnError::DirectSpawn.into());
        }
        if self.profile.uses_perception()
            && (!self.spawn_admitted(
                position,
                self.registry
                    .as_deref()
                    .and_then(|r| r.actor_type(ty))
                    .map_or(Fixed::ZERO, ActorTypeDef::radius),
                1024,
                self.spawn_mover(ty),
            ) || matches!(height,SpawnHeight::Absolute(z) if !crate::sight::within(z,1024)))
        {
            return Err(PerceptionError::Arithmetic.into());
        }
        // After the position bound: the fit lookup descends the BSP.
        self.spawn_fits(ty, position, height)?;
        if self.player_policy().has_imps() && crate::original_registry::is_missile_type(ty) {
            return Err(ProjectileError::Controlled.into());
        }
        if self.profile.uses_perception() {
            self.validate_perception_type(ty)?;
        } else if self.profile == Profile::Projectiles {
            if ty == ActorTypeId(34) {
                return Err(ProjectileError::Controlled.into());
            }
            self.projectile_setup_position(position, height)?;
            self.validate_combat_type(ty)?;
        } else if matches!(self.profile, Profile::Combat | Profile::PlayerWeapons) {
            self.validate_combat_type(ty)?;
        } else {
            self.registry_required()?.validate_type(ty, &self.sectors)?;
        }
        if self.profile.uses_monsters() {
            self.preflight_monster_spawn(ty, position, height)?;
        }
        Ok(())
    }
    pub(crate) fn validate_entry(&self, state: StateId) -> Result<(), SimError> {
        let registry = self.registry_required()?;
        // The Imp's melee root is direct-entry, unlike any former-human attack.
        if self.player_policy().has_imps() {
            // The barrel's states, under the barrels policy only: its closure admits
            // `A_Explode` in `S_BEXP4` and nothing a monster runs (M3h9b decision F4).
            if self.player_policy().has_barrels()
                && identity::OriginalState::try_from(state.0).is_ok_and(barrels::state_allowed)
            {
                registry.validate_state_barrel(state)?;
            } else if self.player_policy().has_episode1()
                && identity::OriginalState::try_from(state.0).is_ok_and(|s| {
                    matches!(
                        s,
                        identity::OriginalState::S_ROCKET
                            | identity::OriginalState::S_EXPLODE1
                            | identity::OriginalState::S_EXPLODE2
                            | identity::OriginalState::S_EXPLODE3
                    )
                })
            {
                // The rocket's states (M4a plan §E.4): `A_Explode` in `S_EXPLODE1`.
                registry.validate_state_rocket(state)?;
            } else {
                registry.validate_state_original_monsters(state)?;
            }
            return Ok(());
        }
        if self.profile.uses_demon() {
            registry.validate_state_demon_melee(state)?;
        } else if self.profile == Profile::Chaingunner {
            registry.validate_state_chaingunner(state)?;
        } else if self.profile == Profile::Shotgunner {
            registry.validate_state_shotgunner(state)?;
        } else if self.profile.uses_monsters() {
            registry.validate_state_monsters(state)?;
        } else if self.profile == Profile::Pursuit {
            registry.validate_state_pursuit(state)?;
        } else if self.profile.uses_perception() {
            registry.validate_state_perception(state)?;
        } else {
            registry.validate_state(state)?;
        }
        for ty in registry.spawn_dependencies(state)? {
            if self.profile.uses_perception() {
                self.validate_perception_type(ty)?;
            } else if self.profile == Profile::Projectiles {
                registry.validate_type_projectile(ty, &self.sectors)?;
            } else if matches!(self.profile, Profile::Combat | Profile::PlayerWeapons) {
                registry.validate_type_combat(ty, &self.sectors)?;
            } else {
                registry.validate_type(ty, &self.sectors)?;
            }
        }
        Ok(())
    }
    pub(crate) fn enter_validated(&mut self, id: ActorId, state: StateId) {
        self.enter_monster_planned(id, state, None);
    }
    pub(crate) fn enter_monster_planned(
        &mut self,
        id: ActorId,
        state: StateId,
        shot: Option<&crate::monsters::MonsterShotPlan>,
    ) {
        self.enter_attack_planned(id, state, shot, None);
    }
    pub(crate) fn enter_attack_planned(
        &mut self,
        id: ActorId,
        state: StateId,
        shot: Option<&crate::monsters::MonsterShotPlan>,
        shotgun: Option<&crate::shotgunner::ShotgunPlan>,
    ) {
        let perception =
            (self.profile.uses_perception()).then_some(crate::perception::PerceptionInputs {
                pursuit: (self.profile == Profile::Pursuit || self.profile.uses_monsters()),
                monsters: self.profile.uses_monsters(),
                shotgunner: self.profile == Profile::Shotgunner,
                chaingunner: self.profile == Profile::Chaingunner,
                demon: self.profile.uses_demon(),
                map_demons: self.profile.uses_demon_map(),
                checked: None,
                melee: None,
                shotgun,
                shot,
                players: crate::player_roster::PerceptionRoster::Historical(
                    &self.perception_players,
                ),
                sound_targets: &self.sound_targets,
            });
        LifeHost {
            specials: None,
            buttons: None,
            player: None,
            statistics: self.statistics.as_mut(),
            perception,
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
        .enter(id, state);
    }
    /// Enter a validated state immediately between tics. Work, action availability
    /// and child capacity are checked before any callback or gameplay RNG draw.
    pub fn enter_actor_state(&mut self, id: ActorId, state: StateId) -> Result<(), SimError> {
        if self.owns_player_body(id) {
            return Err(PlayerBodyError::Unsupported.into());
        }
        let life = self.active_typed(id)?;
        if (self.player_policy().has_world_pickups()
            && world_pickups::kind_for(self.player_policy(), life.actor_type).is_some())
            || e1m1_rules::is_decoration(self.player_policy(), life.actor_type)
            || barrels::is_barrel(self.player_policy(), life.actor_type)
        {
            // Nothing in the source re-enters an item's or a static decoration's
            // state (tics −1, never killed): an explicit refusal, not a no-op.
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.projectile_control(id)?;
        if self.profile == Profile::Shotgunner {
            return self.enter_shotgun_state(id, state);
        }
        if self.profile.uses_monsters() {
            return self.enter_monster_state(id, state);
        }
        if self.profile == Profile::Pursuit {
            self.preflight_pursuit_entry(id, state)?;
        } else if self.profile.uses_perception() {
            self.preflight_perception_entry(id, state)?;
        }
        self.validate_entry(state)?;
        let demand = self.registry_required()?.demand(state, life.tics)?;
        self.reserve_actor_spawns(demand.spawns)?;
        self.enter_validated(id, state);
        Ok(())
    }
    /// Controlled death-state entry for an unbound actor. This does not apply
    /// damage, alter health/height, set corpse flags, award kills or drop items.
    pub fn request_actor_death(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.profile.uses_demon_lifecycle() {
            return Err(MonsterError::Profile.into());
        }
        let life = self.active_typed(id)?;
        // An item is never killed, and neither is a decoration under the E1M1
        // policy (`P_KillMobj` is reached only through `P_DamageMobj`, which returns
        // for a thing without `MF_SHOOTABLE`, p_inter.c:787–788; M3h9 note §2.5):
        // neither has a death state to enter. Under the E1M1 policy the lifecycle
        // profile check above refuses first (`MonsterError::Profile`); this check
        // keeps the refusal explicit for any profile that reaches it.
        if (self.player_policy().has_world_pickups()
            && world_pickups::kind_for(self.player_policy(), life.actor_type).is_some())
            || e1m1_rules::is_decoration(self.player_policy(), life.actor_type)
            || barrels::is_barrel(self.player_policy(), life.actor_type)
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.projectile_control(id)?;
        if self.bindings.contains(&Some(id)) {
            return Err(ActorError::BoundDeath.into());
        }
        let state = self
            .registry_required()?
            .actor_type(life.actor_type)
            .unwrap()
            .states()[5];
        if state.0 == 0 {
            return Err(ActorError::NoDeathState.into());
        }
        self.validate_entry(state)?;
        let demand = self.registry_required()?.demand(state, life.tics)?;
        self.reserve_actor_spawns(demand.spawns)?;
        self.actors
            .get_mut(id)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .death_requested = true;
        self.pending
            .push(Event::ActorState(ActorStateEvent::Death(id)));
        self.enter_validated(id, state);
        Ok(())
    }
    /// Assign a validated store-local target. Role/lifetime rules depend on the
    /// profile: canonical Demons require a current, nonpending context body.
    /// References are retained on retirement; clearing never resolves the old target.
    pub fn set_actor_target(
        &mut self,
        id: ActorId,
        target: Option<ActorId>,
    ) -> Result<(), SimError> {
        if self.owns_player_body(id) {
            return Err(PlayerBodyError::Unsupported.into());
        }
        let life = self.active_typed(id)?;
        // An item never targets, and neither does a decoration under the E1M1
        // policy (no state of the seven types has an action, M3h9 note §2.5): the
        // tic preflight and the DSIM36 reader admit neither with a target.
        if (self.player_policy().has_world_pickups()
            && world_pickups::kind_for(self.player_policy(), life.actor_type).is_some())
            || e1m1_rules::is_decoration(self.player_policy(), life.actor_type)
            || barrels::is_barrel(self.player_policy(), life.actor_type)
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.projectile_control(id)?;
        if self.profile == Profile::Pursuit || self.profile.uses_monsters() {
            if let Some(t) = target {
                self.validate_perception_reference(t)?;
            }
        }
        if target.is_some_and(|target| self.actor(target).is_none()) {
            return Err(StoreError::InvalidHandle.into());
        }
        self.actors
            .get_mut(id)
            .and_then(|actor| actor.life.as_mut())
            .ok_or(StoreError::InvalidHandle)?
            .target = target;
        self.pending
            .push(Event::ActorState(ActorStateEvent::Target {
                actor: id,
                target,
            }));
        Ok(())
    }
    /// Resolve by the stored generation. Pending storage is readable; retired or
    /// reused storage resolves to None, without silently clearing the identity.
    pub fn actor_target(&self, id: ActorId) -> Option<&Actor> {
        self.actor(self.actor(id)?.life?.target?)
    }
    pub(crate) fn preflight_actor_step(&mut self) -> Result<(), SimError> {
        let Some(registry) = &self.registry else {
            return Ok(());
        };
        let mut demand = Demand::default();
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            let life = self.actors.get(id).unwrap().life.unwrap();
            if life.tics == 1 {
                demand
                    .add(registry.demand(registry.state(life.state.unwrap()).unwrap().next, 0)?)?;
            }
        }
        self.reserve_actor_spawns(demand.spawns)
    }
    /// Atomic ordinary placement in source order. The general lifecycle profile
    /// reports starts without creating players and allows explicit repetition.
    /// Profile 16 instead requires a fresh batch and rejects starts/unsupported
    /// enabled types; see `with_demon_map_spawn_registry` for its admission.
    pub fn spawn_map_things(&mut self) -> Result<MapSpawnReport, SimError> {
        let registry = self.registry_required()?;
        // The specials policy's light thinkers follow every map THING (M3h7
        // decision 12, P_SetupLevel order).
        if self.level_specials_spawned() {
            return Err(MapSpawnError::LevelSpecials.into());
        }
        let map_demons = self.profile.uses_demon_map();
        if map_demons && (self.tic != 0 || !self.actors.slots.is_empty()) {
            return Err(MapSpawnError::Setup.into());
        }
        let mut report = MapSpawnReport::default();
        let mut selected = Vec::new();
        // The real-map policy keeps starts as `P_SpawnMapThing` does (types 1–4 in
        // `playerstarts`, up to ten type 11 in `deathmatchstarts`; the player body
        // is spawned by the caller); the earlier map-demon policies reject them.
        let starts_as_records = self.player_policy().has_real_map();
        let level = self.player_policy().has_level();
        let player_type = ActorTypeId(crate::identity::OriginalActorType::MT_PLAYER.ordinal());
        for thing in self.world.map().things() {
            if map_demons && !starts_as_records && (thing.kind <= 4 || thing.kind == 11) {
                return Err(MapSpawnError::StartOrInvalidType(thing.kind).into());
            }
            if thing.kind == 11 {
                if report.deathmatch_starts.len() < 10 {
                    report.deathmatch_starts.push(*thing);
                }
                continue;
            }
            if (1..=4).contains(&thing.kind) {
                report.player_starts.push(*thing);
                // The level policy (decision G4): `P_SpawnMapThing` stores the start
                // and, single player, calls `P_SpawnPlayer` at once (p_mobj.c:
                // 729–736), which returns for a start whose player is not in the
                // game (655–656: types 2–4). The body joins the batch here, at its
                // lump position, with the checks `spawn_type` makes; a second
                // type-1 record (a voodoo doll) is refused.
                if level && thing.kind == 1 {
                    if selected.iter().any(|(_, ty)| *ty == player_type) {
                        return Err(PlayerBodyError::Binding.into());
                    }
                    self.spawn_type_checks(
                        player_type,
                        [
                            Fixed::from_map_units(thing.position.x),
                            Fixed::from_map_units(thing.position.y),
                        ],
                        SpawnHeight::Floor,
                    )?;
                    selected.push((*thing, player_type));
                }
                continue;
            }
            let bit = match self.actor_options.skill {
                Skill::Baby | Skill::Easy => 1,
                Skill::Medium => 2,
                Skill::Hard | Skill::Nightmare => 4,
            };
            if thing.options & 16 != 0 || thing.options & bit == 0 {
                report.filtered += 1;
                continue;
            }
            let mut ty = registry
                .find_doomed(i32::from(thing.kind))
                .ok_or(ActorError::InvalidType)?;
            let info = registry.actor_type(ty).unwrap();
            if self.actor_options.no_monsters
                && (info.flags() & 4194304 != 0 || lifecycle::source_type_is(ty, "MT_SKULL"))
            {
                report.filtered += 1;
                continue;
            }
            if map_demons {
                // Lookup and nomonsters use original metadata, including COUNTKILL,
                // before the explicit executable fixture adaptation.
                let pickup = self.player_policy().has_world_pickups()
                    && world_pickups::kind_for(self.player_policy(), ty).is_some();
                let former = crate::original_registry::is_extended_monster(ty)
                    && crate::original_registry::is_policy_monster(self.player_policy(), ty);
                // One of the seven static decoration types under the E1M1 policy
                // (M3h9 note §2.5): spawned like any THING by `P_SpawnMapThing`
                // (p_mobj.c:708–797), counted against `MAX_DECORATIONS` alone.
                let decoration = e1m1_rules::is_decoration(self.player_policy(), ty);
                // The exploding barrel under the barrels policy (M3h9b note §2.1,
                // decision F4(i)): its own role, never a roster monster, never
                // rewritten to the owner type, counted against `MAX_BARRELS` alone.
                let barrel = barrels::is_barrel(self.player_policy(), ty);
                // The teleport landing (doomednum 14) under wire 24 (M4b step 4):
                // `P_SpawnMapThing` spawns it like any THING; it is in no roster
                // and never rewritten to the owner type.
                let landing = crate::teleport::is_landing(self.player_policy(), ty);
                if !pickup
                    && !former
                    && !decoration
                    && !barrel
                    && !landing
                    && ty
                        != crate::identity::OriginalActorType::MT_SERGEANT
                            .resolve(registry)
                            .map_err(|_| MonsterError::Definition)?
                {
                    return Err(MapSpawnError::UnsupportedType(thing.kind).into());
                }
                // Every older policy refuses octants outside −4..=3; the barrels policy
                // admits every angle (decision F8): `spawn_placed_thing` converts it as
                // p_mobj.c:794 does, `ANG45 × (angle / 45)` modulo 2³², which depends
                // on `(angle / 45) mod 8` only.
                if self.profile.uses_checked_demons()
                    && !self.player_policy().has_barrels()
                    && !(-4..=3).contains(&(thing.angle / 45))
                {
                    return Err(MapSpawnError::Angle(thing.angle).into());
                }
                if decoration
                    && selected
                        .iter()
                        .filter(|(_, ty)| e1m1_rules::is_decoration(self.player_policy(), *ty))
                        .count()
                        == self.decoration_limit()
                {
                    return Err(MapSpawnError::DecorationLimit.into());
                }
                if barrel
                    && selected
                        .iter()
                        .filter(|(_, ty)| barrels::is_barrel(self.player_policy(), *ty))
                        .count()
                        == self.barrel_limit()
                {
                    return Err(MapSpawnError::BarrelLimit.into());
                }
                if (!pickup
                    && !decoration
                    && !barrel
                    && !landing
                    && selected
                        .iter()
                        .filter(|(_, ty)| self.is_roster_monster(*ty))
                        .count()
                        == self.map_demon_limit())
                    || (pickup
                        && selected
                            .iter()
                            .filter(|(_, ty)| {
                                world_pickups::kind_for(self.player_policy(), *ty).is_some()
                            })
                            .count()
                            == world_pickups::pickup_bound(
                                self.player_policy(),
                                self.world.episode1,
                            ))
                {
                    return Err(MapSpawnError::WorkLimit.into());
                }
                if !self.spawn_admitted(
                    [
                        Fixed::from_map_units(thing.position.x),
                        Fixed::from_map_units(thing.position.y),
                    ],
                    info.radius(),
                    978,
                    !pickup && !decoration && !landing,
                ) {
                    return Err(MonsterError::Arithmetic.into());
                }
                self.spawn_fits(
                    ty,
                    [
                        Fixed::from_map_units(thing.position.x),
                        Fixed::from_map_units(thing.position.y),
                    ],
                    SpawnHeight::Floor,
                )?;
                if !pickup && !former && !decoration && !barrel && !landing {
                    ty = self.demon_owner_type();
                }
            }
            if self.profile.uses_perception() {
                if !self.spawn_admitted(
                    [
                        Fixed::from_map_units(thing.position.x),
                        Fixed::from_map_units(thing.position.y),
                    ],
                    info.radius(),
                    1024,
                    self.spawn_mover(ty),
                ) {
                    return Err(PerceptionError::Arithmetic.into());
                }
                self.validate_perception_type(ty)?;
            } else if self.profile == Profile::Projectiles {
                if ty == ActorTypeId(34) {
                    return Err(ProjectileError::Controlled.into());
                }
                self.projectile_setup_position(
                    [
                        Fixed::from_map_units(thing.position.x),
                        Fixed::from_map_units(thing.position.y),
                    ],
                    SpawnHeight::Floor,
                )?;
                self.validate_combat_type(ty)?;
            } else if matches!(self.profile, Profile::Combat | Profile::PlayerWeapons) {
                registry.validate_type_combat(ty, &self.sectors)?;
            } else {
                registry.validate_type(ty, &self.sectors)?;
            }
            if matches!(self.profile, Profile::Combat | Profile::PlayerWeapons) {
                self.validate_combat_type(ty)?;
            }
            selected.push((*thing, ty));
        }
        // A level without its type-1 start has no body to bind (`players[0].mo`
        // stays NULL in the source): refused.
        if level && !selected.iter().any(|(_, ty)| *ty == player_type) {
            return Err(PlayerBodyError::Binding.into());
        }
        if let Some(stats) = self.statistics {
            let count = i32::try_from(
                selected
                    .iter()
                    .filter(|(_, ty)| {
                        registry
                            .actor_type(*ty)
                            .is_some_and(|t| t.flags() & world_pickups::MF_COUNTKILL != 0)
                    })
                    .count(),
            )
            .map_err(|_| MapSpawnError::StatisticsOverflow)?;
            stats
                .total_kills
                .checked_add(count)
                .ok_or(MapSpawnError::StatisticsOverflow)?;
        }
        if self.player_policy().has_player_bonuses() {
            let total = self
                .statistics
                .ok_or(PlayerBodyError::Invariant)?
                .total_items;
            let count = selected
                .iter()
                .filter(|(_, ty)| {
                    registry
                        .actor_type(*ty)
                        .is_some_and(|t| t.flags() & world_pickups::MF_COUNTITEM != 0)
                })
                .count();
            let count = i32::try_from(count).map_err(|_| MapSpawnError::StatisticsOverflow)?;
            let bound =
                world_pickups::pickup_bound(self.player_policy(), self.world.episode1) as i32;
            if !(0..=bound).contains(&total) || total.checked_add(count).is_none_or(|n| n > bound) {
                return Err(MapSpawnError::StatisticsOverflow.into());
            }
            let mut body = None;
            report.spawned = self.checked_map_spawn_batch(selected.len(), |host| {
                let mut ids = line_work::reserved(selected.len())?;
                for (thing, ty) in selected {
                    if level && ty == player_type {
                        body = Some(host.spawn_level_body(thing));
                        continue;
                    }
                    ids.push(host.spawn_placed_thing(thing, ty, true));
                    #[cfg(test)]
                    BONUS_MAP_FAIL_AFTER.with(|counter| {
                        if let Some(left) = counter.get() {
                            if left <= 1 {
                                counter.set(None);
                                let last = *ids.last().unwrap();
                                BONUS_MAP_FAILURE_OBSERVATION.with(|o| {
                                    o.set(Some((
                                        host.rng.indices()[0],
                                        host.statistics.as_ref().unwrap().total_items,
                                        host.access.order().count(),
                                        host.access.get(last).unwrap().life().unwrap().tics,
                                    )))
                                });
                                return Err(MapSpawnError::WorkLimit.into());
                            }
                            counter.set(Some(left - 1));
                        }
                        Ok::<(), SimError>(())
                    })?;
                }
                Ok(ids)
            })?;
            if let Some(id) = body {
                if let Some(p) = &mut self.original_player {
                    p.mo = Some(id);
                }
                report.player_body = Some(id);
            }
        } else {
            self.reserve_actor_spawns(selected.len())?;
            // Legacy batches still have no recoverable work after reservation.
            let mut host = LifeHost {
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
            };
            for (thing, ty) in selected {
                report
                    .spawned
                    .push(host.spawn_placed_thing(thing, ty, false));
            }
        }
        Ok(report)
    }
}

impl<A: lifecycle::ActorAccess> LifeHost<'_, A> {
    /// `P_SpawnPlayer` for player 1 under the level policy (p_mobj.c:642–686),
    /// inside the THINGS batch at the type-1 record's lump position:
    /// `P_SpawnMobj (x, y, ONFLOORZ, MT_PLAYER)` (one `P_Random` for `lastlook`,
    /// 506; `S_PLAY` has `tics` −1 and `P_SpawnMapThing` returned before its tics
    /// draw, 736), then `mobj->angle = ANG45 * (mthing->angle/45)` (671), converted
    /// as `spawn_placed_thing` converts a THING's (decision F8). No spawnpoint is
    /// set (the source sets none). `mobj->health = p->health` (673) is the
    /// `G_PlayerReborn` 100 the policy starts with (g_game.c:824), as are the
    /// other player fields `P_SpawnPlayer` resets (676–684: `refire`,
    /// `damagecount`, `bonuscount`, `extralight` 0, `usedown`/`attackdown` true);
    /// `P_SetupPsprites` (686) is the caller's `initialize_player_weapons`, which
    /// draws nothing. The `Bound` observation follows the spawn's.
    fn spawn_level_body(&mut self, thing: doom_map::Thing) -> ActorId {
        let id = self.spawn(
            ActorTypeId(crate::identity::OriginalActorType::MT_PLAYER.ordinal()),
            [
                Fixed::from_map_units(thing.position.x),
                Fixed::from_map_units(thing.position.y),
            ],
            SpawnHeight::Floor,
        );
        self.access.get_mut(id).unwrap().spec.angle = e1m1_rules::spawn_angle(thing.angle);
        self.events.push(Event::PlayerBody(
            crate::original_player::PlayerBodyEvent::Bound(id),
        ));
        id
    }
    // All definitions and complete counter additions are proved before calling.
    // Both legacy and staged batches use this one original placement service.
    fn spawn_placed_thing(
        &mut self,
        thing: doom_map::Thing,
        ty: ActorTypeId,
        count_items: bool,
    ) -> ActorId {
        let info = self.registry.actor_type(ty).unwrap();
        let height = if info.flags() & 256 != 0 {
            SpawnHeight::Ceiling
        } else {
            SpawnHeight::Floor
        };
        let id = self.spawn(
            ty,
            [
                Fixed::from_map_units(thing.position.x),
                Fixed::from_map_units(thing.position.y),
            ],
            height,
        );
        let actor = self.access.get_mut(id).unwrap();
        let life = actor.life.as_mut().unwrap();
        life.spawnpoint = Some(thing);
        if life.tics > 0 {
            life.tics = 1 + i32::from(self.rng.p_random()) % life.tics;
        }
        if let Some(stats) = self.statistics.as_deref_mut() {
            if life.flags & world_pickups::MF_COUNTKILL != 0 {
                stats.total_kills += 1;
                self.events
                    .push(Event::Statistics(StatisticsEvent::MapKillTotal {
                        actor: id,
                        total: stats.total_kills,
                    }));
            }
            if count_items && life.flags & world_pickups::MF_COUNTITEM != 0 {
                stats.total_items += 1;
                self.events
                    .push(Event::Statistics(StatisticsEvent::MapItemTotal {
                        actor: id,
                        total: stats.total_items,
                    }));
            }
        }
        actor.spec.angle = ((i32::from(thing.angle) / 45) as u32).wrapping_mul(0x20000000);
        if thing.options & 8 != 0 {
            life.flags |= 32;
        }
        id
    }
}
#[cfg(test)]
std::thread_local! {
    pub(crate) static BONUS_MAP_FAILURE_OBSERVATION: std::cell::Cell<Option<(u8,i32,usize,i32)>> = const { std::cell::Cell::new(None) };
    pub(crate) static BONUS_MAP_FAIL_AFTER: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}
