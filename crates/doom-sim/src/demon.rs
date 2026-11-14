//! Normal-timing source SARG melee through shared range and damage services.
use crate::{
    lifecycle::{ActorAccess, LifeHost},
    *,
};
#[derive(Clone, Debug)]
pub(crate) struct MeleePlan {
    owner: ActorId,
    initial: DoomRng,
    accepted: Option<bool>,
    damage: Option<(i32, crate::combat::DamagePlan)>,
}
pub(crate) fn strike<A: ActorAccess>(host: &mut LifeHost<'_, A>, id: ActorId) {
    if host.perception.is_some_and(|input| {
        matches!(
            input.players,
            crate::player_roster::PerceptionRoster::OriginalPlayer(_)
        )
    }) {
        if host
            .perception
            .is_some_and(|input| input.players.allows_player_melee())
        {
            if let Err(error) = strike_player(host, id) {
                host.fail_sight(error);
            }
        } else {
            host.fail_sight(PlayerBodyError::Unsupported.into());
        }
        return;
    }
    if host.access.get(id).unwrap().life.unwrap().target.is_none() {
        return;
    }
    let plan = if host.perception.unwrap().map_demons {
        current_map_melee(host, id)
    } else {
        host.perception
            .unwrap()
            .melee
            .expect("complete melee preflight before mutation")
            .clone()
    };
    assert_eq!(plan.owner, id);
    assert_eq!(plan.initial, *host.rng);
    crate::monsters::face(host, id);
    // The actual source query follows facing. Its pure preview proved all later
    // dependencies; facing cannot change this angle-independent sight/range result.
    let accepted = crate::monsters::melee_host(host, id);
    if let Some(preview) = plan.accepted {
        assert_eq!(accepted, preview);
    }
    if let Some((amount, damage)) = plan.damage.filter(|_| accepted) {
        let actual = (i32::from(crate::monsters::draw(host, id, 4)) % 10 + 1) * 4;
        assert_eq!(actual, amount);
        host.apply_damage(damage);
    }
}

// p_enemy.c A_SargAttack and the existing MonsterEvent random-kind namespace.
const SARG_DAMAGE_CHOICES: i32 = 10;
const SARG_DAMAGE_UNIT: i32 = 4;
const SARG_DAMAGE_DRAW: u8 = 4;

