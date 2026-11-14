//! Deterministic, headless Linux DOOM simulation foundations.
//!
//! Each unpaused [`Simulation::advance`] is one logical 35 Hz tic, never elapsed
//! wall time. [`Simulation::step`] adds diagnostic Snapshot capture.
//! Explicit profiles preserve M2 foundations and M3a grounded player collision,
//! and add M3b static-sector BSP/openings and XY/Z/gravity. None implements full
//! player thinking or establishes complete vanilla gameplay synchronization.
//! Profile 4 adds admitted source actor states, spawning and controlled death.
//! Profile 5 adds bounded unbound-actor damage, autoaim and hitscan combat.
//! Profile 6 adds command-driven player weapons, ammo and source psprites.
//! Profile 7 adds explicit original plasma launch, flight, direct impact and removal.
//! Profile 8 adds source sight, player-slot scanning, noise awareness and admitted A_Look.
//! Profile 9 adds non-attacking native fixture pursuit through source direction decisions.
//! Profile 10 adds native attack selection/facing and bounded PosAttack through shared combat.
//! Profile 11 adds exact SPOS timing and three sequential SPosAttack pellets through shared combat.
//! Profile 12 adds separately aimed CPOS shots and immediate source refire/Chase handoffs.

#[cfg(test)]
extern crate self as doom_sim;

mod actions;
mod actor;
mod direction;
pub use direction::MoveDirection;
mod canonical_demon;
mod diagnostic;
mod execution_context;
mod game_instance;
mod native_demon;
mod original_player;
mod original_registry;
mod player_armor;
mod player_bonuses;
mod player_map_scale;
mod player_monsters;
pub use mover_rules::{MoverBounds, e1m1_mover_bounds, mover_bounds, special_window_lines};
pub use player_armor::{
    ArmorEvent, ArmorType, PLAYER_DEMON_ARMOR_CONTEXT_SCHEMA, PlayerArmorState,
};
pub use player_bonuses::PLAYER_DEMON_BONUSES_CONTEXT_SCHEMA;
pub use player_map_scale::{
    ArithmeticCertificate, BARRELS_ACTOR_SLOTS, Boundary, EPISODE1_ACTOR_SLOTS, HEIGHTS_LINE_FLAGS,
    HEIGHTS_MOMZ_UNITS, HEIGHTS_Z_UNITS, MAP_SCALE_ACTOR_SLOTS, MAP_SCALE_EXTENT_UNITS,
    MAP_SCALE_MAX_LIVE_PROJECTILES, MAP_SCALE_MOMENTUM_UNITS, MAP_SCALE_WALL_LINES, MAX_INTERCEPTS,
    MapScaleError, REAL_MAP_FORMAT_RECORDS, RoomBounds, actor_slots, slide_window_lines,
};
pub use player_monsters::{
    EPISODE1_MAP_NUMBERS, MAX_PLAYER_MONSTERS, PLAYER_BARRELS_CONTEXT_SCHEMA,
    PLAYER_DIAGONAL_CONTEXT_SCHEMA, PLAYER_E1M1_CONTEXT_SCHEMA, PLAYER_EPISODE1_CONTEXT_SCHEMA,
    PLAYER_EPISODE1_V2_CONTEXT_SCHEMA, PLAYER_HEIGHTS_CONTEXT_SCHEMA, PLAYER_IMPS_CONTEXT_SCHEMA,
    PLAYER_LEVEL_CONTEXT_SCHEMA, PLAYER_MAP_SCALE_CONTEXT_SCHEMA, PLAYER_MONSTERS_CONTEXT_SCHEMA,
    PLAYER_MOVERS_CONTEXT_SCHEMA, PLAYER_REAL_MAP_CONTEXT_SCHEMA, PLAYER_SPECIALS_CONTEXT_SCHEMA,
};
mod player_commands;
mod player_pistol;
mod player_roster;
mod player_weapons;
mod world_pickups;
pub use diagnostic::{Diagnostic, DiagnosticError, DiagnosticIdentity, DiagnosticLimits};
pub use execution_context::{ContextActor, ContextError, ExecutionContext, TargetReference};
pub use game_instance::{GameInstance, InstanceActorRef, InstanceId, ProvenanceError};
pub use native_demon::{NativeDemonState, NativeDemonType};
pub use original_player::{
    OriginalPlayer, PLAYER_BODY_CONTEXT_SCHEMA, PLAYER_KNOCKBACK_CONTEXT_SCHEMA, PlayerBodyError,
    PlayerBodyEvent, PlayerBodyState, PlayerPowers, PlayerSlot,
};
pub use original_registry::{
    ContextRole, DefinitionIdentity, ORIGINAL_CONTEXT_SCHEMA, ORIGINAL_REGISTRY_SCHEMA,
    OriginalExecutionProfile, OriginalExecutionRegistry, OriginalRegistryError,
};
pub use player_commands::PLAYER_COMMAND_CONTEXT_SCHEMA;
pub use player_pistol::{INITIAL_PISTOL_BULLETS, PLAYER_DEMON_PISTOL_CONTEXT_SCHEMA, PistolState};
pub use player_roster::{PLAYER_DEMON_CONTEXT_SCHEMA, PLAYER_DEMON_MELEE_CONTEXT_SCHEMA};
pub use player_weapons::{PLAYER_DEMON_WEAPONS_CONTEXT_SCHEMA, PlayerWeaponState};
pub use world_pickups::{
    BARRELS_MAX_PICKUPS, MAX_WORLD_PICKUPS, PLAYER_DEMON_PICKUPS_CONTEXT_SCHEMA, PickupEvent,
    PickupKind, PickupMessage, PickupOutcome, PlayerPickupState,
};
mod chaingunner;
mod demon;
pub use canonical_demon::{CanonicalDemonError, DemonExecutionBinding};
mod demon_expanded_map;
mod demon_lifecycle;
mod demon_map_context;
mod demon_map_spawn;
pub use demon_map_context::{LevelStatistics, MapContextOptions, StatisticsEvent};
mod demon_stand_run;
pub use demon_map_spawn::{MAX_DEMON_MAP_THINGS, MAX_MAP_DEMONS, MapSpawnError};
mod monsters;
mod shotgunner;
pub use chaingunner::{RefireDecision, RefireEvent};
pub use shotgunner::ShotgunEvent;
mod perception;
mod pursuit;
pub use monsters::{MonsterError, MonsterEvent, RangeDecision};
pub use pursuit::{PursuitError, PursuitEvent, PursuitOperation, PursuitSetup};
mod sight;
pub use perception::{PerceptionError, PerceptionEvent, PerceptionPlayer};
pub use sight::{SightResult, SightVisit};
mod projectiles;
pub use projectiles::{
    MAX_LIVE_PROJECTILES, ProjectileAim, ProjectileError, ProjectileEvent, ProjectileRequest,
};
mod weapons;
pub use weapons::{
    BT_ATTACK, BT_CHANGE, BT_USE, BT_WEAPONMASK, Psprite, Weapon, WeaponError, WeaponEvent,
    WeaponInventory, WeaponPlayer,
};
mod combat;
mod combat_trace;
pub use combat::{AttackResult, CombatError, CombatEvent, DamageRequest, DamageResult, LineAttack};
pub use combat_trace::{AimResult, HitscanTrace, Intercept, TraceObject};
/// Canonical original metadata identities, distinct from executable registry slots.
pub mod identity;
mod lifecycle;
mod spawning;
pub use lifecycle::{
    ActionId, ActorAction, ActorError, ActorGameMode, ActorLife, ActorOptions, ActorRegistry,
    ActorStateDef, ActorStateEvent, ActorTypeDef, ActorTypeId, ProbeAction, Skill, StateId,
};
pub use spawning::MapSpawnReport;
mod boss;
mod geometry;
mod line_triggers;
mod line_work;
mod map_extent;
mod map_scale;
mod teleport;
mod use_lines;
pub use map_extent::{
    MAP_EXTENT_COLLISION_WORK, MAP_EXTENT_MAX_CELLS, MAP_EXTENT_PREPARATION_BYTES,
    MAP_EXTENT_SIGHT_WORK, MapExtentLimits,
};
pub use map_scale::{
    MAP_SCALE_MAX_LINES, MAP_SCALE_MAX_REFERENCES, MAP_SCALE_MAX_SECTORS, MapScaleLimits,
    MapScaleMetrics,
};
mod barrels;
pub use barrels::EXPLOSION_SIGHT_QUERIES;
pub mod demo;
mod e1m1_rules;
mod episode1;
pub use episode1::Episode1Bounds;
mod level;
mod level_stats;
pub use e1m1_rules::{
    BARRELS_LINKED_THINGS, E1M1_DECORATIONS, E1M1_LINKED_THINGS, MAX_BARRELS, MAX_DECORATIONS,
};
pub use level_stats::{LevelRunStats, take_level_run_stats};
mod movement;
mod mover_rules;
mod movers;
mod multisector;
mod noise;
mod planes;
pub use movement::{ActorMoveAttempt, Solidity};
pub use multisector::{
    Gravity, Location, MoveFailure, Opening, OpeningVisit, SectorMoveAttempt, SectorState,
    SpawnHeight, VerticalEvent, VerticalSpec,
};
mod sector_thinkers;
mod snapshot;
mod staging;
mod store;
mod thinkers;
include!("fine_sine.rs");

