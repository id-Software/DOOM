//! Original former humans and bounded multi-monster encounters in the shared
//! player world. Every service (actors, thinkers, sight, pursuit, attacks,
//! damage, pickups, statistics and transactions) is the existing owner.
use crate::*;

/// Explicit original multi-monster execution context; historical contexts stay frozen.
pub const PLAYER_MONSTERS_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-1-MEDIUM-GROUNDED-v1";

/// Bounded mixed roster of autonomous original monsters (Demon and former
/// humans). With one player, at most 16 world items, eight drops and bounded
/// impacts this stays well inside the 64 actor slots and per-tic sight budget.
pub const MAX_PLAYER_MONSTERS: usize = 8;

/// Explicit original Imp/fireball execution context; older contexts stay frozen.
pub const PLAYER_IMPS_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-IMPS-1-MEDIUM-GROUNDED-v1";

/// Explicit real-map-scale execution context; older contexts stay frozen.
pub const PLAYER_MAP_SCALE_CONTEXT_SCHEMA: &[u8] =
    b"PLAYER-MONSTERS-MAP-SCALE-1-MEDIUM-GROUNDED-v1";

/// Explicit static-heights execution context; older contexts stay frozen.
pub const PLAYER_HEIGHTS_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-HEIGHTS-1-MEDIUM-v1";

/// Explicit diagonal-wall execution context; older contexts stay frozen.
pub const PLAYER_DIAGONAL_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-DIAGONAL-1-MEDIUM-v1";
/// Explicit real-map-scale execution context; older contexts stay frozen.
pub const PLAYER_REAL_MAP_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-REALMAP-1-MEDIUM-v1";
/// Explicit sky and static-specials execution context; older contexts stay frozen.
pub const PLAYER_SPECIALS_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-SPECIALS-1-MEDIUM-v1";
/// Explicit doors, lifts, floors, switches and exit context; older contexts stay frozen.
pub const PLAYER_MOVERS_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1";
/// Explicit E1M1 context (two-sided scrollers, static decorations); older contexts stay frozen.
pub const PLAYER_E1M1_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-E1M1-1-MEDIUM-v1";
/// Explicit barrels context (64 pickups, 384 slots, every angle, spawn rule S′);
/// older contexts stay frozen.
pub const PLAYER_BARRELS_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-BARRELS-1-MEDIUM-v1";
/// Explicit level context (the barrels world plus the body at its THINGS record,
/// the `damagecount` decay and `P_DeathThink`); older contexts stay frozen.
pub const PLAYER_LEVEL_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-LEVEL-1-MEDIUM-v1";
/// Explicit Episode 1 context (the level world with per-map bounds, 1,024 actor
/// slots and the H3 per-tic witnesses, M4a); older contexts stay frozen.
pub const PLAYER_EPISODE1_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1";
/// Explicit Episode 1 part 2 context (M4b, wire 24, DSIM40): no skill word,
/// because the skill is the canonical `actors.options` field; older contexts stay
/// frozen.
pub const PLAYER_EPISODE1_V2_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-EPISODE1-2-v1";
/// The Episode 1 map numbers the wire-24 constructor admits (`E1M1`–`E1M9`).
pub const EPISODE1_MAP_NUMBERS: std::ops::RangeInclusive<u8> = 1..=9;