/// Runs only inside the existing checked action transaction. No future hit,
/// health or RNG is predicted: Face/range are pure with respect to target fields,
/// then the shared DamagePlan reads the current staged body at actual attack time.
fn strike_player<A: ActorAccess>(host: &mut LifeHost<'_, A>, id: ActorId) -> Result<(), SimError> {
    use crate::identity::{OriginalActorType, OriginalState};
    let owner = host.access.get(id).ok_or(CombatError::Reference)?;
    let life = owner.life().ok_or(MonsterError::Definition)?;
    // The heights policy admits a Demon or player off the floor; A_SargAttack reads
    // neither z, and the shared damage service carries the vertical state.
    let heights = host
        .player
        .as_deref()
        .is_some_and(|p| p.policy.has_heights());
    // The Spectre shares the Demon's `S_SARG_ATK3` and `A_SargAttack` (info.c:
    // 1446–1470; Episode 1 policy, M4a plan §E.5).
    let spectre = crate::original_registry::is_spectre(life.actor_type)
        && host
            .player
            .as_deref()
            .is_some_and(|p| p.policy.has_episode1());
    if (life.actor_type.0 != OriginalActorType::MT_SERGEANT.ordinal() && !spectre)
        || life.health <= 0
        || life.state != Some(StateId(OriginalState::S_SARG_ATK3.ordinal()))
        || (!heights && (owner.z() != Fixed::ZERO || owner.vertical_momentum() != Fixed::ZERO))
    {
        return Err(MonsterError::Definition.into());
    }
    let Some(target) = life.target else {
        return Ok(());
    };
    let player = host.player.as_deref().ok_or(PlayerBodyError::Binding)?;
    if player.policy.has_monsters() && player.mo != Some(target) {
        return strike_monster(host, id, target);
    }
    let body = host.access.get(target).ok_or(CombatError::Reference)?;
    let body_life = body.life().ok_or(PlayerBodyError::Invariant)?;
    // Powers/SHADOW, cheats and arbitrary player flag mutations are unadmitted.
    // JUSTHIT and CORPSE are the source damage/death additions to pristine flags.
    // Wire 24 also admits `A_Saw`'s MF_JUSTATTACKED, set by the player's think
    // earlier in the same tic (p_pspr.c:542) and read by nothing on this path
    // (`has_bite_on_lunging_body`); `apply_damage` keeps it on a kill, as
    // `P_KillMobj` does (p_inter.c:675–680).
    let lunge = if player.policy.has_bite_on_lunging_body() {
        player_commands::MF_JUSTATTACKED
    } else {
        0
    };
    // The four terms are pairwise disjoint bit sets (MT_PLAYER's `MF_SOLID|
    // MF_SHOOTABLE|MF_DROPOFF|MF_PICKUP|MF_NOTDMATCH`, info.c:1130, against
    // `MF_JUSTHIT` 0x40, `MF_JUSTATTACKED` 0x80 and `MF_CORPSE` 0x100000,
    // p_mobj.h:120–193; `saw_bite_tests::the_allowed_flag_terms_are_disjoint`), so
    // each `|` equals `^` here: the mutants `|` → `^` of Mutants run 43543402068 are
    // equivalent.
    let allowed = OriginalActorType::MT_PLAYER.descriptor().flags
        | original_player::MF_JUSTHIT
        | original_player::MF_CORPSE
        | lunge;
    if player.mo != Some(target)
        || body_life.actor_type.0 != OriginalActorType::MT_PLAYER.ordinal()
        || player.health != body_life.health.max(0)
        || (player.state == PlayerBodyState::Dead) != (body_life.health <= 0)
        || body_life.flags & !allowed != 0
        || (!heights && (body.z() != Fixed::ZERO || body.vertical_momentum() != Fixed::ZERO))
    {
        return Err(PlayerBodyError::Invariant.into());
    }
    crate::monsters::face(host, id);
    if !crate::monsters::melee_host(host, id) {
        // A failed checked sight query is already latched for whole-tic rollback.
        return Ok(());
    }
    // Bounded u8 draw makes this source formula4..40 without integer overflow.
    let amount =
        (i32::from(crate::monsters::draw(host, id, SARG_DAMAGE_DRAW)) % SARG_DAMAGE_CHOICES + 1)
            * SARG_DAMAGE_UNIT;
    let body = host.access.get(target).ok_or(CombatError::Reference)?;
    let owner = host.access.get(id).ok_or(CombatError::Reference)?;
    let current = body.life().ok_or(PlayerBodyError::Invariant)?;
    if current.health > 0 && current.flags & original_player::MF_SHOOTABLE != 0 {
        let room = host.player.as_deref().and_then(|p| p.room);
        original_player::preflight_player_impulse(room, heights, body, owner, amount)?;
    }
    let damage = crate::combat::DamagePlan::current_armored_body(
        host.registry,
        *body,
        DamageRequest {
            target,
            inflictor: Some(id),
            source: Some(id),
            amount,
        },
        host.player.as_deref().and_then(|p| p.armor),
        host.player
            .as_deref()
            .ok_or(PlayerBodyError::Binding)?
            .policy,
    )?;
    if damage.spawn_count() != 0 {
        return Err(CombatError::WorkLimit.into());
    }
    host.apply_damage(damage);
    Ok(())
}
/// PlayerMonstersV1 infighting bite: the same A_SargAttack face/range/draw
/// order, then the shared current damage plan for the staged monster target.
fn strike_monster<A: ActorAccess>(
    host: &mut LifeHost<'_, A>,
    id: ActorId,
    target: ActorId,
) -> Result<(), SimError> {
    host.access.get(target).ok_or(MonsterError::Target)?;
    crate::monsters::face(host, id);
    if !crate::monsters::melee_host(host, id) {
        return Ok(());
    }
    let amount =
        (i32::from(crate::monsters::draw(host, id, SARG_DAMAGE_DRAW)) % SARG_DAMAGE_CHOICES + 1)
            * SARG_DAMAGE_UNIT;
    let damage = crate::combat::DamagePlan::current_original(
        host,
        DamageRequest {
            target,
            inflictor: Some(id),
            source: Some(id),
            amount,
        },
    )?;
    host.access.reserve(damage.spawn_count())?;
    host.apply_damage(damage);
    Ok(())
}
impl Simulation {
    /// Explicit normal 8/8/8 SARG fixture, not full Demon/Spectre gameplay.
    /// Nightmare's global source timer rewrite is outside this profile.
    pub fn with_demon_melee_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if limit > 64 {
            return Err(PerceptionError::WorkLimit.into());
        }
        if options.skill == Skill::Nightmare {
            return Err(MonsterError::Definition.into());
        }
        if map.linedefs().iter().any(|l| l.special != 0)
            || map.sectors().iter().any(|s| s.special != 0)
        {
            return Err(PursuitError::Specials.into());
        }
        crate::sight::validate_map(&map)?;
        validate_registry(&registry)?;
        let mut sim = Self::with_combat_registry(map, limit, registry, options)?;
        sim.profile = Profile::DemonMelee;
        sim.sound_targets = vec![None; sim.sectors.len()];
        Ok(sim)
    }
    pub(crate) fn validate_demon_type(&self, ty: ActorTypeId) -> Result<(), SimError> {
        let info = self
            .registry
            .as_deref()
            .unwrap()
            .actor_type(ty)
            .ok_or(MonsterError::Definition)?;
        if ![135, 136].contains(&ty.0)
            || ty.0 == 136 && info.name != "FIXTURE_DEMON_MELEE_136"
            || ty.0 == 135 && info.name != "FIXTURE_DAMAGE_TARGET"
        {
            return Err(MonsterError::Definition.into());
        }
        self.registry
            .as_deref()
            .unwrap()
            .validate_type_demon_melee(ty, &self.sectors)?;
        Ok(())
    }
    pub(crate) fn demon_state_role(&self, ty: ActorTypeId, state: StateId) -> Result<(), SimError> {
        let valid = match ty.0 {
            136 => (149..=173).contains(&state.0) || [1, 2, 4, 5, 6].contains(&state.0),
            135 => (149..=173).contains(&state.0),
            _ => false,
        };
        if !valid {
            return Err(MonsterError::Definition.into());
        }
        Ok(())
    }
    pub(crate) fn prepare_demon_melee(
        &mut self,
        id: ActorId,
    ) -> Result<Option<MeleePlan>, SimError> {
        self.preflight_monster_callback(id)?;
        if self.profile.uses_demon_map() {
            return Ok(None);
        }
        let actor = *self.actor(id).unwrap();
        let Some(target) = actor.life.unwrap().target else {
            return Ok(None);
        };
        let initial = self.rng;
        let mut preview = initial;
        let _ = crate::monsters::face_angle(actor, *self.actor(target).unwrap(), &mut preview);
        let accepted = self.check_melee_range(id)?.accepted;
        // Lifecycle owners can move in XY/Z before their first thinker action.
        // Prepare potential damage even when the current preview misses; actual
        // range after physics/facing controls the draw. Physics consumes no RNG
        // in this unbound, non-missile closure, and no earlier thinker can alter
        // target health. The plan contains no cached post-physics range result.
        let damage = if accepted || self.profile.uses_demon_lifecycle() {
            let amount = (i32::from(preview.p_random()) % 10 + 1) * 4;
            let plan = self.plan_damage(DamageRequest {
                target,
                inflictor: Some(id),
                source: Some(id),
                amount,
            })?;
            self.reserve_actor_spawns(plan.spawn_count())?;
            Some((amount, plan))
        } else {
            None
        };
        Ok(Some(MeleePlan {
            owner: id,
            initial,
            accepted: (!self.profile.uses_demon_lifecycle()).then_some(accepted),
            damage,
        }))
    }
    pub(crate) fn preflight_demon_step(&mut self) -> Result<Option<MeleePlan>, SimError> {
        let owner = self.preflight_monster_owner()?;
        if self.profile.uses_demon_map() {
            return Ok(None);
        }
        owner
            .map(|id| self.prepare_demon_melee(id))
            .transpose()
            .map(Option::flatten)
    }
    /// Atomic original A_SargAttack: Face, current range/sight, one damage draw,
    /// shared direct damage. No attack sound or hitscan impact is invented.
    pub fn demon_melee_attack(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.original_player.is_some() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        if !self.profile.uses_demon() {
            return Err(MonsterError::Profile.into());
        }
        if self.profile.uses_checked_demons() {
            self.preflight_monster_callback(id)?;
            return self.checked_demon_operation(|host| {
                host.events.push(Event::ActorState(ActorStateEvent::Action {
                    actor: id,
                    action: ActorAction::Original(lifecycle::source_action_id("A_SargAttack")),
                }));
                strike(host, id);
            });
        }
        let plan = self.prepare_demon_melee(id)?;
        let mut host = self.monster_host(None);
        host.perception.as_mut().unwrap().melee = plan.as_ref();
        host.events.push(Event::ActorState(ActorStateEvent::Action {
            actor: id,
            action: ActorAction::Original(lifecycle::source_action_id("A_SargAttack")),
        }));
        strike(&mut host, id);
        Ok(())
    }
    pub(crate) fn enter_demon_state(
        &mut self,
        id: ActorId,
        state: StateId,
    ) -> Result<(), SimError> {
        if self.original_player.is_some() && !self.uses_active_player_roster() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        let actor = self.monster_actor(id, 1008)?;
        self.monster_state_role(actor.life.unwrap().actor_type, state)?;
        let direct_entry = if self.profile == Profile::OriginalDemon {
            original_registry::direct(actor.life.unwrap().actor_type, state)
        } else if self.profile.uses_checked_demons() {
            NativeDemonState::checked(state).is_some_and(NativeDemonState::allows_direct_entry)
        } else {
            [1, 2, 4, 5, 6].contains(&state.0)
                || self.profile.uses_demon_stand_run()
                    && (state.0 == 3 || (15..=21).contains(&state.0))
        };
        if self.profile.uses_demon_lifecycle()
            && self.is_roster_monster(actor.life.unwrap().actor_type)
            && (actor.life.unwrap().health <= 0 || !direct_entry)
        {
            return Err(MonsterError::Definition.into());
        }
        if self.monster_is_owner(id) {
            self.preflight_monster_callback(id)?;
        }
        self.validate_entry(state)?;
        if self.profile.uses_checked_demons() {
            return self.checked_demon_operation(|host| host.enter(id, state));
        }
        let plan = if state == StateId(6) {
            self.prepare_demon_melee(id)?
        } else {
            None
        };
        let mut host = self.monster_host(None);
        host.perception.as_mut().unwrap().melee = plan.as_ref();
        host.enter(id, state);
        Ok(())
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
        let expected = match i {
            1 | 2 => ActorStateDef {
                name: if i == 1 {
                    "FIXTURE_SARG_LOOK"
                } else {
                    "FIXTURE_SARG_CHASE"
                }
                .into(),
                sprite: 39,
                frame: 0,
                tics: s.tics,
                action: ActorAction::Original(lifecycle::source_action_id(if i == 1 {
                    "A_Look"
                } else {
                    "A_Chase"
                })),
                next: StateId(i as u16),
                misc: [0; 2],
            },
            4..=6 => {
                let original = source
                    .states()
                    .iter()
                    .find(|s| s.name == format!("S_SARG_ATK{}", i - 3))
                    .unwrap();
                let mut expected = original.clone();
                expected.name = format!("FIXTURE_SARG_ATK{}", i - 3);
                expected.next = StateId(if i == 6 { 2 } else { i as u16 + 1 });
                expected
            }
            _ => source.states()[i].clone(),
        };
        if *s != expected || [1, 2].contains(&i) && !(1..=35).contains(&s.tics) {
            return Err(MonsterError::Definition.into());
        }
    }
    for (i, t) in r.types().iter().enumerate() {
        if ![135, 136].contains(&i) {
            if t != &source.types()[i] {
                return Err(MonsterError::Definition.into());
            }
            continue;
        }
        let owner = i == 136;
        let mut expected = source.types()[0].clone();
        expected.name = if owner {
            "FIXTURE_DEMON_MELEE_136"
        } else {
            "FIXTURE_DAMAGE_TARGET"
        }
        .into();
        expected.properties[0] = -1;
        let variable: &[usize] = if owner {
            &[4, 15, 16, 17, 20, 21]
        } else {
            &[2, 8, 16, 17, 21]
        };
        for &key in variable {
            expected.properties[key] = t.properties[key];
        }
        if owner {
            expected.properties[1] = 1;
            expected.properties[3] = 2;
            expected.properties[5] = 8;
            expected.properties[2] = 150;
            expected.properties[6] = 52;
            expected.properties[18] = 400;
            expected.properties[10] = 4;
            expected.properties[11] = 0;
        }
        let flags = if owner {
            2 | 32 | 64 | 128 | 1024 | 2048 | 2097152 | 33554432
        } else {
            2 | 4 | 32 | 64 | 1024 | 2048 | 262144 | 524288 | 33554432
        };
        if *t != expected
            || t.radius() < Fixed::ZERO
            || t.radius() > Fixed::from_map_units(32)
            || t.height() <= Fixed::ZERO
            || t.flags() & !flags != 0
            || owner && !(1..=16).contains(&t.speed())
            || !owner
                && (!(1..=1000).contains(&t.spawn_health())
                    || !(0..=256).contains(&t.pain_chance()))
        {
            return Err(MonsterError::Definition.into());
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "demon_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "player_melee_tests.rs"]
mod player_melee_tests;

/// Global profile16 preflight proves stable body lifetimes/roles and complete
/// zero-allocation closures before any thinker mutation. This small late plan
/// observes earlier owners' actual RNG/health effects, never a copied world.
fn current_map_melee<A: ActorAccess>(host: &LifeHost<'_, A>, id: ActorId) -> MeleePlan {
    let actor = *host.access.get(id).unwrap();
    let target = actor.life.unwrap().target.unwrap();
    let body = *host.access.get(target).unwrap();
    let initial = *host.rng;
    let mut preview = initial;
    let _ = crate::monsters::face_angle(actor, body, &mut preview);
    let amount = (i32::from(preview.p_random()) % 10 + 1) * 4;
    let damage = crate::combat::DamagePlan::current_melee_body(
        host.registry,
        body,
        DamageRequest {
            target,
            inflictor: Some(id),
            source: Some(id),
            amount,
        },
    )
    .expect("profile16 complete preflight proves current body damage closure");
    assert_eq!(damage.spawn_count(), 0);
    MeleePlan {
        owner: id,
        initial,
        accepted: None,
        damage: Some((amount, damage)),
    }
}

#[cfg(test)]
#[path = "saw_bite_tests.rs"]
mod saw_bite_tests;