pub use actor::{Actor, ActorSpec, TicCommand};
use doom_map::Map;
use doom_types::{DoomRng, Fixed};
pub use geometry::{Collision, GeometryError, LineGeometry, MoveAttempt, StaticWorld};
pub use movers::{Door, DoorKind, FloorKind, FloorMove, Plat, PlatStatus, PlatType};
pub use planes::{ClipOutcome, MoveResult, MoverEvent, MoverFunction, Plane, SoundOrigin};
pub use sector_thinkers::{
    FireFlicker, Glow, LightFlash, SectorThinker, SpecialEvent, StrobeFlash,
};
pub use snapshot::{Divergence, Snapshot, StateField, compare_traces};
use std::{error::Error, fmt, sync::Arc};
pub use store::{
    ActorAvailability, ActorId, SectorThinkerId, StoreError, ThinkerContext, ThinkerId,
    ThinkerStore, Visit,
};

/// Explicit behavior/schema profile; M2 remains frozen for regression consumers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Profile {
    /// M2 non-solid non-player response, command foundations, DSIM-v1.
    Foundation,
    /// M3a grounded player sliding and zero/MF_SOLID actors, DSIM-v2.
    PlayerCollision,
    /// M3b static connected sectors, BSP/XY/Z/gravity and reference lists, DSIM-v3.
    MultiSector,
    /// M3c typed actor lifecycle on static-sector physics, DSIM-v4.
    ActorStates,
    /// M3d1 unbound actor damage/hitscan subset, DSIM-v5.
    Combat,
    /// M3d2 player weapons and psprites, DSIM-v6.
    PlayerWeapons,
    /// M3d2b bounded original plasma projectiles, DSIM-v7.
    Projectiles,
    /// M3d3a sight, player scanning and source awareness, DSIM-v8.
    Perception,
    /// M3d3b non-attacking monster pursuit, DSIM-v9.
    Pursuit,
    /// M3d3c bounded monster attack selection/actions, DSIM-v10.
    MonsterAttacks,
    /// M3d3d sequential native shotgunner attacks, DSIM-v11.
    Shotgunner,
    /// M3d3e separately aimed chaingunner shots and source refire, DSIM-v12.
    Chaingunner,
    /// M3d3f bounded normal-timing Demon melee fixtures, DSIM-v13.
    DemonMelee,
    /// M3d3g normal Demon pain/death/corpse fixtures, DSIM-v14.
    DemonLifecycle,
    /// M3d3h original normal Demon stand/run closure, DSIM-v15.
    DemonStandRun,
    /// M3d3i bounded classic THING Demon admission, DSIM-v16.
    DemonMapSpawn,
    /// M3d3j original map totals and single-player fallback credit, DSIM-v17.
    DemonMapContext,
    /// M3f1 expanded bounded unbound static geometry and read-only sight, DSIM-v18.
    MapScale,
    /// M3f2 checked expanded static extents and spatial work, DSIM-v19.
    MapExtent,
    /// M3f3 bounded expanded static maps with native Demon gameplay, DSIM-v20.
    DemonExpandedMap,
    /// Original-ID normal Demon closure and explicit context, DSIM-v21.
    OriginalDemon,
}

impl Profile {
    pub(crate) const fn uses_checked_demons(self) -> bool {
        matches!(self, Self::DemonExpandedMap | Self::OriginalDemon)
    }
    pub(crate) const fn uses_map_statistics(self) -> bool {
        matches!(
            self,
            Self::DemonMapContext | Self::DemonExpandedMap | Self::OriginalDemon
        )
    }
    pub(crate) const fn uses_expanded_geometry(self) -> bool {
        matches!(
            self,
            Self::MapScale | Self::MapExtent | Self::DemonExpandedMap | Self::OriginalDemon
        )
    }
    pub(crate) const fn uses_demon_map(self) -> bool {
        matches!(self, Self::DemonMapSpawn) || self.uses_map_statistics()
    }
    pub(crate) const fn uses_demon_stand_run(self) -> bool {
        matches!(self, Self::DemonStandRun) || self.uses_demon_map()
    }
    pub(crate) const fn uses_demon_lifecycle(self) -> bool {
        matches!(self, Self::DemonLifecycle) || self.uses_demon_stand_run()
    }
    pub(crate) const fn uses_demon(self) -> bool {
        matches!(self, Self::DemonMelee) || self.uses_demon_lifecycle()
    }
    pub(crate) const fn uses_monsters(self) -> bool {
        matches!(
            self,
            Self::MonsterAttacks | Self::Shotgunner | Self::Chaingunner
        ) || self.uses_demon()
    }
    pub(crate) const fn uses_perception(self) -> bool {
        matches!(self, Self::Perception | Self::Pursuit) || self.uses_monsters()
    }
    pub(crate) const fn uses_sectors(self) -> bool {
        self.uses_perception()
            || self.uses_expanded_geometry()
            || matches!(
                self,
                Self::MultiSector
                    | Self::ActorStates
                    | Self::Combat
                    | Self::PlayerWeapons
                    | Self::Projectiles
            )
    }
}

/// Deterministic stepping/setup failure. Unsupported gameplay is explicit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimError {
    /// Explicit original-player body ownership or behavior rejection.
    PlayerBody(PlayerBodyError),
    /// Actor storage/scheduling failure.
    Store(StoreError),
    /// Actor definition, action admission or bounded preflight failure.
    Actor(ActorError),
    /// Bounded combat validation failure, before mutation.
    Combat(CombatError),
    /// Weapon profile, command, admission or preflight failure.
    Weapon(WeaponError),
    /// Projectile admission, ownership or arithmetic failure.
    Projectile(ProjectileError),
    /// Perception profile, identity, metadata or bounded-work rejection.
    Perception(PerceptionError),
    /// Pursuit admission, target or arithmetic failure.
    Pursuit(PursuitError),
    /// Monster admission, ownership or attack preflight rejection.
    Monster(MonsterError),
    /// Bounded classic THING spawning or map-Demon admission failure.
    MapSpawn(MapSpawnError),
    /// Unsupported or invalid map/profile.
    Geometry(GeometryError),
    /// Real-map-scale room admission or wall-derived bound failure.
    MapScale(MapScaleError),
    /// Input slot is outside 0..4 or duplicates an existing binding.
    InvalidPlayer,
    /// Button behavior is not implemented in either profile.
    UnsupportedButtons,
    /// Logical u64 tic counter exhausted; no state is advanced.
    TicOverflow,
    /// Under the movers policy, the exit switch ran `G_ExitLevel` in an earlier tic:
    /// the next `G_Ticker` would run `G_DoCompleted` (intermission and next map are
    /// out of scope); no state is advanced.
    LevelExited,
    /// Under the movers policy, a dead player held `BT_USE` in an earlier tic
    /// (`PST_REBORN`): the next `G_Ticker` would run `G_DoReborn` (a level restart,
    /// out of scope, decision N3); no state is advanced.
    PlayerReborn,
}
impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "simulation: {self:?}")
    }
}
impl Error for SimError {}
impl From<MapSpawnError> for SimError {
    fn from(e: MapSpawnError) -> Self {
        Self::MapSpawn(e)
    }
}
impl From<MonsterError> for SimError {
    fn from(e: MonsterError) -> Self {
        Self::Monster(e)
    }
}
impl From<PursuitError> for SimError {
    fn from(e: PursuitError) -> Self {
        Self::Pursuit(e)
    }
}
impl From<PerceptionError> for SimError {
    fn from(e: PerceptionError) -> Self {
        Self::Perception(e)
    }
}
impl From<ProjectileError> for SimError {
    fn from(e: ProjectileError) -> Self {
        Self::Projectile(e)
    }
}
impl From<WeaponError> for SimError {
    fn from(e: WeaponError) -> Self {
        Self::Weapon(e)
    }
}
impl From<CombatError> for SimError {
    fn from(e: CombatError) -> Self {
        Self::Combat(e)
    }
}
impl From<ActorError> for SimError {
    fn from(e: ActorError) -> Self {
        Self::Actor(e)
    }
}
impl From<StoreError> for SimError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}
impl From<GeometryError> for SimError {
    fn from(e: GeometryError) -> Self {
        Self::Geometry(e)
    }
}

