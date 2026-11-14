//! Explicit unbound-actor combat services on the accepted state/lifecycle host.
use crate::{
    lifecycle::{ActorAccess, Demand, LifeHost},
    *,
};
use crate::{
    original_player::{
        BASETHRESHOLD, MF_CORPSE, MF_DROPOFF, MF_FLOAT, MF_JUSTHIT, MF_NOBLOOD, MF_NOGRAVITY,
        MF_SHOOTABLE, MF_SKULLFLY, THRUST_FRACTION, THRUST_MASS_SCALE,
    },
    world_pickups::MF_COUNTKILL,
};
use std::{error::Error, fmt};
/// A bounded combat prerequisite failed before authoritative mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatError {
    /// Requires the explicit profile 5.
    Profile,
    /// Combat identity is invalid, stale or pending retirement.
    Reference,
    /// Full player inventory/armor/weapon/scoring behavior is unavailable.
    Player,
    /// Damage must be in 0..=1000 in this declared slice.
    Damage,
    /// Source mass/spawn health or checked health arithmetic is invalid.
    Metadata,
    /// Selected source type requires item drops or another excluded kill branch.
    KillBranch,
    /// Native definitions changed the required original impact behavior.
    ImpactDefinition,
    /// Original sky suppression is not implemented by this profile.
    Sky,
    /// Range must be positive and at most 2048 fixed map units.
    Range,
    /// Trace or thrust coordinate setup exceeds the defined arithmetic slice.
    Coordinates,
    /// Source's 128-intercept storage bound would be exceeded.
    InterceptLimit,
    /// Source's 64-cell traversal did not reach its endpoint.
    TraversalLimit,
    /// Conservative callback/trace work exceeded the supported bound.
    WorkLimit,
    /// Under the Episode 1 policy, a trace intercept or height sum that the
    /// source computes in signed `int` would overflow (decision H3: the checked
    /// twin of certificate clauses (i) and (vii)); reported for the refused tic as
    /// `MapScaleError::Arithmetic`.
    Arithmetic,
}
impl fmt::Display for CombatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "combat: {self:?}")
    }
}
impl Error for CombatError {}
/// Direct P_DamageMobj subset request; source and inflictor are independent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamageRequest {
    /// Actor receiving damage.
    pub target: ActorId,
    /// Physical damage origin, or None for environment damage.
    pub inflictor: Option<ActorId>,
    /// Actor to target in response, or None.
    pub source: Option<ActorId>,
    /// Supplied source damage, not a weapon damage/spread generator.
    pub amount: i32,
}
/// Outcome of one completed direct damage operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamageResult {
    /// Nonshootable/already-dead source early return.
    pub ignored: bool,
    /// Health before/after.
    pub health: [i32; 2],
    /// Pain chance succeeded; its state may itself remove the actor.
    pub pain: bool,
    /// P_KillMobj subset ran.
    pub killed: bool,
    /// Extreme-death state selected by the strict source threshold.
    pub extreme: bool,
}
/// Explicit source P_LineAttack parameters; weapon buttons/ammo are later work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineAttack {
    /// Unsigned binary angle.
    pub angle: u32,
    /// Positive fixed attack range, including its fractional bits.
    pub range: Fixed,
    /// Fixed vertical slope; use aim_line_attack for source autoaim.
    pub slope: Fixed,
    /// Damage zero still spawns an impact and consumes its RNG.
    pub damage: i32,
}
/// Trace and real gameplay effects of a successful explicit attack request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttackResult {
    /// Original candidate/fraction collection.
    pub trace: HitscanTrace,
    /// First applicable line or actor.
    pub hit: Option<TraceObject>,
    /// Impact position before source random Z jitter.
    pub position: Option<[Fixed; 3]>,
    /// Number of intercept callbacks evaluated before stopping.
    pub traversed: u32,
    /// Real puff/blood thinker, not a renderer-only substitute.
    pub effect: Option<ActorId>,
    /// Damage result, absent for walls, misses or zero attack damage.
    pub damage: Option<DamageResult>,
}
/// Deterministic combat observations in source side-effect order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatEvent {
    /// Explicit shot parameters before trace observations/effects.
    Attack {
        /// Valid shooter.
        shooter: ActorId,
        /// Source request.
        attack: LineAttack,
    },
    /// Complete read-only decision, recorded before impacts/damage.
    Trace {
        /// Source collection and stopping result.
        result: Box<AttackResult>,
    },
    /// Direct damage invocation before thrust/health/RNG side effects.
    Damage(DamageRequest),
    /// Source implicit draw; kind 0=fall-forward, 1=pain, 2=kill timer,
    /// 3/4=impact Z draws, 5=impact timer (spawn lastlook stays implicit).
    Random {
        /// Defined service draw category.
        kind: u8,
        /// Original P_Random lookup.
        value: u8,
    },
    /// Source XY impulse applied before health reduction.
    Knockback {
        /// Receiving actor.
        target: ActorId,
        /// Binary angle after any reversal.
        angle: u32,
        /// Fixed impulse magnitude.
        thrust: Fixed,
        /// Resulting XY momentum.
        momentum: [Fixed; 2],
    },
    /// Health subtraction before pain/kill decisions.
    Health {
        /// Receiving actor.
        target: ActorId,
        /// Before and after health.
        values: [i32; 2],
    },
    /// JUSTHIT set before entering pain state.
    Pain {
        /// Receiving actor.
        target: ActorId,
        /// Registered pain state.
        state: StateId,
    },
    /// Source kill flag/height changes, before death actions/timer draw.
    Kill {
        /// Killed actor.
        target: ActorId,
        /// Selected death state.
        state: StateId,
        /// Extreme-death threshold succeeded.
        extreme: bool,
    },
    /// Source post-action randomized/clamped death tics.
    DeathTimer {
        /// Killed actor.
        target: ActorId,
        /// Remaining state duration.
        tics: i32,
    },
    /// Nonlethal damage reset reactiontime after pain callbacks.
    Reaction(ActorId),
    /// Source target/threshold assignment, before any see-state entry.
    Retarget {
        /// Damaged owner.
        target: ActorId,
        /// Valid source identity (may be pending after a callback).
        source: ActorId,
    },
    /// Real impact initialization completed before damage.
    Impact {
        /// Spawned effect.
        actor: ActorId,
    },
    /// Source ignored target; no gameplay RNG/health/momentum changed.
    Ignored(ActorId),
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct DamagePlan {
    request: DamageRequest,
    armor: Option<ArmorEvent>,
    health: [i32; 2],
    killed: bool,
    extreme: bool,
    state: StateId,
    demand: Demand,
    ignored: bool,
    credit: bool,
    /// The damage after `P_DamageMobj`'s player-specific cap (the Episode 1
    /// policy's sector special 11, p_inter.c:837–842) and before armor;
    /// `request.amount` otherwise. The thrust keeps `request.amount` (806–832
    /// precede the cap).
    amount: i32,
    /// `P_DamageMobj`'s thrust block runs (p_inter.c:806–832): false only under
    /// the Episode 1 policy when the source is the player and its ready weapon is
    /// the chainsaw (811), whatever the inflictor (a rocket still in flight
    /// included).
    thrust: bool,
    /// A telefrag of wire 24 (`PIT_StompThing`'s `P_DamageMobj(thing, tmthing,
    /// tmthing, 10000)`, p_map.c:105; decision I3): the thrust is the wrapped
    /// 32-bit value of `damage*(FRACUNIT>>3)*100/mass` (p_inter.c:817). False for
    /// every other damage, whose thrust never overflows (amount ≤ 1,000).
    telefrag: bool,
}
#[derive(Clone, Debug)]
pub(crate) struct PreparedAttack {
    shooter: ActorId,
    attack: LineAttack,
    pub result: AttackResult,
    plan: Option<DamagePlan>,
    /// The specials policy's sky-ended trace, emitted right after its trace record.
    sky: Option<crate::SpecialEvent>,
}
impl DamagePlan {
    fn start(
        actor: Actor,
        request: DamageRequest,
        armor: Option<PlayerArmorState>,
        policy: OriginalExecutionProfile,
    ) -> Result<Self, SimError> {
        Self::start_capped(actor, request, request.amount, armor, policy)
    }
    fn start_capped(
        actor: Actor,
        request: DamageRequest,
        amount: i32,
        armor: Option<PlayerArmorState>,
        policy: OriginalExecutionProfile,
    ) -> Result<Self, SimError> {
        let life = actor.life().ok_or(CombatError::Metadata)?;
        let ignored = life.flags & MF_SHOOTABLE == 0 || life.health <= 0;
        let armor = if ignored {
            None
        } else {
            armor
                .map(|a| ArmorEvent::plan_for(policy, request.target, amount, a))
                .transpose()?
        };
        let after = if ignored {
            life.health
        } else {
            life.health
                .checked_sub(armor.map_or(amount, |a| a.effective))
                .ok_or(CombatError::Metadata)?
        };
        Ok(Self {
            request,
            armor,
            health: [life.health, after],
            killed: !ignored && after <= 0,
            extreme: false,
            state: StateId(0),
            demand: Demand::default(),
            ignored,
            credit: false,
            amount,
            thrust: true,
            telefrag: false,
        })
    }
    pub(crate) fn spawn_count(self) -> usize {
        self.demand.spawns
    }
    fn finish(
        self,
        actor: Actor,
        registry: &ActorRegistry,
        validate_entry: impl Fn(StateId) -> Result<(), SimError>,
    ) -> Result<Self, SimError> {
        self.finish_with(actor, registry, validate_entry, false)
    }
    /// `drops` admits the P_KillMobj item branch (PlayerMonstersV1 only); its
    /// one MF_DROPPED item is a counted spawn demand like any other thinker.
    fn finish_with(
        mut self,
        actor: Actor,
        registry: &ActorRegistry,
        validate_entry: impl Fn(StateId) -> Result<(), SimError>,
        drops: bool,
    ) -> Result<Self, SimError> {
        let life = actor.life.unwrap();
        let info = registry.actor_type(life.actor_type).unwrap();
        if self.killed {
            let dropper = ["MT_POSSESSED", "MT_WOLFSS", "MT_SHOTGUY", "MT_CHAINGUY"]
                .iter()
                .any(|name| lifecycle::source_type_is(life.actor_type, name));
            if dropper && !(drops && crate::original_registry::is_former_human(life.actor_type)) {
                return Err(CombatError::KillBranch.into());
            }
            self.extreme = self.health[1] < -info.spawn_health() && info.states()[6].0 != 0;
            self.state = info.states()[if self.extreme { 6 } else { 5 }];
            validate_entry(self.state)?;
            self.demand = registry.demand(self.state, life.tics)?;
            if dropper {
                self.demand.spawns += 1;
            }
        } else {
            if info.pain_chance() > 0 {
                self.demand
                    .add(registry.demand(info.states()[2], life.tics)?)?;
            }
            if self.request.source.is_some() {
                self.demand
                    .add(registry.demand(info.states()[1], life.tics)?)?;
            }
        }
        Ok(self)
    }
    /// Current canonical bullet damage, sharing health/pain/death/retarget planning.
    pub(crate) fn current_player_attack(
        registry: &ActorRegistry,
        actor: Actor,
        request: DamageRequest,
        statistics: &LevelStatistics,
    ) -> Result<Self, SimError> {
        let life = actor.life().ok_or(CombatError::Metadata)?;
        if life.actor_type.0 != identity::OriginalActorType::MT_SERGEANT.ordinal() {
            return Err(MonsterError::Definition.into());
        }
        let mut plan = Self::current_melee_body(registry, actor, request)?;
        if plan.killed {
            // The exact admitted Demon definition has MF_COUNTKILL (p_mobj.h).
            if life.flags & MF_COUNTKILL != 0 {
                statistics
                    .single_player_kill_count
                    .checked_add(1)
                    .ok_or(MapSpawnError::StatisticsOverflow)?;
                plan.credit = true;
            }
        }
        Ok(plan)
    }
    /// Current damage for a globally preflighted body role. The canonical player
    /// caller additionally proves current ownership and physical impulse bounds.
    /// Earlier actions may change health/RNG; never reuse a prior-tic plan.
    pub(crate) fn current_melee_body(
        registry: &ActorRegistry,
        actor: Actor,
        request: DamageRequest,
    ) -> Result<Self, SimError> {
        Self::current_armored_body(
            registry,
            actor,
            request,
            None,
            OriginalExecutionProfile::DemonNormalV1,
        )
    }
    /// Same shared plan, with source armor owned by the current genuine player.
    pub(crate) fn current_armored_body(
        registry: &ActorRegistry,
        actor: Actor,
        request: DamageRequest,
        armor: Option<PlayerArmorState>,
        policy: OriginalExecutionProfile,
    ) -> Result<Self, SimError> {
        Self::current_armored_body_capped(registry, actor, request, request.amount, armor, policy)
    }
    /// [`Self::current_armored_body`] with the pre-armor damage `amount` after
    /// the player-specific cap.
    fn current_armored_body_capped(
        registry: &ActorRegistry,
        actor: Actor,
        request: DamageRequest,
        amount: i32,
        armor: Option<PlayerArmorState>,
        policy: OriginalExecutionProfile,
    ) -> Result<Self, SimError> {
        let plan = Self::start_capped(actor, request, amount, armor, policy)?;
        if plan.ignored {
            return Ok(plan);
        }
        plan.finish(actor, registry, |state| {
            registry.validate_state_demon_melee(state)?;
            Ok(())
        })
    }
}
/// P_KillMobj item for an admitted former human (p_inter.c drop switch).
pub(crate) fn source_drop(ty: ActorTypeId) -> Option<identity::OriginalActorType> {
    use identity::OriginalActorType as T;
    match ty.0 {
        n if n == T::MT_POSSESSED.ordinal() => Some(T::MT_CLIP),
        n if n == T::MT_SHOTGUY.ordinal() => Some(T::MT_SHOTGUN),
        n if n == T::MT_CHAINGUY.ordinal() => Some(T::MT_CHAINGUN),
        _ => None,
    }
}
impl DamagePlan {
    /// PlayerMonstersV1 current damage for any admitted target, planned from the
    /// staged transaction at actual attack time: the genuine player (with armor
    /// and the grounded impulse proof) or a roster monster (with the grounded
    /// impulse proof, kill credit and source drop demand). Never a cached plan.
    pub(crate) fn current_original<A: ActorAccess>(
        host: &LifeHost<'_, A>,
        request: DamageRequest,
    ) -> Result<Self, SimError> {
        Self::current_original_in(host, request, None)
    }
    /// [`Self::current_original`]; `special` is the runtime special of the
    /// victim's sector when the caller holds the staged specials outside the host
    /// (`P_PlayerInSpecialSector`), otherwise it is read from `host.specials`.
    /// Only the Episode 1 policy reads it, for the player body: the "end of game
    /// hell hack" caps a blow at `health − 1` when the body's sector special is
    /// 11 (p_inter.c:837–842), for every damage source, before invulnerability
    /// and armor. A host without the specials then refuses the damage.
    pub(crate) fn current_original_in<A: ActorAccess>(
        host: &LifeHost<'_, A>,
        request: DamageRequest,
        special: Option<i16>,
    ) -> Result<Self, SimError> {
        let player = host.player.as_deref().ok_or(PlayerBodyError::Profile)?;
        let policy = player.policy;
        if !policy.has_monsters() {
            return Err(PlayerBodyError::Profile.into());
        }
        if !(0..=1000).contains(&request.amount) {
            return Err(CombatError::Damage.into());
        }
        let victim = *host
            .access
            .get(request.target)
            .ok_or(CombatError::Reference)?;
        let life = victim.life().ok_or(CombatError::Metadata)?;
        let inflictor = request
            .inflictor
            .map(|id| host.access.get(id).copied().ok_or(CombatError::Reference))
            .transpose()?;
        // The barrels policy's bomb source is a barrel's stored `target`, which may
        // name an actor retired since the wound (C reads freed memory: undefined).
        // A source that no longer resolves refuses the tic before any mutation of
        // this victim (M3h9b decision F4(iv)); pending storage stays readable.
        if policy.has_barrels() && request.source.is_some_and(|s| host.access.get(s).is_none()) {
            return Err(PlayerBodyError::Invariant.into());
        }
        let live = life.health > 0 && life.flags & MF_SHOOTABLE != 0;
        // p_inter.c:806–811: no thrust when the source's player has the chainsaw
        // ready (Episode 1 policy; no older policy gives the chainsaw). The weapon
        // machine writes the ready weapon back only after its operation, and no
        // operation fires after a change of the ready weapon (`A_Lower` then
        // `A_Raise` from the bottom), so the stored value is the source's.
        let thrust = !(policy.has_episode1()
            && request.source.is_some()
            && request.source == player.mo
            && player
                .weapon
                .and_then(crate::player_weapons::PlayerWeaponRuntime::pack)
                .is_some_and(|w| w.ready == Weapon::Chainsaw));
        let impulse = |origin: &Actor, mass: i32| {
            let airborne = origin
                .life()
                .is_some_and(|l| crate::original_registry::is_missile_type(l.actor_type));
            if airborne {
                original_player::preflight_missile_impulse(
                    player.room,
                    player.policy.has_heights(),
                    &victim,
                    origin,
                    request.amount,
                    mass,
                )
            } else {
                original_player::preflight_grounded_impulse(
                    player.room,
                    player.policy.has_heights(),
                    &victim,
                    origin,
                    request.amount,
                    mass,
                )
            }
        };
        let mass = host
            .registry
            .actor_type(life.actor_type)
            .ok_or(CombatError::Metadata)?
            .mass();
        if player.mo == Some(request.target) {
            if let (true, Some(origin)) = (live && thrust, inflictor.as_ref()) {
                impulse(origin, mass)?;
            }
            let mut amount = request.amount;
            if policy.has_episode1() && live {
                let sector = usize::from(victim.location.ok_or(PlayerBodyError::Invariant)?.sector);
                let special = match special {
                    Some(special) => special,
                    None => {
                        host.specials
                            .ok_or(PlayerBodyError::Profile)?
                            .get(sector)
                            .ok_or(PlayerBodyError::Invariant)?
                            .special
                    }
                };
                if special == 11 && amount >= life.health {
                    amount = life.health - 1;
                }
            }
            return Self::current_armored_body_capped(
                host.registry,
                victim,
                request,
                amount,
                player.armor,
                policy,
            )
            .map(|plan| plan.with_thrust(thrust));
        }
        // The exploding barrel of the barrels policy (M3h9b note §2.3, decision F4)
        // is the one non-monster damage target: no armor; not `MF_COUNTKILL`, so no
        // kill credit below; no drop (`source_drop` names former humans only); its
        // death chain `S_BEXP..S_BEXP5` has no spawning action, so the spawn demand
        // is 0. Its thrust uses the shared impulse proof with mass 100 and radius 10:
        // `damage * 8192 * 100 / 100` is at most 1000 · 8192 (the request bound), and
        // `travel_admitted` bounds the resulting momentum like any actor's.
        let barrel = crate::barrels::is_barrel(policy, life.actor_type);
        if !barrel && !crate::original_registry::is_policy_monster(policy, life.actor_type) {
            return Err(MonsterError::Target.into());
        }
        if let (true, Some(origin)) = (live && thrust, inflictor.as_ref()) {
            impulse(origin, mass)?;
        }
        let mut plan = Self::start(victim, request, None, policy)?;
        if plan.ignored {
            return Ok(plan);
        }
        if plan.killed && life.flags & MF_COUNTKILL != 0 {
            host.statistics
                .as_deref()
                .ok_or(MapSpawnError::Ownership)?
                .single_player_kill_count
                .checked_add(1)
                .ok_or(MapSpawnError::StatisticsOverflow)?;
            plan.credit = true;
        }
        plan.finish_with(
            victim,
            host.registry,
            |state| {
                crate::original_registry::state_role_for(policy, life.actor_type, state)
                    .map_err(|_| MonsterError::Definition)?;
                if barrel {
                    host.registry.validate_state_barrel(state)?;
                } else {
                    host.registry.validate_state_original_monsters(state)?;
                }
                Ok(())
            },
            true,
        )
        .map(|plan| plan.with_thrust(thrust))
    }
    fn with_thrust(mut self, thrust: bool) -> Self {
        self.thrust = thrust;
        self
    }
    /// The telefrag of wire 24 (decisions I3 and I8): `PIT_StompThing`'s
    /// `P_DamageMobj (thing, tmthing, tmthing, 10000)` (p_map.c:105), planned like
    /// [`Self::current_original`] for a monster or barrel victim, except that the
    /// amount is exactly [`crate::teleport::TELEFRAG_DAMAGE`] and the thrust is the
    /// wrapped 32-bit value the genuine strict builds compute
    /// ([`telefrag_thrust`]). Only the player stomps on maps 1–9 (p_map.c:102) and
    /// the level spawns no second player-linked body, so the victim is never the
    /// player. Every other damage keeps the 1,000 bound under every policy.
    pub(crate) fn telefrag<A: ActorAccess>(
        host: &LifeHost<'_, A>,
        request: DamageRequest,
    ) -> Result<Self, SimError> {
        let player = host.player.as_deref().ok_or(PlayerBodyError::Profile)?;
        let policy = player.policy;
        if !policy.has_teleport()
            || request.amount != crate::teleport::TELEFRAG_DAMAGE
            || request.inflictor.is_none()
            || request.inflictor != request.source
            || request.source != player.mo
            || player.mo == Some(request.target)
        {
            return Err(CombatError::Damage.into());
        }
        let victim = *host
            .access
            .get(request.target)
            .ok_or(CombatError::Reference)?;
        let life = victim.life().ok_or(CombatError::Metadata)?;
        let origin = *host
            .access
            .get(request.inflictor.ok_or(CombatError::Reference)?)
            .ok_or(CombatError::Reference)?;
        let barrel = crate::barrels::is_barrel(policy, life.actor_type);
        if !barrel && !crate::original_registry::is_policy_monster(policy, life.actor_type) {
            return Err(MonsterError::Target.into());
        }
        // p_inter.c:806–811: no thrust when the source's player has the chainsaw
        // ready (pendingweapon does not matter).
        let thrust = !player
            .weapon
            .and_then(crate::player_weapons::PlayerWeaponRuntime::pack)
            .is_some_and(|w| w.ready == Weapon::Chainsaw);
        let mass = host
            .registry
            .actor_type(life.actor_type)
            .ok_or(CombatError::Metadata)?
            .mass();
        if life.health > 0 && life.flags & MF_SHOOTABLE != 0 && thrust {
            original_player::preflight_thrust(
                player.room,
                &victim,
                &origin,
                telefrag_thrust(request.amount, mass)?,
            )?;
        }
        let mut plan = Self::start(victim, request, None, policy)?;
        plan.telefrag = true;
        if plan.ignored {
            return Ok(plan);
        }
        if plan.killed && life.flags & MF_COUNTKILL != 0 {
            host.statistics
                .as_deref()
                .ok_or(MapSpawnError::Ownership)?
                .single_player_kill_count
                .checked_add(1)
                .ok_or(MapSpawnError::StatisticsOverflow)?;
            plan.credit = true;
        }
        plan.finish_with(
            victim,
            host.registry,
            |state| {
                crate::original_registry::state_role_for(policy, life.actor_type, state)
                    .map_err(|_| MonsterError::Definition)?;
                if barrel {
                    host.registry.validate_state_barrel(state)?;
                } else {
                    host.registry.validate_state_original_monsters(state)?;
                }
                Ok(())
            },
            true,
        )
        .map(|plan| plan.with_thrust(thrust))
    }
}
/// The telefrag thrust of wire 24 (decision I3): `damage*(FRACUNIT>>3)*100/mass`
/// (p_inter.c:817) is `int` arithmetic, and for damage 10,000 the product
/// 8,192,000,000 exceeds `INT_MAX`. The genuine `strict` (`-fwrapv`) and `strict32`
/// builds wrap it to −397,934,592 before the signed division, which truncates
/// toward zero: −3,979,345 for mass 100, −994,836 for 400, −397,934 for 1,000 (M4b
/// note §2 T8.5, observed in TFRAG1). Only the stomp's 10,000 is computed this way.
pub(crate) fn telefrag_thrust(amount: i32, mass: i32) -> Result<i32, SimError> {
    if amount != crate::teleport::TELEFRAG_DAMAGE || mass <= 0 {
        return Err(CombatError::Damage.into());
    }
    let product = (amount as u32)
        .wrapping_mul(THRUST_FRACTION as u32)
        .wrapping_mul(THRUST_MASS_SCALE as u32) as i32;
    Ok(product / mass)
}
impl PreparedAttack {
    pub(crate) fn current(
        shooter: ActorId,
        attack: LineAttack,
        result: AttackResult,
        plan: Option<DamagePlan>,
    ) -> Self {
        Self {
            shooter,
            attack,
            result,
            plan,
            sky: None,
        }
    }
    /// The same attack whose trace ended at a sky ceiling (no hit, no puff).
    pub(crate) fn with_sky(mut self, sky: Option<crate::SpecialEvent>) -> Self {
        self.sky = sky;
        self
    }
    pub fn parameters(&self) -> LineAttack {
        self.attack
    }
    pub fn spawn_count(&self) -> usize {
        usize::from(self.result.hit.is_some()) + self.plan.map_or(0, |p| p.demand.spawns)
    }
}
impl Simulation {
    /// Profile 5 using recognized source definitions; no THINGS spawned implicitly.
    pub fn combat(map: Map) -> Result<Self, SimError> {
        Self::with_combat_registry(map, 1024, ActorRegistry::vanilla(), ActorOptions::default())
    }
    /// Immutable native/source definitions for the declared unbound combat slice.
    pub fn with_combat_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        registry.validate_impact_source()?;
        if map
            .sectors()
            .iter()
            .any(|s| s.flats[1].as_bytes().eq_ignore_ascii_case(b"F_SKY1\0\0"))
        {
            return Err(CombatError::Sky.into());
        }
        let mut sim = Self::with_actor_registry(map, limit, registry, options)?;
        sim.profile = Profile::Combat;
        // Impacts are spawned through the infallible validated lifecycle host.
        // Prove their dimensions/closures fit before admitting any attack.
        for name in ["MT_PUFF", "MT_BLOOD"] {
            sim.validate_combat_type(lifecycle::source_type_id(name).unwrap())?;
        }
        Ok(sim)
    }
    pub(crate) fn validate_combat_type(&self, ty: ActorTypeId) -> Result<(), SimError> {
        let registry = self.registry.as_deref().ok_or(CombatError::Profile)?;
        if self.profile.uses_monsters() {
            return self.validate_monster_type(ty);
        }
        if self.profile == Profile::Projectiles {
            self.validate_projectile_type(ty)?;
        } else {
            registry.validate_type_combat(ty, &self.sectors)?;
        }
        let info = registry.actor_type(ty).ok_or(ActorError::InvalidType)?;
        if self.profile == Profile::PlayerWeapons {
            self.validate_weapon_type(ty)?;
        }
        if info.flags() & 4 != 0 {
            if info.mass() <= 0
                || info.spawn_health() <= 0
                || !(0..=256).contains(&info.pain_chance())
            {
                return Err(CombatError::Metadata.into());
            }
            for state in [
                info.states()[1],
                info.states()[2],
                info.states()[5],
                info.states()[6],
            ] {
                self.validate_entry(state)?;
            }
        }
        Ok(())
    }
    fn combat_actor(&self, id: ActorId) -> Result<Actor, SimError> {
        self.combat_actor_with(id, None)
    }
    fn combat_actor_with(&self, id: ActorId, shooter: Option<ActorId>) -> Result<Actor, SimError> {
        if !matches!(
            self.profile,
            Profile::Combat
                | Profile::PlayerWeapons
                | Profile::Projectiles
                | Profile::MonsterAttacks
                | Profile::Shotgunner
                | Profile::Chaingunner
                | Profile::DemonMelee
                | Profile::DemonLifecycle
                | Profile::DemonStandRun
                | Profile::DemonMapSpawn
                | Profile::DemonMapContext
                | Profile::DemonExpandedMap
                | Profile::OriginalDemon
        ) {
            return Err(CombatError::Profile.into());
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(CombatError::Reference.into());
        }
        if self.bindings.contains(&Some(id))
            && !(self.profile == Profile::PlayerWeapons && shooter == Some(id))
        {
            return Err(CombatError::Player.into());
        }
        self.actor(id).copied().ok_or(CombatError::Reference.into())
    }
    /// Pure source autoaim query. This is not general P_CheckSight or AI perception.
    pub fn aim_line_attack(
        &self,
        shooter: ActorId,
        angle: u32,
        range: Fixed,
    ) -> Result<AimResult, SimError> {
        if self.profile.uses_checked_demons() {
            return Err(CombatError::Profile.into());
        }
        if self.profile.uses_monsters() && !self.profile.uses_demon_lifecycle() {
            return Err(CombatError::Profile.into());
        }
        self.combat_actor(shooter)?;
        if self.profile.uses_demon_lifecycle()
            && self.actor(shooter).unwrap().life.unwrap().actor_type != self.demon_body_type()
        {
            return Err(MonsterError::Definition.into());
        }
        Ok(crate::combat_trace::aim(self, shooter, angle, range)?)
    }
    pub(crate) fn plan_damage(&self, request: DamageRequest) -> Result<DamagePlan, SimError> {
        self.plan_damage_with(request, None)
    }
    pub(crate) fn plan_weapon_damage(
        &self,
        request: DamageRequest,
        shooter: ActorId,
    ) -> Result<(), SimError> {
        self.plan_damage_with(request, Some(shooter)).map(|_| ())
    }
    fn plan_damage_with(
        &self,
        request: DamageRequest,
        shooter: Option<ActorId>,
    ) -> Result<DamagePlan, SimError> {
        self.plan_damage_lookup(request, shooter, None)
    }
    fn plan_damage_lookup(
        &self,
        request: DamageRequest,
        shooter: Option<ActorId>,
        lookup: Option<&dyn Fn(ActorId) -> Option<Actor>>,
    ) -> Result<DamagePlan, SimError> {
        self.preflight_player_damage(request)?;
        self.preflight_map_damage(request)?;
        let base = self.combat_actor(request.target)?;
        let actor = match lookup {
            Some(f) => f(request.target).ok_or(CombatError::Reference)?,
            None => base,
        };
        for id in [request.inflictor, request.source].into_iter().flatten() {
            self.combat_actor_with(id, shooter)?;
        }
        if !(0..=1000).contains(&request.amount) {
            return Err(CombatError::Damage.into());
        }
        let life = actor.life.unwrap();
        let armor = self
            .original_player
            .as_ref()
            .filter(|p| p.mo == Some(request.target))
            .and_then(|p| p.armor);
        let mut plan = DamagePlan::start(actor, request, armor, self.player_policy())?;
        if plan.ignored {
            return Ok(plan);
        }
        self.validate_combat_type(life.actor_type)?;
        let registry = self.registry.as_deref().unwrap();
        if life.flags & MF_COUNTKILL != 0 && plan.killed {
            let stats = self.statistics.ok_or(CombatError::KillBranch)?;
            stats
                .single_player_kill_count
                .checked_add(1)
                .ok_or(MapSpawnError::StatisticsOverflow)?;
            plan.credit = true;
        }

        if let Some(inflictor) = request.inflictor {
            let from = match lookup {
                Some(f) => f(inflictor).ok_or(CombatError::Reference)?,
                None => *self.actor(inflictor).unwrap(),
            }
            .position();
            for (a, b) in actor.position().into_iter().zip(from) {
                let delta = a
                    .bits()
                    .checked_sub(b.bits())
                    .ok_or(CombatError::Coordinates)?;
                delta.checked_abs().ok_or(CombatError::Coordinates)?;
            }
        }
        plan.finish(actor, registry, |state| self.validate_entry(state))
    }

    /// Atomic supported P_DamageMobj operation. No full-simulation clone is used.
    ///
    /// Under the barrels policy a barrel is refused as the `source`
    /// (`PlayerBodyError::Unsupported`, state unchanged): the source never passes
    /// one (sources are a shooter, a missile's `target`, the bomb source or a skull;
    /// `A_Explode` passes the barrel only as the inflictor, p_enemy.c:1600,
    /// p_map.c:1194), and a barrel source would retarget its victim to the barrel,
    /// which no admitted monster may target. A barrel stays admitted as the
    /// inflictor and as the target (M3h9b decision F4(v)).
    pub fn damage_actor(&mut self, request: DamageRequest) -> Result<DamageResult, SimError> {
        if self.profile.uses_monsters() && !self.profile.uses_demon_lifecycle() {
            return Err(CombatError::Profile.into());
        }
        let policy = self.player_policy();
        if request.source.is_some_and(|id| {
            self.actor(id)
                .and_then(Actor::life)
                .is_some_and(|l| crate::barrels::is_barrel(policy, l.actor_type))
        }) {
            return Err(crate::PlayerBodyError::Unsupported.into());
        }
        if self.profile.uses_demon_lifecycle() {
            self.preflight_lifecycle_combat(&[
                Some(request.target),
                request.source,
                request.inflictor,
            ])?;
        }
        let plan = self.plan_damage(request)?;
        if self.profile.uses_checked_demons() {
            if plan.spawn_count() != 0 {
                return Err(CombatError::WorkLimit.into());
            }
            return self.checked_demon_operation(|host| host.apply_damage(plan));
        }
        self.reserve_actor_spawns(plan.demand.spawns)?;
        Ok(self.combat_host().apply_damage(plan))
    }
    /// Explicit supported P_LineAttack. Impact thinkers/RNG precede damage;
    /// zero damage still creates the source impact. Errors preserve canonical state.
    pub fn line_attack(
        &mut self,
        shooter: ActorId,
        attack: LineAttack,
    ) -> Result<AttackResult, SimError> {
        if self.profile.uses_checked_demons() {
            return Err(CombatError::Profile.into());
        }
        if self.profile.uses_monsters() && !self.profile.uses_demon_lifecycle() {
            return Err(CombatError::Profile.into());
        }
        self.line_attack_with(shooter, attack, None)
    }
    pub(crate) fn weapon_line_attack(
        &mut self,
        shooter: ActorId,
        attack: LineAttack,
    ) -> Result<AttackResult, SimError> {
        self.line_attack_with(shooter, attack, Some(shooter))
    }
    fn line_attack_with(
        &mut self,
        shooter: ActorId,
        attack: LineAttack,
        permit: Option<ActorId>,
    ) -> Result<AttackResult, SimError> {
        if self.profile.uses_demon_lifecycle() {
            self.preflight_lifecycle_combat(&[Some(shooter)])?;
            if self.actor(shooter).unwrap().life.unwrap().actor_type != self.demon_body_type() {
                return Err(MonsterError::Definition.into());
            }
        }
        let prepared = self.prepare_attack(shooter, attack, permit)?;
        self.reserve_actor_spawns(prepared.spawn_count())?;
        Ok(self.combat_host().execute_attack(prepared))
    }
    pub(crate) fn prepare_attack(
        &self,
        shooter: ActorId,
        attack: LineAttack,
        permit: Option<ActorId>,
    ) -> Result<PreparedAttack, SimError> {
        self.prepare_attack_lookup(shooter, attack, permit, None)
    }
    pub(crate) fn prepare_attack_lookup(
        &self,
        shooter: ActorId,
        attack: LineAttack,
        permit: Option<ActorId>,
        lookup: Option<&dyn Fn(ActorId) -> Option<Actor>>,
    ) -> Result<PreparedAttack, SimError> {
        self.combat_actor_with(shooter, permit)?;
        if !(0..=1000).contains(&attack.damage) {
            return Err(CombatError::Damage.into());
        }
        let trace = crate::combat_trace::collect(self, shooter, attack.angle, attack.range)?;
        let (hit, position, traversed) = match lookup {
            Some(f) => crate::combat_trace::hit_with(
                self,
                shooter,
                &trace,
                attack.range,
                attack.slope,
                &|id| f(id),
            ),
            None => crate::combat_trace::hit(self, shooter, &trace, attack.range, attack.slope),
        };
        let plan = match hit {
            Some(TraceObject::Actor(target)) if attack.damage != 0 => {
                Some(self.plan_damage_lookup(
                    DamageRequest {
                        target,
                        inflictor: Some(shooter),
                        source: Some(shooter),
                        amount: attack.damage,
                    },
                    permit,
                    lookup,
                )?)
            }
            _ => None,
        };

        Ok(PreparedAttack {
            shooter,
            attack,
            plan,
            sky: None,
            result: AttackResult {
                trace,
                hit,
                position,
                traversed,
                effect: None,
                damage: None,
            },
        })
    }
    fn combat_host(&mut self) -> LifeHost<'_, &mut ThinkerStore<Actor>> {
        if self.profile.uses_demon_lifecycle() {
            return self.monster_host(None);
        }
        LifeHost {
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
    }
}
pub(crate) fn pain_decision(draw: u8, chance: i32) -> bool {
    i32::from(draw) < chance
}
impl<A: ActorAccess> LifeHost<'_, A> {
    fn combat_event(&mut self, event: CombatEvent) {
        self.events.push(Event::Combat(event));
    }
    fn combat_random(&mut self, kind: u8) -> u8 {
        let value = self.rng.p_random();
        self.combat_event(CombatEvent::Random { kind, value });
        value
    }
    pub(crate) fn apply_damage(&mut self, plan: DamagePlan) -> DamageResult {
        let DamagePlan {
            request,
            health,
            killed,
            extreme,
            state,
            ignored,
            ..
        } = plan;
        let id = request.target;
        self.combat_event(CombatEvent::Damage(request));
        let mut result = DamageResult {
            ignored,
            health,
            pain: false,
            killed,
            extreme,
        };
        if ignored {
            self.combat_event(CombatEvent::Ignored(id));
            return result;
        }
        let old = *self.access.get(id).unwrap();
        let life = old.life.unwrap();
        let info = self.registry.actor_type(life.actor_type).unwrap().clone();
        if let Some(inflictor) = request.inflictor.filter(|_| plan.thrust) {
            let origin = *self.access.get(inflictor).unwrap();
            let delta = [
                old.position()[0] - origin.position()[0],
                old.position()[1] - origin.position()[1],
            ];
            let mut angle = movement::point_angle(delta);
            let mut thrust = if plan.telefrag {
                telefrag_thrust(request.amount, info.mass()).expect("planned telefrag thrust")
            } else {
                request.amount * THRUST_FRACTION * THRUST_MASS_SCALE / info.mass()
            };
            if request.amount < 40
                && request.amount > life.health
                && old.z() - origin.z() > Fixed::from_map_units(64)
                && self.combat_random(0) & 1 != 0
            {
                angle = angle.wrapping_add(0x80000000);
                thrust *= 4;
            }
            let thrust = Fixed::from_bits(thrust);
            let phase = (angle >> 19) as usize;
            let a = self.access.get_mut(id).unwrap();
            a.spec.momentum[0] =
                a.spec.momentum[0] + thrust * Fixed::from_bits(FINE_SINE[phase + 2048]);
            a.spec.momentum[1] = a.spec.momentum[1] + thrust * Fixed::from_bits(FINE_SINE[phase]);
            let momentum = a.spec.momentum;
            self.combat_event(CombatEvent::Knockback {
                target: id,
                angle,
                thrust,
                momentum,
            });
        }
        let effective = plan.armor.map_or(plan.amount, |a| a.effective);
        if let Some(armor) = plan.armor {
            if let Some(p) = self.player.as_deref_mut() {
                p.armor = Some(armor.after);
            }
            self.events.push(Event::Armor(armor));
        }
        if let Some(p) = self.player.as_deref_mut().filter(|p| p.mo == Some(id)) {
            p.health = (p.health - effective).max(0);
            p.attacker = request.source;
            p.damage_count = (p.damage_count + effective).min(original_player::MAX_DAMAGE_COUNT);
            self.events
                .push(Event::PlayerBody(PlayerBodyEvent::Damaged {
                    health: p.health,
                    attacker: p.attacker,
                    damage_count: p.damage_count,
                    tactile: original_player::TACTILE_BASE
                        + effective.min(original_player::MAX_DAMAGE_COUNT) * 2,
                }));
        }
        self.access
            .get_mut(id)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .health = health[1];
        self.combat_event(CombatEvent::Health {
            target: id,
            values: health,
        });
        if killed {
            let a = self.access.get_mut(id).unwrap();
            let life = a.life.as_mut().unwrap();
            life.flags &= !(MF_SHOOTABLE | MF_FLOAT | MF_SKULLFLY);
            if !lifecycle::source_type_is(life.actor_type, "MT_SKULL") {
                life.flags &= !MF_NOGRAVITY;
            }
            life.flags |= MF_CORPSE | MF_DROPOFF;
            a.spec.height = Fixed::from_bits(a.spec.height.bits() >> 2);
            a.allow_dropoff = true;
            a.gravity = if life.flags & MF_NOGRAVITY != 0 {
                Gravity::None
            } else {
                Gravity::Normal
            };
            if plan.credit {
                let stats = self
                    .statistics
                    .as_mut()
                    .expect("preflight admitted fallback credit");
                stats.single_player_kill_count += 1;
                self.events
                    .push(Event::Statistics(StatisticsEvent::SinglePlayerKill {
                        actor: id,
                        total: stats.single_player_kill_count,
                    }));
            }
            if let Some(p) = self.player.as_deref_mut().filter(|p| p.mo == Some(id)) {
                if request.source.is_none() || request.source == p.mo {
                    p.frags += 1;
                }
                p.state = PlayerBodyState::Dead;
                if let Some(a) = self.access.get_mut(id) {
                    if let Some(l) = &mut a.life {
                        l.flags &= !original_player::MF_SOLID;
                    }
                    a.solidity = Solidity::NonSolid;
                }
                self.events.push(Event::PlayerBody(PlayerBodyEvent::Died {
                    frags: p.frags,
                    weapon_down: p.weapon.and_then(|w| w.pack()).map_or(
                        identity::OriginalState::S_PISTOLDOWN,
                        |w| match w.ready {
                            Weapon::Fist => identity::OriginalState::S_PUNCHDOWN,
                            Weapon::Shotgun => identity::OriginalState::S_SGUNDOWN,
                            Weapon::Chaingun => identity::OriginalState::S_CHAINDOWN,
                            Weapon::Rocket => identity::OriginalState::S_MISSILEDOWN,
                            Weapon::Chainsaw => identity::OriginalState::S_SAWDOWN,
                            _ => identity::OriginalState::S_PISTOLDOWN,
                        },
                    ),
                }));
            }
            if self.player.as_ref().is_some_and(|p| p.mo == Some(id)) {
                if let Err(error) = player_pistol::drop_weapon(self) {
                    self.fail_sight(error);
                    return result;
                }
            }
            self.combat_event(CombatEvent::Kill {
                target: id,
                state,
                extreme,
            });
            self.enter(id, state);
            let random = self.combat_random(2);
            let life = self.access.get_mut(id).unwrap().life.as_mut().unwrap();
            life.tics = (life.tics - i32::from(random & 3)).max(1);
            let tics = life.tics;
            self.combat_event(CombatEvent::DeathTimer { target: id, tics });
            if self
                .player
                .as_ref()
                .is_some_and(|p| p.policy.has_monsters())
            {
                if let Some(item) = source_drop(old.life.unwrap().actor_type) {
                    // p_inter.c P_KillMobj: P_SpawnMobj(x, y, ONFLOORZ), then MF_DROPPED.
                    let position = self.access.get(id).unwrap().position();
                    let ty = ActorTypeId(item.ordinal());
                    let fits = !self.player.as_ref().is_some_and(|p| p.policy.has_heights())
                        || self.registry.actor_type(ty).is_some_and(|t| {
                            crate::player_map_scale::spawn_fits(
                                self.world,
                                self.sectors,
                                position,
                                SpawnHeight::Floor,
                                t.height(),
                            )
                        });
                    if fits {
                        let drop = self.spawn(ty, position, SpawnHeight::Floor);
                        self.access
                            .get_mut(drop)
                            .unwrap()
                            .life
                            .as_mut()
                            .unwrap()
                            .flags |= crate::world_pickups::MF_DROPPED;
                    } else {
                        // Rust-only: a drop taller than the corpse's sector would stand
                        // through the ceiling in the source; the checked tic fails and
                        // rolls back instead (M3h4 note, section 4.3).
                        self.fail_sight(crate::MapScaleError::Fit.into());
                    }
                }
            }
            return result;
        }
        let random = self.combat_random(1);
        if pain_decision(random, info.pain_chance()) {
            result.pain = true;
            self.access
                .get_mut(id)
                .unwrap()
                .life
                .as_mut()
                .unwrap()
                .flags |= MF_JUSTHIT;
            self.combat_event(CombatEvent::Pain {
                target: id,
                state: info.states()[2],
            });
            self.enter(id, info.states()[2]);
        }
        self.access
            .get_mut(id)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .reaction_time = 0;
        self.combat_event(CombatEvent::Reaction(id));
        if let Some(source) = request.source {
            let current = self.access.get(id).unwrap().life.unwrap();
            // Every planner proves the source readable: the historical ones through
            // `combat_actor_with`, the barrels policy's `current_original` for a bomb
            // source. A staged tic still refuses rather than panic if it is not.
            let Some(source_type) = self
                .access
                .get(source)
                .and_then(Actor::life)
                .map(|l| l.actor_type)
            else {
                if self.perception.and_then(|p| p.checked).is_some() {
                    if !self.sight_failed() {
                        self.fail_sight(PlayerBodyError::Invariant.into());
                    }
                    return result;
                }
                unreachable!("planned damage source proved readable");
            };
            if (current.threshold == 0 || lifecycle::source_type_is(current.actor_type, "MT_VILE"))
                && source != id
                && !lifecycle::source_type_is(source_type, "MT_VILE")
            {
                self.target(id, Some(source));
                self.access
                    .get_mut(id)
                    .unwrap()
                    .life
                    .as_mut()
                    .unwrap()
                    .threshold = BASETHRESHOLD;
                self.combat_event(CombatEvent::Retarget { target: id, source });
                if self.access.get(id).unwrap().life.unwrap().state == Some(info.states()[0])
                    && info.states()[1].0 != 0
                {
                    self.enter(id, info.states()[1]);
                }
            }
        }
        result
    }
    fn execute_attack(&mut self, prepared: PreparedAttack) -> AttackResult {
        let PreparedAttack {
            shooter,
            attack,
            mut result,
            plan,
            sky,
        } = prepared;
        let hit = result.hit;
        let position = result.position;
        self.combat_event(CombatEvent::Attack { shooter, attack });
        self.combat_event(CombatEvent::Trace {
            result: Box::new(result.clone()),
        });
        if let Some(sky) = sky {
            self.events.push(Event::Special(sky));
        }
        if let Some(mut position) = position {
            let blood = matches!(hit,Some(TraceObject::Actor(id)) if self.access.get(id).unwrap().life.unwrap().flags&MF_NOBLOOD==0);
            let first = self.combat_random(3);
            let second = self.combat_random(4);
            position[2] =
                position[2] + Fixed::from_bits((i32::from(first) - i32::from(second)) * 1024);
            let ty = lifecycle::source_type_id(if blood { "MT_BLOOD" } else { "MT_PUFF" }).unwrap();
            let id = self.spawn(
                ty,
                [position[0], position[1]],
                SpawnHeight::Absolute(position[2]),
            );
            self.access.get_mut(id).unwrap().momz =
                Fixed::from_map_units(if blood { 2 } else { 1 });
            let random = self.combat_random(5);
            let life = self.access.get_mut(id).unwrap().life.as_mut().unwrap();
            life.tics = (life.tics - i32::from(random & 3)).max(1);
            let state = if blood && attack.damage < 9 {
                Some("S_BLOOD3")
            } else if blood && attack.damage <= 12 {
                Some("S_BLOOD2")
            } else if !blood && attack.range == Fixed::from_map_units(64) {
                Some("S_PUFF3")
            } else {
                None
            };
            if let Some(name) = state {
                self.enter(id, lifecycle::source_state_id(name).unwrap());
            }
            self.combat_event(CombatEvent::Impact { actor: id });
            result.effect = Some(id);
        }
        if let Some(plan) = plan {
            result.damage = Some(self.apply_damage(plan));
        }
        result
    }
}
pub(crate) fn execute_attack<A: ActorAccess>(
    host: &mut LifeHost<'_, A>,
    prepared: PreparedAttack,
) -> AttackResult {
    host.execute_attack(prepared)
}