impl Simulation {
    /// Autonomous roster member: the native/original Demon owner or a policy
    /// monster (former humans, and under `PlayerMonstersImpV1` the Imp).
    pub(crate) fn is_roster_monster(&self, ty: ActorTypeId) -> bool {
        ty == self.demon_owner_type()
            || original_registry::is_policy_monster(self.player_policy(), ty)
    }
    /// Bonus world plus original MT_POSSESSED/MT_SHOTGUY/MT_CHAINGUY, a bounded
    /// mixed roster, source infighting and P_KillMobj item drops.
    pub fn original_player_monsters_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            limit,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// Former-human world plus the original Imp, its fireball and the source
    /// same-species missile rule.
    pub fn original_player_imps_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            limit,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersImpV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// The Imp world inside a walled rectangle of up to 2,048 units: travel bounds
    /// derived from the loaded walls, `MAP_SCALE_ACTOR_SLOTS` slots and
    /// `MAP_SCALE_MAX_LIVE_PROJECTILES` live fireballs. Geometry outside the proven
    /// domain is rejected here, never during play.
    pub fn original_player_map_scale_map(
        map: Map,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            crate::player_map_scale::MAP_SCALE_ACTOR_SLOTS,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersMapScaleV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// The map-scale world off flat floors: axis-aligned sectors with static
    /// floor/ceiling heights joined by two-sided lines inside the walled rectangle,
    /// steps, ledges, windows, fences, falls and landings. Moving sectors, line and
    /// sector specials and sky ceilings are rejected here, never during play.
    pub fn original_player_heights_map(
        map: Map,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            crate::player_map_scale::MAP_SCALE_ACTOR_SLOTS,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersHeightsV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// The heights world with linedefs of any direction: one- and two-sided diagonal
    /// walls inside a closed boundary of one-sided walls facing their rooms, wall
    /// slides at any angle, diagonal risers, windows, pillars and corridors. The
    /// boundary and BLOCKMAP rules of the M3h5 note are checked here, never during
    /// play; the one refused legal source move is the corner jump
    /// (`MapScaleError::Escape`).
    pub fn original_player_diagonal_map(
        map: Map,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            crate::player_map_scale::MAP_SCALE_ACTOR_SLOTS,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersDiagonalV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// The diagonal world at the scale of classic maps: static-geometry caps taken
    /// from the WAD format (32,767 of every record kind), the arithmetic
    /// certificate and the slide-window rule of the M3h6 note checked here, player
    /// starts kept as records, and the 129th intercept of one trace refused during
    /// play (`MapScaleError::Intercepts`). Roster, slots, fireballs, heights,
    /// specials and sky keep the diagonal policy's rules.
    pub fn original_player_real_map_map(
        map: Map,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            crate::player_map_scale::MAP_SCALE_ACTOR_SLOTS,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersRealMapV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// The real-map world plus sky ceilings and the static sector specials (M3h7
    /// note §4.2, wire 18, DSIM34): sector specials 1, 2, 3, 4, 5, 7, 8, 9, 12, 13,
    /// 16 and 17, line special 48 on at most 64 one-sided lines, and `F_SKY1`
    /// ceilings are admitted; every other special, a two-sided 48, sector
    /// specials 10, 11 and 14 and `BT_USE` stay rejected. Roster, slots,
    /// fireballs, certificates, boundary rules, witnesses and the checked
    /// transaction are the real-map policy's. After `spawn_map_things` and
    /// `spawn_player_body`, call [`Simulation::spawn_level_specials`] once
    /// before the first tic.
    pub fn original_player_specials_map(
        map: Map,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            crate::player_map_scale::MAP_SCALE_ACTOR_SLOTS,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersSpecialsV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// The specials world plus doors, lifts, floors, switches and the exit (M3h8
    /// note §4.0, wire 19): line specials 1, 63, 109, 88, 36, 38 and 11 under the
    /// mover load rules of §7 (`validate_movers`), on top of everything
    /// [`Simulation::original_player_specials_map`] admits; a two-sided scrolling
    /// wall stays rejected (decision N1). The sector-thinker slot class holds the
    /// light sectors plus twice the mover sectors. As for the specials policy,
    /// call [`Simulation::spawn_level_specials`] once before the first tic.
    pub fn original_player_movers_map(
        map: Map,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            crate::player_map_scale::MAP_SCALE_ACTOR_SLOTS,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersMoversV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// The movers world plus the E1M1 admissions of the M3h9 note §5.1 (wire 20):
    /// line special 48 on two-sided lines, blocking or not (decision E1; the three
    /// game-code effects of a 48 line, `spechit` entry in `PIT_CheckLine`
    /// p_map.c:240–244, the monster `DI_NODIR` reset with `P_UseSpecialLine` false
    /// p_enemy.c:310–324 and the `PTR_UseTraverse` stop p_map.c:1113–1122, are the
    /// M3h8 bodies), and the seven static decoration types of info.c (decision E2),
    /// on top of everything [`Simulation::original_player_movers_map`] admits.
    /// Captures are DSIM36. As for the specials policy, call
    /// [`Simulation::spawn_level_specials`] once before the first tic.
    pub fn original_player_e1m1_map(
        map: Map,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            crate::player_map_scale::MAP_SCALE_ACTOR_SLOTS,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersE1M1V1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// The E1M1 world under the bounds and rules of the M3h9b note §5.1 (wire 21):
    /// up to 64 pickups and 384 actor slots (decision F2), every THING angle
    /// (decision F8) and the spawn rule S′ (decision F3), on top of everything
    /// [`Simulation::original_player_e1m1_map`] admits. Captures are DSIM37, the
    /// DSIM36 layout under policy 21 (decision F9). As for the specials policy,
    /// call [`Simulation::spawn_level_specials`] once before the first tic.
    pub fn original_player_barrels_map(
        map: Map,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let policy = OriginalExecutionProfile::PlayerMonstersBarrelsV1;
        Self::with_original_registry(
            map,
            crate::player_map_scale::actor_slots(policy),
            OriginalExecutionRegistry::new(policy)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// The barrels world (wire 21) plus the three behaviours of decision G4
    /// (M3h9c, wire 22): [`Simulation::spawn_map_things`] spawns the player body
    /// when it meets the type-1 THINGS record, in lump order, with that record's
    /// angle (`P_LoadThings`, p_setup.c:313–346; `P_SpawnMapThing`,
    /// p_mobj.c:729–736; `P_SpawnPlayer`, 642–686), and a level without exactly
    /// one type-1 record is refused; the player's `damagecount` decays by one per
    /// live tic (p_user.c:355–356); a dead player runs `P_DeathThink`
    /// (p_user.c:182–228), turning toward its attacker, and accepts any admitted
    /// command, of which only `BT_USE` is read. [`Simulation::spawn_player_body`]
    /// is refused. Captures are DSIM38, the DSIM37 layout under policy 22. As for
    /// the barrels policy, call [`Simulation::initialize_player_weapons`] and
    /// [`Simulation::spawn_level_specials`] once before the first tic.
    pub fn original_player_level_map(
        map: Map,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let policy = OriginalExecutionProfile::PlayerMonstersLevelV1;
        Self::with_original_registry(
            map,
            crate::player_map_scale::actor_slots(policy),
            OriginalExecutionRegistry::new(policy)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// Episode 1, part 1 (M4a, wire 23): the level world of
    /// [`Simulation::original_player_level_map`] with the bounds of decision H4
    /// derived from the map's own THINGS lump (monsters, pickups, drops,
    /// decorations and barrels, counted with the `P_SpawnMapThing` skill and mode
    /// filter) and [`crate::EPISODE1_ACTOR_SLOTS`] actor slots, the store-full
    /// refusal (`StoreError::Capacity`) standing in for the older policies'
    /// "the store never fills" proof; and the load proofs of decision H3 that
    /// refuse Episode 1 maps replaced by per-tic witnesses: the arithmetic
    /// certificate keeps only its clauses (iii)–(vi) and (viii), clauses (i), (ii)
    /// and (vii) and the ±512 height rule becoming checked arithmetic on the
    /// traced intercepts and heights; the one-sided boundary is optional; the
    /// mover rules are those of `mover_rules::validate_episode1`. Captures are
    /// DSIM39. As for the level policy, call [`Simulation::spawn_map_things`],
    /// [`Simulation::initialize_player_weapons`] and
    /// [`Simulation::spawn_level_specials`] once before the first tic.
    pub fn original_player_episode1_map(
        map: Map,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let policy = OriginalExecutionProfile::PlayerMonstersEpisode1V1;
        Self::with_original_registry(
            map,
            crate::player_map_scale::actor_slots(policy),
            OriginalExecutionRegistry::new(policy)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// Episode 1, part 2 (M4b, wire 24, decision I2): the world of
    /// [`Simulation::original_player_episode1_map`] for the map `E1M<map_number>`
    /// at `skill`, plus what M4b admits. The map number (1–9,
    /// [`EPISODE1_MAP_NUMBERS`]) is held for the run and captured as DSIM40
    /// `level.map`; the skill is `actors.options[0]`. Captures are DSIM40, read by
    /// [`crate::Diagnostic::decode_episode1_v2`].
    ///
    /// Admitted so far: teleportation (line special 97, the landing doomednum 14,
    /// the fog and the telefrag; M4b step 4), the Baron of Hell (doomednum 3003,
    /// `A_BruisAttack` and its fireball) and `A_BossDeath` with the E1M8 tag-666
    /// floor (step 5), and hard skill (step 6, decision I4): `skill` is
    /// [`Skill::Medium`] or [`Skill::Hard`], and the per-map bounds, the landing
    /// count and the item total's cross-check count the THINGs with that skill's
    /// option bit (2 or 4, p_mobj.c:743–751), as the spawner does. The per-map
    /// bounds count the Baron as a monster. Refused at load, each explicitly: a map
    /// number outside 1–9 and any skill but medium or hard
    /// (`SimError::PlayerBody(PlayerBodyError::Unsupported)`, as every older
    /// player policy refuses non-default options). As for
    /// wire 23, call [`Simulation::spawn_map_things`],
    /// [`Simulation::initialize_player_weapons`] and
    /// [`Simulation::spawn_level_specials`] once before the first tic.
    pub fn original_player_episode1_v2_map(
        map: Map,
        map_number: u8,
        skill: Skill,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let policy = OriginalExecutionProfile::PlayerMonstersEpisode1V2;
        if !EPISODE1_MAP_NUMBERS.contains(&map_number) {
            return Err(SimError::from(PlayerBodyError::Unsupported).into());
        }
        let mut sim = Self::with_original_registry(
            map,
            crate::player_map_scale::actor_slots(policy),
            OriginalExecutionRegistry::new(policy)?,
            ActorOptions {
                skill,
                ..ActorOptions::default()
            },
            context,
        )?;
        sim.level_map = Some(map_number);
        sim.world.map_number = Some(map_number);
        Ok(sim)
    }
}

use crate::lifecycle::{ActorAccess, LifeHost};

/// p_local.h MISSILERANGE (32*64 map units).
const MISSILE_RANGE_UNITS: i16 = 2048;
/// Shotgunner pellets per A_SPosAttack (p_enemy.c loop bound).
const SHOTGUNNER_PELLETS: u8 = 3;

/// Original former-human hitscan callbacks sharing one implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Hitscan {
    /// A_PosAttack: face, aim, pistol sound, one bullet.
    Pistol,
    /// A_SPosAttack: shotgun sound, face, aim, three complete bullets.
    Shotgun,
    /// A_CPosAttack: shotgun sound, face, aim, one bullet.
    Chaingun,
}

pub(crate) fn uses_current_monsters<A: ActorAccess>(h: &LifeHost<'_, A>) -> bool {
    h.player.as_ref().is_some_and(|p| p.policy.has_monsters())
}

/// Current-state original attack inside the bounded checked transaction. Any
/// late error is latched and discards the whole staged tic.
pub(crate) fn attack<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId, kind: Hitscan) {
    if let Err(error) = attack_checked(h, id, kind) {
        // Keep the first latched cause; the staged tic is discarded either way.
        if !h.sight_failed() {
            h.fail_sight(error);
        }
    }
}

fn readable_target<A: ActorAccess>(
    h: &LifeHost<'_, A>,
    id: ActorId,
) -> Result<Option<ActorId>, SimError> {
    let actor = h.access.get(id).ok_or(CombatError::Reference)?;
    let target = actor.life().ok_or(MonsterError::Definition)?.target;
    if let Some(t) = target {
        // Original targets stay allocated in this world (no corpse removal);
        // an unreadable handle is outside the admitted domain.
        h.access.get(t).ok_or(MonsterError::Target)?;
    }
    Ok(target)
}

fn attack_checked<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    kind: Hitscan,
) -> Result<(), SimError> {
    if readable_target(h, id)?.is_none() {
        return Ok(());
    }
    if kind == Hitscan::Shotgun {
        h.events
            .push(Event::Shotgun(ShotgunEvent::Begin { actor: id }));
    }
    let sound = |name| lifecycle::source_sound(name).ok_or(MonsterError::Definition);
    if kind != Hitscan::Pistol {
        h.sound(Some(id), sound("sfx_shotgn")?);
    }
    crate::monsters::face(h, id);
    let range = Fixed::from_map_units(MISSILE_RANGE_UNITS);
    let angle = h.access.get(id).ok_or(CombatError::Reference)?.angle();
    let aim = {
        let lookup = |i| h.access.get(i).copied();
        let trace = combat_trace::collect_in(h.world, h.spatial, &lookup, id, angle, range)?;
        combat_trace::aim_trace(h.world, h.sectors, id, trace, range, &lookup)?
    };
    let slope = aim.slope;
    h.events.push(Event::Monster(MonsterEvent::Aim {
        actor: id,
        result: Box::new(aim),
    }));
    if kind == Hitscan::Pistol {
        h.sound(Some(id), sound("sfx_pistol")?);
    }
    let pellets = if kind == Hitscan::Shotgun {
        SHOTGUNNER_PELLETS
    } else {
        1
    };
    for index in 1..=pellets {
        if kind == Hitscan::Shotgun {
            h.events.push(Event::Shotgun(ShotgunEvent::PelletStart {
                actor: id,
                index,
            }));
        }
        let (shot, damage) = weapons::monster_shot(angle, &mut |k| crate::monsters::draw(h, id, k));
        if kind == Hitscan::Shotgun {
            h.events.push(Event::Shotgun(ShotgunEvent::Pellet {
                actor: id,
                index,
                angle: shot,
                slope,
                damage,
            }));
        }
        let result = bullet(h, id, shot, slope, damage)?;
        if kind == Hitscan::Shotgun {
            h.events.push(Event::Shotgun(ShotgunEvent::PelletDone {
                actor: id,
                index,
                rng: h.rng.indices()[0],
                effect: result.effect,
            }));
        }
    }
    if kind == Hitscan::Shotgun {
        h.events.push(Event::Shotgun(ShotgunEvent::Completed {
            actor: id,
            rng: h.rng.indices()[0],
        }));
    }
    Ok(())
}

/// One complete current P_LineAttack: collect, hit, plan current damage, then
/// the shared impact/damage executor. Earlier pellets' effects are visible.
fn bullet<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    angle: u32,
    slope: Fixed,
    damage: i32,
) -> Result<AttackResult, SimError> {
    let attack = LineAttack {
        angle,
        range: Fixed::from_map_units(MISSILE_RANGE_UNITS),
        slope,
        damage,
    };
    let (trace, ((hit, position, traversed), sky)) = {
        let lookup = |i| h.access.get(i).copied();
        let trace = combat_trace::collect_in(h.world, h.spatial, &lookup, id, angle, attack.range)?;
        let hit = combat_trace::hit_trace_sky(
            h.world,
            h.sectors,
            id,
            &trace,
            attack.range,
            slope,
            &lookup,
        )?;
        (trace, hit)
    };
    // P_ShootSpecialLine for the special lines the trace visited (Episode 1).
    crate::use_lines::shoot_lines(h, false, &trace, traversed)?;
    let plan = match hit {
        Some(TraceObject::Actor(target)) if damage != 0 => {
            Some(combat::DamagePlan::current_original(
                h,
                DamageRequest {
                    target,
                    inflictor: Some(id),
                    source: Some(id),
                    amount: damage,
                },
            )?)
        }
        _ => None,
    };
    let prepared = combat::PreparedAttack::current(
        id,
        attack,
        AttackResult {
            trace,
            hit,
            position,
            traversed,
            effect: None,
            damage: None,
        },
        plan,
    )
    .with_sky(sky);
    h.access.reserve(prepared.spawn_count())?;
    let result = combat::execute_attack(h, prepared);
    if h.sight_failed() {
        return Err(PlayerBodyError::Unsupported.into());
    }
    Ok(result)
}

/// p_enemy.c A_TroopAttack: `(P_Random()%8+1)*3`, in the MonsterEvent draw namespace.
const IMP_DAMAGE_CHOICES: i32 = 8;
const IMP_DAMAGE_UNIT: i32 = 3;
const IMP_DAMAGE_DRAW: u8 = 4;

/// A_TroopAttack: the single attack state chooses melee or missile from current
/// state. Any late error is latched and discards the whole staged tic.
pub(crate) fn troop_attack<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) {
    if let Err(error) = troop_attack_checked(h, id) {
        if !h.sight_failed() {
            h.fail_sight(error);
        }
    }
}
fn troop_attack_checked<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
) -> Result<(), SimError> {
    let Some(target) = readable_target(h, id)? else {
        return Ok(());
    };
    crate::monsters::face(h, id);
    if crate::monsters::melee_host(h, id) {
        let claw = lifecycle::source_sound("sfx_claw").ok_or(MonsterError::Definition)?;
        h.sound(Some(id), claw);
        let draw = crate::monsters::draw(h, id, IMP_DAMAGE_DRAW);
        let amount = (i32::from(draw) % IMP_DAMAGE_CHOICES + 1) * IMP_DAMAGE_UNIT;
        let plan = combat::DamagePlan::current_original(
            h,
            DamageRequest {
                target,
                inflictor: Some(id),
                source: Some(id),
                amount,
            },
        )?;
        h.access.reserve(plan.spawn_count())?;
        h.apply_damage(plan);
        return Ok(());
    }
    if h.sight_failed() {
        return Ok(());
    }
    projectiles::launch_fireball(h, id, target)
}

/// p_enemy.c A_BruisAttack: `(P_Random()%8+1)*10` (p_enemy.c:989), in the
/// MonsterEvent draw namespace of the Imp's claw.
const BARON_DAMAGE_CHOICES: i32 = 8;
const BARON_DAMAGE_UNIT: i32 = 10;

/// `A_BruisAttack` (p_enemy.c:979–996, wire 24): the Baron's `S_BOSS_ATK3`. Unlike
/// `A_TroopAttack` it does not call `A_FaceTarget` (M4b note §3 fact 4); the
/// claw-or-fireball choice is made again here from the current state. Any late
/// error is latched and discards the whole staged tic.
pub(crate) fn bruis_attack<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) {
    if let Err(error) = bruis_attack_checked(h, id) {
        if !h.sight_failed() {
            h.fail_sight(error);
        }
    }
}
fn bruis_attack_checked<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
) -> Result<(), SimError> {
    if !h.player.as_deref().is_some_and(|p| p.policy.has_baron()) {
        return Err(PlayerBodyError::Unsupported.into());
    }
    // `if (!actor->target) return;` (983–984).
    let Some(target) = readable_target(h, id)? else {
        return Ok(());
    };
    // `P_CheckMeleeRange` (986; p_enemy.c:174–191): no draw.
    if crate::monsters::melee_host(h, id) {
        let claw = lifecycle::source_sound("sfx_claw").ok_or(MonsterError::Definition)?;
        h.sound(Some(id), claw);
        let draw = crate::monsters::draw(h, id, IMP_DAMAGE_DRAW);
        let amount = (i32::from(draw) % BARON_DAMAGE_CHOICES + 1) * BARON_DAMAGE_UNIT;
        // `P_DamageMobj (actor->target, actor, actor, damage)` (990).
        let plan = combat::DamagePlan::current_original(
            h,
            DamageRequest {
                target,
                inflictor: Some(id),
                source: Some(id),
                amount,
            },
        )?;
        h.access.reserve(plan.spawn_count())?;
        h.apply_damage(plan);
        return Ok(());
    }
    if h.sight_failed() {
        return Ok(());
    }
    // `P_SpawnMissile (actor, actor->target, MT_BRUISERSHOT)` (995).
    projectiles::launch_missile(
        h,
        id,
        target,
        ActorTypeId(crate::identity::OriginalActorType::MT_BRUISERSHOT.ordinal()),
    )
}

/// A_CPosRefire with the checked expanded sight query; decision order is the
/// shared source short-circuit in `chaingunner::decide`.
pub(crate) fn refire<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId, work: &mut usize) {
    if let Err(error) = refire_checked(h, id, work) {
        // Keep the first latched cause; the staged tic is discarded either way.
        if !h.sight_failed() {
            h.fail_sight(error);
        }
    }
}
fn refire_checked<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    work: &mut usize,
) -> Result<(), SimError> {
    let target = readable_target(h, id)?;
    crate::monsters::face(h, id);
    let roll = crate::monsters::draw(h, id, 6);
    let a = *h.access.get(id).ok_or(CombatError::Reference)?;
    let checked = h.perception.and_then(|p| p.checked);
    let mut failure = None;
    let (decision, sight) = crate::chaingunner::decide(
        roll,
        || target.and_then(|t| h.access.get(t).and_then(Actor::life).map(|l| l.health)),
        || {
            let t = *h
                .access
                .get(target.expect("health branch proved target"))
                .unwrap();
            crate::demon_expanded_map::action_sight(checked, h.world, h.sectors, a, t)
                .unwrap_or_else(|error| {
                    failure = Some(error);
                    SightResult {
                        visible: false,
                        reject_index: 0,
                        rejected: true,
                        eye: None,
                        slopes: None,
                        visits: Vec::new(),
                    }
                })
        },
    );
    if let Some(error) = failure {
        return Err(error);
    }
    if let Some(result) = sight {
        h.events.push(Event::Perception(PerceptionEvent::Sight {
            observer: id,
            target: target.ok_or(MonsterError::Target)?,
            result,
        }));
    }
    h.events.push(Event::Refire(RefireEvent {
        actor: id,
        decision,
    }));
    if decision.exits() {
        let see = h
            .registry
            .actor_type(a.life().ok_or(MonsterError::Definition)?.actor_type)
            .ok_or(MonsterError::Definition)?
            .states()[1];
        h.enter_counted(id, see, work);
    }
    Ok(())
}