/// Visible phase order, from P_Ticker. Specials/respawn are phase markers only.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    /// Input slots in numerical order, before thinkers.
    Players,
    /// Append-sensitive thinker traversal.
    Thinkers,
    /// No map-special implementation in M2.
    Specials,
    /// No respawn implementation in M2.
    Respawn,
}

/// Deterministic observations. Presentation may read but never consume/mutate them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    /// Movers-policy plane, sector-change and clip observations (DSIM35 tag 26).
    Mover(MoverEvent),
    /// Specials-policy secret, sky-removal and sky-shot observations (DSIM34).
    Special(SpecialEvent),
    /// Original player armor absorption, DSIM26.
    Armor(ArmorEvent),
    /// Source world-item contact and resource-grant observations (DSIM25).
    Pickup(PickupEvent),
    /// Original-player field and presentation-service observations, DSIM-v22 and later.
    PlayerBody(PlayerBodyEvent),
    /// Ordered profile-11 shotgun boundaries and per-pellet observations.
    Shotgun(ShotgunEvent),
    /// Source profile-12 refire decision before immediate see-state entry.
    Refire(RefireEvent),
    /// Ordered profile-10 attack observations.
    Monster(MonsterEvent),
    /// Ordered original pursuit operations in profile 9.
    Pursuit(PursuitEvent),
    /// Source perception decisions in profile 8.
    Perception(PerceptionEvent),
    /// Ordered source projectile observations in profile 7.
    Projectile(ProjectileEvent),
    /// Ordered profile-6 player weapon observations.
    Weapon(WeaponEvent),
    /// Profile-5 combat evidence in source side-effect order.
    Combat(CombatEvent),
    /// Typed state/action/lifecycle evidence in execution order.
    ActorState(ActorStateEvent),
    /// Admitted source map totals or fallback death credit.
    Statistics(StatisticsEvent),
    /// Phase boundary.
    Phase(Phase),
    /// Actor appended, after its initial lastlook RNG draw.
    Spawned(ActorId),
    /// Logical removal mark, before physical retirement.
    RemovalMarked(ActorId),
    /// Storage retired and old generation invalidated.
    Retired(ActorId),
    /// Ground command applied in player-slot order.
    Player {
        /// Input slot.
        slot: u8,
        /// Controlled mover.
        actor: ActorId,
    },
    /// Actor update in linked order.
    Think(ActorId),
    /// One destination attempt, including candidate order and blocking result.
    Movement {
        /// Moving actor.
        actor: ActorId,
        /// Attempt details.
        attempt: MoveAttempt,
    },
    /// Explicit gameplay random request, separate from implicit spawn lastlook.
    GameplayRandom(u8),
    /// M3a things-before-lines destination attempt.
    ActorMovement {
        /// Moving actor.
        actor: ActorId,
        /// Ordered collision evidence.
        attempt: ActorMoveAttempt,
    },
    /// M3b destination and opening/clearance decision.
    SectorMovement {
        /// Moving actor.
        actor: ActorId,
        /// Ordered collision/height evidence.
        attempt: SectorMoveAttempt,
    },
    /// M3b Z update after this actor's XY movement.
    Vertical {
        /// Updated actor.
        actor: ActorId,
        /// Physical floor/ceiling/gravity observation.
        update: VerticalEvent,
    },
    /// Selected wall projection, in slide retry order.
    Slide {
        /// Sliding player.
        actor: ActorId,
        /// Selected linedef.
        line: u16,
    },
}

/// Ground thrust/angle part of P_MovePlayer, using exact original sine data.
/// The caller supplies grounded state. It does not implement player state changes,
/// airborne movement, weapon buttons or the player's blocked-move sliding response.
pub fn command_thrust(
    angle: u32,
    momentum: [Fixed; 2],
    command: TicCommand,
) -> Result<(u32, [Fixed; 2]), SimError> {
    if command.buttons != 0 {
        return Err(SimError::UnsupportedButtons);
    }
    // Explicit bit interpretation of the historical unsigned-shift adapter.
    // The literal original negative signed left shift has no ISO-C guarantee.
    let turn = u32::from_le_bytes(i32::from(command.angle_turn).to_le_bytes());
    let angle = angle.wrapping_add(turn.wrapping_shl(16));
    let mut momentum = momentum;
    for (heading, amount) in [
        (angle, command.forward),
        (angle.wrapping_sub(0x40000000), command.side),
    ] {
        if amount == 0 {
            continue;
        }
        let index = (heading >> 19) as usize;
        let force = Fixed::from_bits(i32::from(amount) * 2048);
        momentum[0] = momentum[0] + force * Fixed::from_bits(FINE_SINE[index + 2048]);
        momentum[1] = momentum[1] + force * Fixed::from_bits(FINE_SINE[index]);
    }
    Ok((angle, momentum))
}

/// Shared P_PlayerThink movement gate plus P_MovePlayer's angle/thrust portion.
/// Caller owns liveness, command/button policy and source state entry. Returns
/// whether P_MovePlayer executes, including when its directional input is zero.
pub(crate) fn apply_player_motion(
    actor: &mut Actor,
    command: TicCommand,
) -> Result<bool, SimError> {
    let life = actor.life.as_mut().ok_or(ActorError::InvalidState)?;
    if life.reaction_time != 0 {
        life.reaction_time = life
            .reaction_time
            .checked_sub(1)
            .ok_or(ActorError::InvalidState)?;
        return Ok(false);
    }
    // Buttons belong to the caller's separately admitted services. Existing
    // weapon execution supplies them; the canonical command policy rejects them.
    let physical = if actor.z > actor.floor {
        TicCommand {
            forward: 0,
            side: 0,
            buttons: 0,
            ..command
        }
    } else {
        TicCommand {
            buttons: 0,
            ..command
        }
    };
    let (angle, momentum) = command_thrust(actor.spec.angle, actor.spec.momentum, physical)?;
    actor.spec.angle = angle;
    actor.spec.momentum = momentum;
    Ok(true)
}

/// Owned deterministic state. Map data is immutable and all runtime state is explicit.
#[derive(Clone, Debug)]
pub struct Simulation {
    pub(crate) profile: Profile,
    pub(crate) perception_players: [PerceptionPlayer; 4],
    pub(crate) registry: Option<Arc<ActorRegistry>>,
    pub(crate) weapon_player: Option<WeaponPlayer>,
    pub(crate) original_player: Option<original_player::PlayerRuntime>,
    pub(crate) sound_targets: Vec<Option<ActorId>>,
    pub(crate) actor_options: ActorOptions,
    pub(crate) statistics: Option<LevelStatistics>,
    pub(crate) spatial: movement::SpatialIndex,
    pub(crate) sectors: Vec<SectorState>,
    pub(crate) world: StaticWorld,
    pub(crate) actors: ThinkerStore<Actor>,
    pub(crate) rng: DoomRng,
    pub(crate) tic: u64,
    pub(crate) paused: bool,
    pub(crate) bindings: [Option<ActorId>; 4],
    pub(crate) commands: [TicCommand; 4],
    pub(crate) events: Vec<Event>,
    pub(crate) pending: Vec<Event>,
    pub(crate) map_fields: Arc<Vec<StateField>>,
    /// Immutable definitions already validated against this registry and these
    /// static sectors, by type ordinal. A memo of a pure function, never state.
    pub(crate) admitted_types: demon_lifecycle::AdmittedTypes,
    /// Under the heights policy, the tallest static sector: type dimensions are
    /// admitted against it ("every type fits somewhere") and fitting is proved per
    /// spawn instead. Derived from the immutable map; never canonical state.
    pub(crate) type_envelope: Option<SectorState>,
    /// Under the diagonal policy, the one-sided walls and their parity test (the
    /// spawn rule and the escape witness). Derived from the immutable map; never
    /// canonical state.
    pub(crate) boundary: Option<Arc<player_map_scale::Boundary>>,
    /// Under the heights policies, the sound-target vector of the last committed
    /// plan, kept as the next plan's staging buffer so a tic allocates nothing
    /// for it. Never read; not canonical state, not encoded, not compared.
    pub(crate) spare_sound_targets: Vec<Option<ActorId>>,
    /// Under the specials policy, the runtime sector lights and specials and the
    /// load-operation phase (canonical state from DSIM34); `None` otherwise.
    pub(crate) specials: Option<sector_thinkers::LevelSpecials>,
    /// The specials vector of the last committed plan, reused as the next plan's
    /// staging buffer (like `spare_sound_targets`). Never read or encoded.
    pub(crate) spare_specials: Vec<sector_thinkers::SectorSpecial>,
    /// The sector-thinker slot vector of the last committed plan, reused as the next
    /// plan's staging buffer (like `spare_specials`). Empty, and never allocating,
    /// without sector thinkers. Never read or encoded.
    pub(crate) spare_sector_slots: Vec<store::Slot<sector_thinkers::SectorThinker>>,
    /// The runtime special of every line (`line_t::special`), changed by W1 and S1
    /// lines; empty, and never allocating, under every policy without movers, whose
    /// line specials are the map's.
    pub(crate) line_specials: Vec<i16>,
    /// The sector and line-special staging buffers of the last committed plan (the
    /// tables a commit replaced), reused as the next plan's (like `spare_specials`).
    /// Never read or encoded.
    pub(crate) spare_writes: staging::StagedWrites,
    /// Under the movers policy, what its load rules derived from the immutable map
    /// (families, `M`, reachable heights, `Δ`); never canonical state.
    pub(crate) movers: Option<mover_rules::MoverAdmission>,
    /// Under the movers policy, source `gameaction == ga_completed`: the exit switch
    /// ran `G_ExitLevel` and every later tic is refused (`SimError::LevelExited`).
    pub(crate) level_exit: bool,
    /// Under the Episode 1 policy, source `secretexit`: the exit was
    /// `G_SecretExitLevel` (special 51, g_game.c:1009–1018), so `level_exit` is
    /// set too (DSIM39 `level.secret_exit`). False under every other policy.
    pub(crate) level_secret_exit: bool,
    /// Under the Episode 1 part 2 policy (wire 24) alone, source `gamemap`: the
    /// map number 1–9 of the Episode 1 slot, given to the constructor and constant
    /// for the run (DSIM40 `level.map`; read by `A_BossDeath`, p_enemy.c:1629–1634,
    /// and `PIT_StompThing`, p_map.c:102). `None` under every other policy.
    pub(crate) level_map: Option<u8>,
    /// Under the movers policy, the running switch buttons (`buttonlist`): timer,
    /// line and sound-origin sector of each slot, DSIM35 state (decision N9).
    pub(crate) buttons: use_lines::Buttons,
}
/// Static-geometry table bounds a construction prepares against: the M3f2/M3h3
/// constants for every historical profile, the WAD format's own bounds (M3h6 note
/// §4.1) under the real-map policy.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GeometryCapacity {
    pub lines: usize,
    pub sectors: usize,
    /// Subsectors and nodes.
    pub tree: usize,
    pub cells: usize,
    /// Map line and sector specials pass geometry construction (the specials
    /// policy validates its admitted set before construction).
    pub specials: bool,
}
impl GeometryCapacity {
    pub const CLASSIC: Self = Self {
        lines: MAP_SCALE_MAX_LINES,
        sectors: MAP_SCALE_MAX_SECTORS,
        tree: 4096,
        cells: MAP_EXTENT_MAX_CELLS,
        specials: false,
    };
    /// `short` references bound every table at 32,767 records; BLOCKMAP list
    /// offsets are `short` word indices, so the lump holds at most 32,768 words and
    /// fewer cells than words.
    pub const FORMAT: Self = Self {
        lines: player_map_scale::REAL_MAP_FORMAT_RECORDS,
        sectors: player_map_scale::REAL_MAP_FORMAT_RECORDS,
        tree: player_map_scale::REAL_MAP_FORMAT_RECORDS,
        cells: 32_768,
        specials: false,
    };
}
/// A simulation moves between threads and is shared by reference across them; the
/// validation memo is atomic so this holds at compile time.
const _: () = {
    const fn send_and_sync<T: Send + Sync>() {}
    send_and_sync::<Simulation>();
};
impl Simulation {
    /// Creates a fresh supported world with capacity for 1024 logical actor slots.
    /// Map THINGS are retained as data; complete type-driven spawning is M3 work.
    pub fn new(map: Map) -> Result<Self, SimError> {
        Self::with_actor_limit(map, 1024)
    }
    /// Creates a fresh world with an explicit maximum of 65536 logical slots.
    pub fn with_actor_limit(map: Map, max_actors: u32) -> Result<Self, SimError> {
        Self::create(map, max_actors, Profile::Foundation)
    }
    /// M3a static grounded player/solid collision profile (DSIM-v2), 1024 slots.
    pub fn player_collision(map: Map) -> Result<Self, SimError> {
        Self::with_player_collision_limit(map, 1024)
    }
    /// M3a profile with at most 1024 logical slots and bounded geometry.
    pub fn with_player_collision_limit(map: Map, max_actors: u32) -> Result<Self, SimError> {
        if max_actors > 1024 {
            return Err(StoreError::InvalidLimit.into());
        }
        Self::create(map, max_actors, Profile::PlayerCollision)
    }
    /// Explicit M3b static multi-sector and XY/Z profile (DSIM-v3). Supplied
    /// BLOCKMAP lists define collision candidates; missing spatial coverage is
    /// diagnosed conservatively, never silently rebuilt.
    pub fn multisector(map: Map) -> Result<Self, SimError> {
        Self::with_multisector_limit(map, 1024)
    }
    /// M3b profile with an explicit maximum of 1024 logical actor slots.
    pub fn with_multisector_limit(map: Map, max_actors: u32) -> Result<Self, SimError> {
        if max_actors > 1024 {
            return Err(StoreError::InvalidLimit.into());
        }
        Self::create(map, max_actors, Profile::MultiSector)
    }
    fn create(map: Map, max_actors: u32, profile: Profile) -> Result<Self, SimError> {
        Self::create_bounded(map, max_actors, profile, MAP_SCALE_MAX_REFERENCES)
    }
    fn create_bounded(
        map: Map,
        max_actors: u32,
        profile: Profile,
        reference_budget: usize,
    ) -> Result<Self, SimError> {
        Self::create_bounded_with(
            map,
            max_actors,
            profile,
            reference_budget,
            GeometryCapacity::CLASSIC,
        )
    }
    fn create_bounded_with(
        map: Map,
        max_actors: u32,
        profile: Profile,
        reference_budget: usize,
        capacity: GeometryCapacity,
    ) -> Result<Self, SimError> {
        if profile == Profile::MapScale {
            if map.linedefs().len() > MAP_SCALE_MAX_LINES
                || map.sectors().len() > MAP_SCALE_MAX_SECTORS
            {
                return Err(GeometryError::Unsupported("map-scale geometry capacity").into());
            }
            let origin = map.blockmap().origin();
            if origin.x.unsigned_abs() > 512 || origin.y.unsigned_abs() > 512 {
                return Err(GeometryError::CoordinateOverflow.into());
            }
            sight::validate_map(&map)?;
        }
        let mut world = if profile.uses_sectors() {
            StaticWorld::new_multisector(map, capacity.specials)?
        } else {
            StaticWorld::new(map)?
        };
        if profile == Profile::PlayerCollision {
            movement::validate_profile(&mut world)?;
        }
        if profile.uses_sectors() {
            if profile.uses_expanded_geometry() {
                if profile != Profile::MapScale {
                    movement::prepare_extent_with(
                        &mut world,
                        capacity.lines,
                        reference_budget,
                        capacity.cells,
                    )?;
                } else {
                    movement::prepare_map_scale(&mut world, reference_budget)?;
                }
                world.topology = Some(multisector::Topology::new_with_capacity(
                    &world,
                    capacity.sectors,
                    capacity.tree,
                )?);
            } else {
                movement::prepare_reference_profile(&mut world)?;
                world.topology = Some(multisector::Topology::new(&world)?);
            }
        }
        let mut sectors = if profile.uses_sectors() {
            line_work::reserved(world.map().sectors().len())?
        } else {
            Vec::new()
        };
        if profile.uses_sectors() {
            sectors.extend(world.map().sectors().iter().map(|s| SectorState {
                floor: Fixed::from_map_units(s.heights[0]),
                ceiling: Fixed::from_map_units(s.heights[1]),
            }));
        }
        let map_fields = Arc::new(snapshot::map_fields(world.map()));
        Ok(Self {
            profile,
            perception_players: [PerceptionPlayer::default(); 4],
            registry: None,
            weapon_player: None,
            original_player: None,
            sound_targets: Vec::new(),
            actor_options: ActorOptions::default(),
            statistics: None,
            sectors,
            spatial: movement::SpatialIndex::default(),
            world,
            actors: ThinkerStore::new(max_actors)?,
            rng: DoomRng::default(),
            tic: 0,
            paused: false,
            bindings: [None; 4],
            commands: [TicCommand::default(); 4],
            events: Vec::new(),
            pending: Vec::new(),
            map_fields,
            admitted_types: demon_lifecycle::AdmittedTypes::default(),
            type_envelope: None,
            boundary: None,
            spare_sound_targets: Vec::new(),
            specials: None,
            spare_specials: Vec::new(),
            spare_sector_slots: Vec::new(),
            line_specials: Vec::new(),
            spare_writes: staging::StagedWrites::default(),
            movers: None,
            level_exit: false,
            level_secret_exit: false,
            level_map: None,
            buttons: [None; mover_rules::MAX_BUTTONS],
        })
    }
    /// The sectors a type's dimensions are admitted against: the heights policy's
    /// envelope, or every static sector.
    pub(crate) fn dimension_sectors(&self) -> &[SectorState] {
        match &self.type_envelope {
            Some(envelope) => std::slice::from_ref(envelope),
            None => &self.sectors,
        }
    }
    /// The committed runtime sectors as a read-through view without staged writes:
    /// every query outside a checked plan, and every historical profile.
    #[inline]
    pub(crate) fn sector_view(&self) -> staging::Sectors<'_> {
        staging::Sectors::committed(&self.sectors)
    }
    /// Historical service-family selection. For original-ID contexts, also use
    /// [`Self::original_execution_profile`] to distinguish the explicit policies.
    pub const fn profile(&self) -> Profile {
        self.profile
    }
    /// Most-recent spatial link order in M3a, including off-map live actors.
    pub fn spatial_order(&self) -> &[ActorId] {
        &self.spatial.order
    }
    /// Read-only M3a collision query at a destination; no occupancy side effects.
    pub fn check_actor_position(
        &self,
        id: ActorId,
        target: [Fixed; 2],
    ) -> Result<ActorMoveAttempt, SimError> {
        if self.profile != Profile::PlayerCollision {
            return Err(GeometryError::Unsupported("requires M3a profile").into());
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        let actor = self.actors.get(id).ok_or(StoreError::InvalidHandle)?;
        Ok(movement::check(
            &self.world,
            id,
            actor,
            target,
            &self.spatial,
            &|id| self.actors.get(id).copied(),
        ))
    }
    /// Current read-only runtime sector state (empty in older profiles).
    pub fn sectors(&self) -> &[SectorState] {
        &self.sectors
    }
    /// Current sector-head ordering, independent of thinker or handle order.
    pub fn sector_actor_order(&self, sector: u16) -> Result<Vec<ActorId>, SimError> {
        if !self.profile.uses_sectors() {
            return Err(GeometryError::Unsupported("sector order requires M3b profile").into());
        }
        if usize::from(sector) >= self.sectors.len() {
            return Err(GeometryError::InvalidReference.into());
        }
        Ok(self
            .spatial
            .order
            .iter()
            .copied()
            .filter(|id| lifecycle::in_sector_list(self.actors.get(*id).expect("linked actor")))
            .filter(|id| {
                self.actors
                    .get(*id)
                    .expect("live spatial handle")
                    .location
                    .expect("M3b membership")
                    .sector
                    == sector
            })
            .collect())
    }
    /// P_LineOpening calculation from current runtime heights. One-sided lines
    /// return None; unsupported profile/index errors have no state effects.
    pub fn line_opening(&self, index: usize) -> Result<Option<Opening>, SimError> {
        if !self.profile.uses_sectors() {
            return Err(GeometryError::Unsupported("openings require M3b profile").into());
        }
        if index >= self.world.lines.len() {
            return Err(GeometryError::InvalidReference.into());
        }
        Ok(multisector::opening(&self.world, self.sector_view(), index))
    }
    /// Read-only source TryMove decision, including step/dropoff/headroom checks.
    pub fn check_multisector_position(
        &self,
        id: ActorId,
        target: [Fixed; 2],
    ) -> Result<SectorMoveAttempt, SimError> {
        if !self.profile.uses_sectors() {
            return Err(GeometryError::Unsupported("sector collision requires M3b profile").into());
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        if self.profile == Profile::Projectiles {
            self.preflight_projectile_step()?;
            if target
                .iter()
                .any(|v| v.bits().unsigned_abs() > 8192 * 65536)
            {
                return Err(ProjectileError::Arithmetic.into());
            }
            return Ok(self.query_projectile_move(id, target).0);
        }
        if self.profile == Profile::MapScale || self.profile.uses_checked_demons() {
            map_scale::bounded(target, self.sight_margin())?;
            if self.profile.uses_checked_demons() {
                self.perception_actor(id)?;
            }
            if self.player_policy().has_imps()
                && self
                    .actor(id)
                    .and_then(Actor::life)
                    .is_some_and(|l| original_registry::is_missile_type(l.actor_type))
            {
                return Ok(self.query_projectile_move(id, target).0);
            }
        }
        if self.profile == Profile::MapExtent {
            map_extent::position(target, map_extent::QUERY_UNITS)?;
        }
        let actor = self.actors.get(id).ok_or(StoreError::InvalidHandle)?;
        Ok(multisector::check_move(
            (&self.world, self.sector_view()),
            id,
            actor,
            target,
            &self.spatial,
            &|id| self.actors.get(id).copied(),
            self.bindings.contains(&Some(id)),
        ))
    }
    /// Static immutable geometry.
    pub const fn world(&self) -> &StaticWorld {
        &self.world
    }
    /// Number of completed logical tics, independent of presentation rate.
    pub const fn tic(&self) -> u64 {
        self.tic
    }
    /// Whether stepping is frozen before all gameplay phases.
    pub const fn paused(&self) -> bool {
        self.paused
    }
    /// Explicit pause control. Paused step calls consume no commands/RNG/events.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }
    /// Read-only actor lookup; marked storage remains readable until retirement.
    pub fn actor(&self, id: ActorId) -> Option<&Actor> {
        self.actors.get(id)
    }
    /// Linked processing order, independent of slot reuse.
    pub fn actor_order(&self) -> impl Iterator<Item = ActorId> + '_ {
        self.actors.order()
    }
    /// Copy of RNG state for observation; mutating the copy cannot affect simulation.
    pub const fn rng(&self) -> DoomRng {
        self.rng
    }
    /// Last completed tic's events; never drained by presentation.
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    /// Read-only observations from explicit operations since the last tic.
    /// These are retained in the next completed tic and in canonical pending state.
    pub fn pending_events(&self) -> &[Event] {
        &self.pending
    }
    /// Explicit deterministic gameplay draw, queued as an observation for next tic.
    pub fn gameplay_random(&mut self) -> u8 {
        let value = self.rng.p_random();
        self.pending.push(Event::GameplayRandom(value));
        value
    }
    /// Spawns a parameterized grounded mover, consuming exactly one P_Random for
    /// lastlook after storage succeeds. Full P_SpawnMobj type/state setup is absent.
    pub fn spawn(&mut self, spec: ActorSpec) -> Result<ActorId, SimError> {
        self.spawn_with_solidity(spec, Solidity::NonSolid)
    }
    /// Parameterized setup with zero/MF_SOLID flags in M3a. Like P_SpawnMobj,
    /// overlapping initial placements are allowed; query occupancy explicitly.
    /// Validation/capacity errors preserve logical state, index order and RNG.
    pub fn spawn_with_solidity(
        &mut self,
        spec: ActorSpec,
        solidity: Solidity,
    ) -> Result<ActorId, SimError> {
        if self.profile == Profile::ActorStates {
            return Err(ActorError::UnsupportedMode.into());
        }
        if self.profile.uses_sectors() {
            return self.spawn_vertical(spec, solidity, VerticalSpec::default());
        }
        self.world.validate_actor(spec.radius, spec.height)?;
        if self.profile == Profile::Foundation && solidity != Solidity::NonSolid {
            return Err(GeometryError::Unsupported("solid actors require M3a profile").into());
        }
        if self.profile == Profile::PlayerCollision {
            self.spatial.reserve_spawn()?;
        }
        let actor = Actor {
            spec,
            floor: self.world.floor(),
            ceiling: self.world.ceiling(),
            z: self.world.floor(),
            lastlook: 0,
            solidity,
            momz: Fixed::ZERO,
            gravity: Gravity::Normal,
            allow_dropoff: false,
            location: None,
            life: None,
        };
        let id = self.actors.spawn(actor)?;
        self.actors
            .get_mut(id)
            .expect("newly spawned actor handle")
            .lastlook = self.rng.p_random() % 4;
        if self.profile == Profile::PlayerCollision {
            self.spatial.link(id);
        }
        self.pending.push(Event::Spawned(id));
        Ok(id)
    }
    /// M3b parameterized spawn with source floor/ceiling/absolute Z, vertical
    /// momentum and explicit MF_NOGRAVITY/MF_DROPOFF exceptions. No state/type,
    /// teleport, floating monster or collision-side-effect spawning is implied.
    pub fn spawn_vertical(
        &mut self,
        spec: ActorSpec,
        solidity: Solidity,
        vertical: VerticalSpec,
    ) -> Result<ActorId, SimError> {
        if !matches!(
            self.profile,
            Profile::MultiSector | Profile::MapScale | Profile::MapExtent
        ) {
            return Err(GeometryError::Unsupported(
                "parameterized vertical spawn requires M3b profile",
            )
            .into());
        }
        if self.profile == Profile::MapScale {
            map_scale::spawn_bounds(spec, vertical)?;
        }
        if self.profile == Profile::MapExtent {
            map_extent::spawn_bounds(spec, vertical)?;
        }
        let location = self.world.locate_validated(spec.position);
        let sector = self.sectors[usize::from(location.sector)];
        if spec.radius < Fixed::ZERO
            || spec.radius > Fixed::from_map_units(32)
            || spec.height <= Fixed::ZERO
            || spec.height > sector.ceiling - sector.floor
        {
            return Err(GeometryError::InvalidActor.into());
        }
        let z = match vertical.height {
            SpawnHeight::Floor => sector.floor,
            SpawnHeight::Ceiling => sector.ceiling - spec.height,
            SpawnHeight::Absolute(z) => z,
        };
        self.spatial.reserve_spawn()?;
        let actor = Actor {
            spec,
            floor: sector.floor,
            ceiling: sector.ceiling,
            z,
            lastlook: 0,
            solidity,
            momz: vertical.momentum,
            gravity: vertical.gravity,
            allow_dropoff: vertical.allow_dropoff,
            location: Some(location),
            life: None,
        };
        let id = self.actors.spawn(actor)?;
        self.actors.get_mut(id).expect("new M3b handle").lastlook = self.rng.p_random() % 4;
        self.spatial.link(id);
        self.pending.push(Event::Spawned(id));
        Ok(id)
    }
    /// Changes M3b vertical momentum between tics; invalid/profile errors preserve state.
    pub fn set_vertical_momentum(&mut self, id: ActorId, momentum: Fixed) -> Result<(), SimError> {
        if self.owns_player_body(id) {
            return Err(PlayerBodyError::Unsupported.into());
        }
        if matches!(self.profile, Profile::MapScale | Profile::MapExtent) {
            map_scale::bounded([momentum], 64)?;
        }
        if !self.profile.uses_sectors() {
            return Err(
                GeometryError::Unsupported("vertical momentum requires M3b profile").into(),
            );
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        self.projectile_control(id)?;
        if self.profile.uses_monsters() && self.monster_is_owner(id) && momentum != Fixed::ZERO {
            return Err(MonsterError::Ownership.into());
        }
        if self.profile.uses_perception() && momentum.bits().unsigned_abs() > 64 * 65536 {
            return Err(PerceptionError::Arithmetic.into());
        }
        if self.profile == Profile::Projectiles && momentum.bits().unsigned_abs() > 64 * 65536 {
            return Err(ProjectileError::Arithmetic.into());
        }
        // The E1M1 policy refuses vertical momentum on a decoration or an item
        // (`MF_SPECIAL`) after the shared input bounds, as `set_momentum` refuses a
        // push: both are spawned at `ONFLOORZ` (p_mobj.c:775–783) and nothing in the
        // source gives them `momz` (`P_DamageMobj` needs `MF_SHOOTABLE`,
        // p_inter.c:787–788), so `P_ZMovement` never runs for them (p_mobj.c:428–436)
        // and neither the decoration witness (M3h9 note §2.5) nor the relaxed item
        // witness (M3h8 note §4.2) admits one off its floor. Older policies keep
        // their historical admission (M3h9 validation record, independent review).
        if self.player_policy().has_e1m1()
            && momentum != Fixed::ZERO
            && self.actor(id).and_then(Actor::life).is_some_and(|l| {
                l.flags & world_pickups::MF_SPECIAL != 0
                    || e1m1_rules::is_decoration(self.player_policy(), l.actor_type)
            })
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.actors.get_mut(id).unwrap().momz = momentum;
        Ok(())
    }
    /// Marks removal and unbinds input slots immediately. Storage retires at traversal.
    /// The fixed-active capability rejects removal of its owned player body.
    pub fn mark_remove(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.uses_active_player_roster() && self.owns_player_body(id) {
            return Err(PlayerBodyError::Unsupported.into());
        }
        // The E1M1 policy never removes a decoration (`P_RemoveMobj` is reached only
        // through death, pickup or a missile's end, M3h9 note §5.1 item 2); the tic
        // preflight and the DSIM36 reader admit no pending decoration.
        if self.actors.is_pending(id) == Some(false)
            && self.actor(id).and_then(Actor::life).is_some_and(|l| {
                e1m1_rules::is_decoration(self.player_policy(), l.actor_type)
                    || barrels::is_barrel(self.player_policy(), l.actor_type)
            })
        {
            // The barrels policy removes a barrel only through its own `S_NULL`
            // (p_mobj.c:62–66, M3h9b decision F4(v)).
            return Err(PlayerBodyError::Unsupported.into());
        }
        // Removing a flying fireball's owner could never step again.
        self.preflight_owner_removal(id)?;
        if self.actors.mark_remove(id)? {
            self.spatial.unlink(id);
            self.pending.push(Event::RemovalMarked(id));
        }
        for binding in &mut self.bindings {
            if *binding == Some(id) {
                *binding = None;
            }
        }
        Ok(())
    }
    /// Changes explicit momentum between tics; next update clamps it like the reference.
    /// Under the diagonal policy this is the declared mirror of the C harness's
    /// `push_*` input: the player body (the corner-jump replay), monsters, corpses
    /// and items are admitted (fireballs stay refused by the projectile control);
    /// every older physical-player policy keeps refusing them all.
    pub fn set_momentum(&mut self, id: ActorId, momentum: [Fixed; 2]) -> Result<(), SimError> {
        if (self.owns_player_body(id) || self.player_policy().has_physical_player())
            && !self.player_policy().has_diagonals()
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        if self.profile.uses_checked_demons() {
            map_scale::bounded(momentum, 64)?;
        }
        if matches!(self.profile, Profile::MapScale | Profile::MapExtent) {
            map_scale::bounded(momentum, 64)?;
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        // The real-map policy refuses a push to an item (`MF_SPECIAL`): nothing in
        // the source gives a pickup momentum, and the DSIM31+ item witness and the
        // tic preflight never admit one that moves. The diagonal policy keeps its
        // wider admission unchanged (M3h6 note §8.3). The E1M1 policy refuses a
        // push to a decoration the same way: thrust comes only from `P_DamageMobj`,
        // which needs `MF_SHOOTABLE` (p_map.c:304–305, 1013–1014, 1170–1171), and
        // the decoration witness never admits one that moves.
        if self.player_policy().has_real_map()
            && momentum != [Fixed::ZERO; 2]
            && self.actor(id).and_then(Actor::life).is_some_and(|l| {
                l.flags & world_pickups::MF_SPECIAL != 0
                    || e1m1_rules::is_decoration(self.player_policy(), l.actor_type)
            })
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.projectile_control(id)?;
        if self.profile.uses_monsters()
            && !self.profile.uses_demon_lifecycle()
            && self.monster_is_owner(id)
            && momentum != [Fixed::ZERO; 2]
        {
            return Err(MonsterError::Ownership.into());
        }
        if self.profile.uses_perception()
            && momentum
                .iter()
                .any(|v| v.bits().unsigned_abs() > 4096 * 65536)
        {
            return Err(PerceptionError::Arithmetic.into());
        }
        if self.profile == Profile::Projectiles
            && momentum
                .iter()
                .any(|v| v.bits().unsigned_abs() > 4096 * 65536)
        {
            return Err(ProjectileError::Arithmetic.into());
        }
        self.actors.get_mut(id).unwrap().spec.momentum = momentum;
        Ok(())
    }
    /// Binds one of four ordered command slots. Controlled movers keep the
    /// M2 non-player response or M3a player XY response, according to the profile.
    /// This does not implement full type/state-driven player spawning.
    pub fn bind_player(&mut self, slot: usize, id: ActorId) -> Result<(), SimError> {
        if matches!(self.profile, Profile::MapScale | Profile::MapExtent) {
            return Err(GeometryError::Unsupported("map-scale player binding").into());
        }
        if self.profile.uses_perception() {
            return Err(PerceptionError::Roster.into());
        }
        if self.profile == Profile::Projectiles {
            return Err(ProjectileError::Player.into());
        }
        if self.profile == Profile::PlayerWeapons {
            return Err(WeaponError::Player.into());
        }
        if slot >= 4
            || self
                .bindings
                .iter()
                .enumerate()
                .any(|(i, v)| i != slot && *v == Some(id))
        {
            return Err(SimError::InvalidPlayer);
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        self.bindings[slot] = Some(id);
        Ok(())
    }
    /// Run [`Self::advance`] and capture one diagnostic Snapshot on success.
    /// A paused call captures unchanged state. Recoverable errors preserve
    /// canonical state and capture nothing. Checked transactions retain rollback
    /// in debug and release. Allocation exhaustion outside fallible reservations
    /// remains process-level Rust policy; invariant assertions indicate bugs.
    pub fn step(&mut self, commands: [TicCommand; 4]) -> Result<Snapshot, SimError> {
        self.advance(commands)?;
        Ok(self.snapshot())
    }

    /// Advance one logical tic without constructing a diagnostic Snapshot.
    /// Paused calls succeed without changing time, commands, RNG or events.
    /// Recoverable errors preserve canonical state, just as with [`Self::step`].
    /// Ordered events remain available through [`Self::events`]; gameplay and
    /// transactional validation may still allocate. Call [`Self::snapshot`]
    /// explicitly when diagnostics are needed, including in release builds.
    pub fn advance(&mut self, commands: [TicCommand; 4]) -> Result<(), SimError> {
        // The specials policy's explicit load operation must run first (M3h7
        // decision 12): no tic exists before the level's sector thinkers.
        if self.specials.as_ref().is_some_and(|s| !s.spawned) {
            return Err(MapSpawnError::LevelSpecials.into());
        }
        // G_Ticker runs a pending gameaction before anything else, paused or not.
        if self.level_exit {
            return Err(SimError::LevelExited);
        }
        if self.original_player.as_ref().is_some_and(|p| p.reborn) {
            return Err(SimError::PlayerReborn);
        }
        if self.paused {
            return Ok(());
        }
        if self.tic == u64::MAX {
            return Err(SimError::TicOverflow);
        }
        if self.profile == Profile::PlayerWeapons {
            self.preflight_weapon_step(&commands)?;
        } else if !self.player_policy().has_player_weapons()
            && commands.iter().any(|c| c.buttons != 0)
        {
            return Err(SimError::UnsupportedButtons);
        }
        if self.original_player.is_some()
            && !self.player_policy().has_player_commands()
            && commands.iter().any(|c| *c != TicCommand::default())
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        if self.profile.uses_checked_demons() {
            return self.advance_expanded_demons(commands);
        }
        if self.profile == Profile::Projectiles {
            self.preflight_projectile_step()?;
        }
        let monster_shot = if matches!(self.profile, Profile::MonsterAttacks | Profile::Chaingunner)
        {
            self.preflight_monster_step()?
        } else {
            None
        };
        let shotgun_blast = if self.profile == Profile::Shotgunner {
            self.preflight_shotgun_step()?
        } else {
            None
        };
        if self.profile == Profile::Pursuit {
            self.preflight_pursuit_step()?;
        } else if self.profile == Profile::Perception {
            self.preflight_perception_step()?;
        }
        let demon_melee = if self.profile.uses_demon() {
            self.preflight_demon_step()?
        } else {
            None
        };
        if self.profile == Profile::MapScale {
            self.preflight_map_scale_step()?;
        }
        if self.profile == Profile::MapExtent {
            self.preflight_map_extent_step()?;
        }
        self.preflight_actor_step()?;
        self.events = std::mem::take(&mut self.pending);
        self.commands = commands;
        self.events.push(Event::Phase(Phase::Players));
        if self.profile == Profile::PlayerWeapons {
            self.weapon_tick(commands);
            self.events.append(&mut self.pending);
        } else {
            for (slot, command) in commands.into_iter().enumerate() {
                if let Some(id) = self.bindings[slot] {
                    if self.actors.is_pending(id) != Some(false) {
                        self.bindings[slot] = None;
                        continue;
                    }
                    let actor = self.actors.get_mut(id).expect("validated active binding");
                    let physical_command = if self.profile.uses_sectors() && actor.z > actor.floor {
                        TicCommand {
                            forward: 0,
                            side: 0,
                            ..command
                        }
                    } else {
                        command
                    };
                    let (angle, momentum) =
                        command_thrust(actor.spec.angle, actor.spec.momentum, physical_command)
                            .expect("buttons prevalidated");
                    actor.spec.angle = angle;
                    actor.spec.momentum = momentum;
                    self.events.push(Event::Player {
                        slot: slot as u8,
                        actor: id,
                    });
                }
            }
        }
        self.events.push(Event::Phase(Phase::Thinkers));
        if self.profile == Profile::Projectiles {
            self.projectile_thinkers_with(|_, _| {});
        } else {
            let perception_inputs =
                (self.profile.uses_perception()).then_some(perception::PerceptionInputs {
                    pursuit: self.profile == Profile::Pursuit || self.profile.uses_monsters(),
                    monsters: self.profile.uses_monsters(),
                    shotgunner: self.profile == Profile::Shotgunner,
                    chaingunner: self.profile == Profile::Chaingunner,
                    demon: self.profile.uses_demon(),
                    map_demons: self.profile.uses_demon_map(),
                    checked: None,
                    melee: demon_melee.as_ref(),
                    shotgun: shotgun_blast.as_ref(),
                    shot: monster_shot.as_ref(),
                    players: crate::player_roster::PerceptionRoster::Historical(
                        &self.perception_players,
                    ),
                    sound_targets: &self.sound_targets,
                });
            thinkers::run(
                &mut self.actors,
                thinkers::Inputs {
                    world: &self.world,
                    committed: &self.sectors,
                    committed_lines: &[],
                    writes: &mut staging::StagedWrites::default(),
                    leveltime: self.tic,
                    profile: self.profile,
                    bindings: self.bindings,
                    live_bindings: &mut self.bindings,
                    registry: &self.registry,
                    options: self.actor_options,
                    statistics: &mut self.statistics,
                    player: None,
                    rng: &mut self.rng,
                    spatial: &mut self.spatial,
                    events: &mut self.events,
                    commands,
                    perception_inputs,
                    boundary: self.boundary.as_deref(),
                    specials: None,
                    buttons: None,
                },
            );
        }
        self.events.push(Event::Phase(Phase::Specials));
        self.events.push(Event::Phase(Phase::Respawn));
        self.tic += 1;
        Ok(())
    }
    /// Captures every implemented state field and immutable map data canonically.
    pub fn snapshot(&self) -> Snapshot {
        snapshot::capture(self)
    }
}

/// Runs a command stream headlessly and records one canonical snapshot per call.
/// Setup/spawn/removal/random requests, if needed, are explicit caller operations.
pub fn run_commands(
    simulation: &mut Simulation,
    commands: impl IntoIterator<Item = [TicCommand; 4]>,
) -> Result<Vec<Snapshot>, SimError> {
    commands
        .into_iter()
        .map(|commands| simulation.step(commands))
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn logical_tic_exhaustion_rejects_before_mutation() {
        let wad = doom_wad::Wad::parse(
            include_bytes!("../../../fuzz/corpus/map/classic_square"),
            doom_wad::Limits::default(),
        )
        .unwrap();
        let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
        for profile in [
            super::Profile::Foundation,
            super::Profile::PlayerCollision,
            super::Profile::MultiSector,
        ] {
            let mut simulation = super::Simulation::create(map.clone(), 1024, profile).unwrap();
            simulation.tic = u64::MAX;
            let before = simulation.snapshot();
            assert_eq!(
                simulation.step([super::TicCommand::default(); 4]),
                Err(super::SimError::TicOverflow)
            );
            assert_eq!(simulation.snapshot(), before);
        }
    }
    #[test]
    fn command_button_rejection_is_explicit() {
        assert_eq!(
            super::command_thrust(
                0,
                [doom_types::Fixed::ZERO; 2],
                super::TicCommand {
                    buttons: 1,
                    ..Default::default()
                }
            ),
            Err(super::SimError::UnsupportedButtons)
        );
    }
    #[test]
    fn allocator_capacity_does_not_change_multisector_state_or_results() {
        let wad = doom_wad::Wad::parse(
            include_bytes!("../../../tests/reference/m3b_map_2.bin"),
            doom_wad::Limits::default(),
        )
        .unwrap();
        let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
        let mut expected = super::Simulation::multisector(map).unwrap();
        let id = expected.spawn(super::ActorSpec::default()).unwrap();
        expected.bind_player(0, id).unwrap();
        let mut actual = expected.clone();
        actual.actors.slots.try_reserve(257).unwrap();
        actual.spatial.order.try_reserve(131).unwrap();
        actual.pending.try_reserve(127).unwrap();
        actual.events.try_reserve(255).unwrap();
        actual.sectors.try_reserve(127).unwrap();
        actual.world.lines.try_reserve(127).unwrap();
        for cell in actual.world.collision_cells.as_mut().unwrap() {
            cell.try_reserve(127).unwrap();
        }
        assert_eq!(expected.snapshot(), actual.snapshot());
        for _ in 0..64 {
            let mut command = [super::TicCommand::default(); 4];
            command[0].forward = 50;
            assert_eq!(
                expected.step(command).unwrap(),
                actual.step(command).unwrap()
            );
        }
    }
}

#[cfg(test)]
mod advance_tests;

#[cfg(test)]
#[path = "../../../tests/support/m3g8.rs"]
mod weapon_pack_evidence;

#[cfg(test)]
#[path = "../../../tests/support/m3g9.rs"]
mod pickup_evidence;
#[cfg(test)]
mod world_pickup_tests;

#[cfg(test)]
#[path = "../../../tests/support/m3g10.rs"]
mod armor_evidence;
#[cfg(test)]
mod armor_tests;

#[cfg(test)]
mod bonus_actor_tests;
#[cfg(test)]
#[path = "../../../tests/support/m3g11.rs"]
mod bonus_evidence;

#[cfg(test)]
mod barrels_replay;
#[cfg(test)]
mod bonus_tests;

#[cfg(test)]
#[path = "../../../tests/support/m3h1.rs"]
mod monster_evidence;
#[cfg(test)]
mod monster_tests;

#[cfg(test)]
#[path = "../../../tests/support/m3h5.rs"]
mod diagonal_evidence;
#[cfg(test)]
mod diagonal_room;
#[cfg(test)]
mod e1m1_replay;
#[cfg(test)]
mod hard_skill_tests;
#[cfg(test)]
#[path = "../../../tests/support/m3h4.rs"]
mod heights_evidence;
#[cfg(test)]
mod heights_room;
#[cfg(test)]
#[path = "../../../tests/support/m3h2.rs"]
mod imp_evidence;
#[cfg(test)]
mod imp_tests;
#[cfg(test)]
#[path = "../../../tests/support/m3h3.rs"]
mod map_scale_evidence;
#[cfg(test)]
mod movers_replay;
#[cfg(test)]
mod player_barrel_combat_tests;
#[cfg(test)]
mod player_barrels_tests;
#[cfg(test)]
mod player_diagonal_tests;
#[cfg(test)]
mod player_e1m1_soak_tests;
#[cfg(test)]
mod player_e1m1_tests;
#[cfg(test)]
mod player_episode1_commands_tests;
#[cfg(test)]
mod player_episode1_items_tests;
#[cfg(test)]
mod player_episode1_movers_tests;
#[cfg(test)]
mod player_episode1_tests;
#[cfg(test)]
mod player_episode1_v2_tests;
#[cfg(test)]
mod player_episode1_weapons_tests;
#[cfg(test)]
mod player_heights_tests;
#[cfg(test)]
mod player_level_tests;
#[cfg(test)]
mod player_map_scale_tests;
#[cfg(test)]
mod player_movers_soak_tests;
#[cfg(test)]
mod player_movers_tests;
#[cfg(test)]
mod player_real_map_tests;
#[cfg(test)]
mod player_specials_tests;
#[cfg(test)]
#[path = "../../../tests/support/m3h6.rs"]
mod real_map_evidence;
#[cfg(test)]
#[path = "../../../tests/support/m3h7.rs"]
mod specials_evidence;
#[cfg(test)]
mod v_prime_tests;
#[cfg(test)]
mod wire24_rejection_tests;
