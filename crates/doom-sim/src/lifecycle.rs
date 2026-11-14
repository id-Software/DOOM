//! Validated source actor metadata and bounded state/action execution.
use crate::{
    Actor, ActorId, ActorSpec, Gravity, Solidity, SpawnHeight, StaticWorld, StoreError,
    ThinkerContext, ThinkerStore,
};
use doom_types::{DoomRng, Fixed};
use std::{collections::BTreeSet, error::Error, fmt};
include!("source_actors.rs");
/// Registry-local type candidate; use identity::OriginalActorType for canonical metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ActorTypeId(pub u16);
/// Registry-local state candidate. Canonical metadata uses identity::OriginalState.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct StateId(pub u16);
/// Original symbolic action identity, indexed by the registry action-name table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionId(pub u16);
/// Explicit synthetic reference-adapter capabilities, not vanilla AI or scripts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbeAction {
    /// Consume one gameplay draw, with an observation.
    Random,
    /// Recursively request state entry, preserving the outer source nextstate.
    Enter(StateId),
    /// Append a typed child, set parent target to child, child target to parent.
    SpawnTarget(ActorTypeId),
    /// Immediate spatial unlink and deferred thinker retirement of self.
    RemoveSelf,
    /// Mark the current resolvable target; stale targets are observed as unavailable.
    RemoveTarget,
    /// Assign a validated self-reference.
    TargetSelf,
}
/// Dispatch metadata; unsupported original callbacks never become no-ops.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorAction {
    /// Source NULL action pointer.
    None,
    /// An original symbolic action (admission requires implemented semantics).
    Original(ActionId),
    /// A visibly separate, bounded synthetic callback used in reference probes.
    Probe(ProbeAction),
}
/// Immutable definition of one actor state, including inert presentation data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorStateDef {
    /// Symbolic name, at most 64 UTF-8 bytes.
    pub name: String,
    /// Original sprite-table index; no sprite pixels are loaded.
    pub sprite: u16,
    /// Original frame/fullbright bit pattern.
    pub frame: u32,
    /// Duration: -1 infinite, zero immediate on entry, positive countdown.
    pub tics: i32,
    /// Typed dispatch identity.
    pub action: ActorAction,
    /// Next state, including S_NULL.
    pub next: StateId,
    /// Preserved original miscellaneous fields.
    pub misc: [i32; 2],
}
/// Complete original mobjinfo properties. Recognition does not imply execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorTypeDef {
    /// Symbolic type name.
    pub name: String,
    /// Source's 23 ordered mobjinfo integer properties; validated by Registry.
    /// Access named properties through the methods below; values retain source units.
    pub properties: [i32; 23],
}
impl ActorTypeDef {
    /// DoomEd number (-1 means not selected by ordinary THINGS lookup).
    pub const fn doomed_number(&self) -> i32 {
        self.properties[0]
    }
    /// Spawn/see/pain/melee/missile/death/xdeath/raise state references.
    pub fn states(&self) -> [StateId; 8] {
        [1, 3, 7, 10, 11, 12, 13, 22].map(|i| StateId(self.properties[i] as u16))
    }
    /// Initial health, without damage processing.
    pub const fn spawn_health(&self) -> i32 {
        self.properties[2]
    }
    /// Initial reaction time except nightmare (not a generic per-tic decrement).
    pub const fn reaction_time(&self) -> i32 {
        self.properties[5]
    }
    /// Source pain-chance metadata; no pain/damage calculation is implied.
    pub const fn pain_chance(&self) -> i32 {
        self.properties[8]
    }
    /// See/attack/pain/death/active sound identifiers, as inert metadata.
    pub fn sounds(&self) -> [u16; 5] {
        [4, 6, 9, 14, 20].map(|i| self.properties[i] as u16)
    }
    /// Source speed value, whose units depend on actor type.
    pub const fn speed(&self) -> i32 {
        self.properties[15]
    }
    /// Type collision radius.
    pub const fn radius(&self) -> Fixed {
        Fixed::from_bits(self.properties[16])
    }
    /// Type collision height.
    pub const fn height(&self) -> Fixed {
        Fixed::from_bits(self.properties[17])
    }
    /// Source mass metadata.
    pub const fn mass(&self) -> i32 {
        self.properties[18]
    }
    /// Source damage metadata; not executable combat behavior.
    pub const fn damage(&self) -> i32 {
        self.properties[19]
    }
    /// Complete original flag bits.
    pub const fn flags(&self) -> u32 {
        self.properties[21] as u32
    }
}
/// Actor/state admission or preflight failure; no private input locations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorError {
    /// Invalid type index.
    InvalidType,
    /// Invalid state/index/definition reference.
    InvalidState,
    /// Sprite, sound, name or duration is invalid.
    InvalidMetadata,
    /// Unsupported original action identity.
    UnsupportedAction(ActionId),
    /// Type flags require later behavior.
    UnsupportedFlags(u32),
    /// Immediate zero/callback state cycles are not safe to execute.
    ImmediateCycle,
    /// Bounded state/spawn closure exceeded supported work.
    WorkLimit,
    /// Spawn dependency cycle in synthetic callback graph.
    SpawnCycle,
    /// Type dimensions do not fit the declared static-sector profile.
    Dimensions,
    /// This type has no death state; controlled death is not arbitrary deletion.
    NoDeathState,
    /// Full bound-player death/respawn is not implemented here.
    BoundDeath,
    /// Operation requires a typed profile, or the game mode is unsupported.
    UnsupportedMode,
}
impl fmt::Display for ActorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "actor-state: {self:?}")
    }
}
impl Error for ActorError {}
/// Single-player skill levels with source filtering/reaction-time rules.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum Skill {
    /// Baby uses THINGS bit 1.
    Baby = 0,
    /// Easy also uses bit 1.
    Easy = 1,
    /// Medium uses bit 2.
    #[default]
    Medium = 2,
    /// Hard uses bit 4.
    Hard = 3,
    /// Nightmare uses bit 4 and zero initial reaction time; no monster respawn.
    Nightmare = 4,
}
/// Explicit map-spawn mode. Only single-player DOOM II is executable in M3c.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ActorGameMode {
    /// Supported filtered map-prop spawning; full player-start creation deferred.
    #[default]
    SinglePlayer,
    /// Recognized but currently rejected before mutation.
    Cooperative,
    /// Recognized but currently rejected before mutation.
    Deathmatch,
}
/// Profile-4 source setup options, included in canonical state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActorOptions {
    /// Source skill.
    pub skill: Skill,
    /// Only SinglePlayer is currently executable.
    pub mode: ActorGameMode,
    /// Source nomonsters filtering precedes unsupported selected-type admission.
    pub no_monsters: bool,
}
/// Immutable validated source/native definitions, separate from dispatch services.
#[derive(Clone, Debug)]
pub struct ActorRegistry {
    pub(crate) states: Vec<ActorStateDef>,
    pub(crate) types: Vec<ActorTypeDef>,
}
pub(crate) fn source_type_id(name: &str) -> Option<ActorTypeId> {
    SOURCE_TYPES
        .iter()
        .position(|(n, _)| *n == name)
        .map(|i| ActorTypeId(i as u16))
}
pub(crate) fn source_player_states() -> [StateId; 8] {
    [1, 3, 7, 10, 11, 12, 13, 22].map(|i| StateId(SOURCE_TYPES[0].1[i] as u16))
}
pub(crate) fn source_sound(name: &str) -> Option<u16> {
    SOURCE_SOUNDS
        .iter()
        .position(|n| *n == name)
        .map(|i| i as u16)
}
pub(crate) fn source_state_id(name: &str) -> Option<StateId> {
    SOURCE_STATES
        .iter()
        .position(|(n, _)| *n == name)
        .map(|i| StateId(i as u16))
}
pub(crate) fn source_type_is(ty: ActorTypeId, name: &str) -> bool {
    SOURCE_TYPES
        .get(usize::from(ty.0))
        .is_some_and(|(original, _)| *original == name)
}
const ALLOWED_FLAGS: u32 = 2 | 4 | 8 | 16 | 32 | 256 | 512 | 1024 | 2048 | 262144 | 33554432;
#[derive(Clone, Copy)]
struct Admission {
    flags: u32,
    look: bool,
    chase: bool,
    attacks: bool,
    shotgun: bool,
    chaingun: bool,
    demon: bool,
    /// The exploding barrel of the barrels policy: `A_Explode` at `S_BEXP4` alone.
    barrel: bool,
    /// The rocket of the Episode 1 policy: `A_Explode` at `S_EXPLODE1` alone.
    rocket: bool,
}
impl Admission {
    const fn special(self) -> SpecialActions {
        if self.barrel {
            SpecialActions::Barrel
        } else if self.rocket {
            SpecialActions::Rocket
        } else if self.demon && self.chaingun {
            SpecialActions::OriginalMonsters
        } else if self.demon {
            SpecialActions::Demon
        } else if self.chaingun {
            SpecialActions::Chaingunner
        } else {
            SpecialActions::None
        }
    }
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum SpecialActions {
    None,
    Chaingunner,
    Demon,
    /// Demon plus original former-human attacks (PlayerMonstersV1 only).
    OriginalMonsters,
    /// `A_Explode` in `S_BEXP4`, the barrel's death chain (barrels policy only,
    /// M3h9b decision F4(ii)): the rocket's `S_EXPLODE1` and every other state
    /// naming it stay `UnsupportedAction`.
    Barrel,
    /// `A_Explode` in `S_EXPLODE1`, the rocket's death chain (Episode 1 policy
    /// only, M4a plan §E.4).
    Rocket,
}
const ENTRY_LIMIT: usize = 128;
const STEP_LIMIT: usize = 8192;
impl ActorRegistry {
    /// Compile all original metadata; unsupported actions/types stay recognizable.
    pub fn vanilla() -> Self {
        let states = SOURCE_STATES
            .iter()
            .map(|(name, v)| ActorStateDef {
                name: (*name).into(),
                sprite: v[0] as u16,
                frame: v[1] as u32,
                tics: v[2],
                action: if v[3] == 0 {
                    ActorAction::None
                } else {
                    ActorAction::Original(ActionId(v[3] as u16))
                },
                next: StateId(v[4] as u16),
                misc: [v[5], v[6]],
            })
            .collect();
        let types = SOURCE_TYPES
            .iter()
            .map(|(name, v)| ActorTypeDef {
                name: (*name).into(),
                properties: *v,
            })
            .collect();
        Self::new(states, types).expect("generated original metadata is validated")
    }
    /// Validate owned metadata before any actor can use it. Original unsupported
    /// action identities may be recognized; execution admission is separate.
    pub fn new(states: Vec<ActorStateDef>, types: Vec<ActorTypeDef>) -> Result<Self, ActorError> {
        if states.is_empty() || states.len() > 2048 || types.len() > 256 {
            return Err(ActorError::WorkLimit);
        }
        if states[0].tics != -1
            || states[0].next != StateId(0)
            || states[0].action != ActorAction::None
        {
            return Err(ActorError::InvalidState);
        }
        for s in &states {
            if s.name.len() > 64 || s.tics < -1 || usize::from(s.sprite) >= SOURCE_SPRITES.len() {
                return Err(ActorError::InvalidMetadata);
            }
            if usize::from(s.next.0) >= states.len() {
                return Err(ActorError::InvalidState);
            }
            match s.action {
                ActorAction::Original(id)
                    if id.0 == 0 || usize::from(id.0) >= SOURCE_ACTIONS.len() =>
                {
                    return Err(ActorError::InvalidMetadata);
                }
                ActorAction::Probe(ProbeAction::Enter(id)) if usize::from(id.0) >= states.len() => {
                    return Err(ActorError::InvalidState);
                }
                ActorAction::Probe(ProbeAction::SpawnTarget(id))
                    if usize::from(id.0) >= types.len() =>
                {
                    return Err(ActorError::InvalidType);
                }
                _ => {}
            }
        }
        for t in &types {
            if t.name.len() > 64 {
                return Err(ActorError::InvalidMetadata);
            }
            for i in [1, 3, 7, 10, 11, 12, 13, 22] {
                if t.properties[i] < 0 || t.properties[i] as usize >= states.len() {
                    return Err(ActorError::InvalidState);
                }
            }
            for i in [4, 6, 9, 14, 20] {
                if t.properties[i] < 0 || t.properties[i] as usize >= SOURCE_SOUNDS.len() {
                    return Err(ActorError::InvalidMetadata);
                }
            }
        }
        Ok(Self { states, types })
    }
    /// Read validated state metadata by candidate ID.
    pub fn state(&self, id: StateId) -> Option<&ActorStateDef> {
        self.states.get(usize::from(id.0))
    }
    /// Read recognized actor metadata, including currently unsupported types.
    pub fn actor_type(&self, id: ActorTypeId) -> Option<&ActorTypeDef> {
        self.types.get(usize::from(id.0))
    }
    /// States in deterministic registry order.
    pub fn states(&self) -> &[ActorStateDef] {
        &self.states
    }
    /// Types in deterministic source/registry order.
    pub fn types(&self) -> &[ActorTypeDef] {
        &self.types
    }
    /// Source type by exact symbolic name.
    pub fn find_type(&self, name: &str) -> Option<ActorTypeId> {
        self.types
            .iter()
            .position(|t| t.name == name)
            .map(|i| ActorTypeId(i as u16))
    }
    /// Source state by exact symbolic name.
    pub fn find_state(&self, name: &str) -> Option<StateId> {
        self.states
            .iter()
            .position(|s| s.name == name)
            .map(|i| StateId(i as u16))
    }
    /// Last-match-free original ordered DoomEd lookup.
    pub fn find_doomed(&self, number: i32) -> Option<ActorTypeId> {
        self.types
            .iter()
            .position(|t| t.doomed_number() == number)
            .map(|i| ActorTypeId(i as u16))
    }
    /// Original symbolic callback name, without granting execution support.
    pub fn action_name(&self, id: ActionId) -> Option<&'static str> {
        SOURCE_ACTIONS.get(usize::from(id.0)).copied()
    }
    /// Whether a callback is admitted by the baseline lifecycle profiles.
    /// Profile 8 separately admits A_Look; profile 9 adds constrained A_Chase.
    pub fn action_supported(&self, id: ActionId) -> bool {
        matches!(
            self.action_name(id),
            Some("A_Fall" | "A_Pain" | "A_Scream" | "A_XScream" | "A_PlayerScream")
        )
    }
    /// Validate an executable state closure without changing simulation state.
    pub fn validate_state(&self, state: StateId) -> Result<(), ActorError> {
        self.validate_state_with(state, false, false, false, false, SpecialActions::None)
    }
    pub(crate) fn validate_state_perception(&self, state: StateId) -> Result<(), ActorError> {
        self.validate_state_with(state, true, false, false, false, SpecialActions::None)
    }
    fn validate_state_with(
        &self,
        state: StateId,
        look: bool,
        chase: bool,
        attacks: bool,
        shotgun: bool,
        specialized: SpecialActions,
    ) -> Result<(), ActorError> {
        let mut seen = BTreeSet::new();
        let mut pending = vec![state];
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            let s = self.state(id).ok_or(ActorError::InvalidState)?;
            if id.0 == 0 {
                continue;
            }
            if let ActorAction::Original(action) = s.action {
                if !self.action_supported(action)
                    && !(look && self.action_name(action) == Some("A_Look"))
                    && !(chase && self.action_name(action) == Some("A_Chase"))
                    && !(specialized == SpecialActions::Demon
                        && matches!(
                            self.action_name(action),
                            Some("A_FaceTarget" | "A_SargAttack")
                        ))
                    && !(specialized == SpecialActions::OriginalMonsters
                        && matches!(
                            self.action_name(action),
                            Some(
                                "A_FaceTarget"
                                    | "A_SargAttack"
                                    | "A_PosAttack"
                                    | "A_SPosAttack"
                                    | "A_CPosAttack"
                                    | "A_CPosRefire"
                                    | "A_TroopAttack"
                            )
                        ))
                    // The Baron's two own callbacks, each at the one state that
                    // names it (info.c:675, 684); its role is wire 24's alone.
                    && !(specialized == SpecialActions::OriginalMonsters
                        && ((id.0 == crate::identity::OriginalState::S_BOSS_ATK3.ordinal()
                            && self.action_name(action) == Some("A_BruisAttack"))
                            || (id.0 == crate::identity::OriginalState::S_BOSS_DIE7.ordinal()
                                && self.action_name(action) == Some("A_BossDeath"))))
                    && !(specialized == SpecialActions::Barrel
                        && id.0 == crate::identity::OriginalState::S_BEXP4.ordinal()
                        && self.action_name(action) == Some("A_Explode"))
                    && !(specialized == SpecialActions::Rocket
                        && id.0 == crate::identity::OriginalState::S_EXPLODE1.ordinal()
                        && self.action_name(action) == Some("A_Explode"))
                    && !(specialized == SpecialActions::Chaingunner
                        && matches!(
                            self.action_name(action),
                            Some("A_FaceTarget" | "A_CPosAttack" | "A_CPosRefire")
                        ))
                    && !(shotgun
                        && matches!(
                            self.action_name(action),
                            Some("A_FaceTarget" | "A_SPosAttack")
                        ))
                    && !(attacks
                        && matches!(
                            self.action_name(action),
                            Some("A_FaceTarget" | "A_PosAttack")
                        ))
                {
                    return Err(ActorError::UnsupportedAction(action));
                }
            }
            pending.push(s.next);
            if let ActorAction::Probe(ProbeAction::Enter(target)) = s.action {
                pending.push(target);
            }
        }
        for id in seen {
            let mut path = BTreeSet::new();
            self.immediate_cycle(id, &mut path, &mut BTreeSet::new())?;
            let mut tics = 0;
            let mut work = 0;
            self.flow(id, &mut tics, &mut work, &mut Vec::new())?;
        }
        Ok(())
    }
    fn immediate_cycle(
        &self,
        id: StateId,
        path: &mut BTreeSet<StateId>,
        done: &mut BTreeSet<StateId>,
    ) -> Result<(), ActorError> {
        if id.0 == 0 || done.contains(&id) {
            return Ok(());
        }
        if !path.insert(id) {
            return Err(ActorError::ImmediateCycle);
        }
        if path.len() > ENTRY_LIMIT {
            return Err(ActorError::WorkLimit);
        }
        let s = self.state(id).ok_or(ActorError::InvalidState)?;
        if s.tics == 0 {
            self.immediate_cycle(s.next, path, done)?;
        }
        if let ActorAction::Probe(ProbeAction::Enter(next)) = s.action {
            self.immediate_cycle(next, path, done)?;
        }
        path.remove(&id);
        done.insert(id);
        Ok(())
    }
    fn flow(
        &self,
        mut state: StateId,
        tics: &mut i32,
        work: &mut usize,
        spawns: &mut Vec<ActorTypeId>,
    ) -> Result<(), ActorError> {
        loop {
            *work += 1;
            if *work > ENTRY_LIMIT {
                return Err(ActorError::WorkLimit);
            }
            if state.0 == 0 {
                return Ok(());
            }
            let s = self.state(state).ok_or(ActorError::InvalidState)?;
            *tics = s.tics;
            match s.action {
                ActorAction::Probe(ProbeAction::Enter(next)) => {
                    self.flow(next, tics, work, spawns)?
                }
                ActorAction::Probe(ProbeAction::SpawnTarget(ty)) => spawns.push(ty),
                _ => {}
            }
            if *tics != 0 {
                return Ok(());
            }
            state = s.next;
        }
    }
    pub(crate) fn spawn_dependencies(&self, root: StateId) -> Result<Vec<ActorTypeId>, ActorError> {
        let mut seen = BTreeSet::new();
        let mut todo = vec![root];
        let mut children = BTreeSet::new();
        while let Some(id) = todo.pop() {
            if id.0 == 0 || !seen.insert(id) {
                continue;
            }
            let s = self.state(id).ok_or(ActorError::InvalidState)?;
            todo.push(s.next);
            match s.action {
                ActorAction::Probe(ProbeAction::Enter(next)) => todo.push(next),
                ActorAction::Probe(ProbeAction::SpawnTarget(ty)) => {
                    children.insert(ty);
                }
                _ => {}
            }
        }
        Ok(children.into_iter().collect())
    }
    /// Check source behavior admission and map-wide dimension fit. This rejects
    /// unsupported AI/combat/pickup paths before spawn/RNG or callback execution.
    pub fn validate_type(
        &self,
        ty: ActorTypeId,
        sectors: &[crate::SectorState],
    ) -> Result<(), ActorError> {
        self.admit_type(
            ty,
            sectors,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut 0,
            Admission {
                flags: ALLOWED_FLAGS,
                look: false,
                chase: false,
                attacks: false,
                shotgun: false,
                chaingun: false,
                demon: false,
                barrel: false,
                rocket: false,
            },
        )
    }
    fn admit_type(
        &self,
        ty: ActorTypeId,
        sectors: &[crate::SectorState],
        path: &mut BTreeSet<ActorTypeId>,
        done: &mut BTreeSet<ActorTypeId>,
        work: &mut usize,
        admission: Admission,
    ) -> Result<(), ActorError> {
        if done.contains(&ty) {
            return Ok(());
        }
        *work += 1;
        if *work > 32768 {
            return Err(ActorError::WorkLimit);
        }
        if !path.insert(ty) {
            return Err(ActorError::SpawnCycle);
        }
        let t = self.actor_type(ty).ok_or(ActorError::InvalidType)?;
        if t.flags() & !admission.flags != 0 {
            return Err(ActorError::UnsupportedFlags(t.flags() & !admission.flags));
        }
        if t.radius() < Fixed::ZERO
            || t.radius() > Fixed::from_map_units(32)
            || t.height() <= Fixed::ZERO
            || sectors.iter().any(|s| t.height() > s.ceiling - s.floor)
        {
            return Err(ActorError::Dimensions);
        }
        for root in [t.states()[0], t.states()[5]] {
            self.validate_state_with(
                root,
                admission.look,
                admission.chase,
                admission.attacks,
                admission.shotgun,
                admission.special(),
            )?;
            let mut seen = BTreeSet::new();
            let mut todo = vec![root];
            while let Some(id) = todo.pop() {
                if id.0 == 0 || !seen.insert(id) {
                    continue;
                }
                *work += 1;
                if *work > 32768 {
                    return Err(ActorError::WorkLimit);
                }
                let s = self.state(id).ok_or(ActorError::InvalidState)?;
                todo.push(s.next);
                match s.action {
                    ActorAction::Probe(ProbeAction::Enter(next)) => todo.push(next),
                    ActorAction::Probe(ProbeAction::SpawnTarget(child)) => {
                        self.admit_type(child, sectors, path, done, work, admission)?
                    }
                    _ => {}
                }
            }
        }
        path.remove(&ty);
        done.insert(ty);
        Ok(())
    }
    pub(crate) fn validate_type_combat(
        &self,
        ty: ActorTypeId,
        sectors: &[crate::SectorState],
    ) -> Result<(), ActorError> {
        self.admit_type(
            ty,
            sectors,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut 0,
            Admission {
                flags: ALLOWED_FLAGS | 524288,
                look: false,
                chase: false,
                attacks: false,
                shotgun: false,
                chaingun: false,
                demon: false,
                barrel: false,
                rocket: false,
            },
        )
    }
    pub(crate) fn validate_type_projectile(
        &self,
        ty: ActorTypeId,
        sectors: &[crate::SectorState],
    ) -> Result<(), ActorError> {
        self.admit_type(
            ty,
            sectors,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut 0,
            Admission {
                flags: ALLOWED_FLAGS | 524288 | if ty.0 == 34 { 65536 } else { 0 },
                look: false,
                chase: false,
                attacks: false,
                shotgun: false,
                chaingun: false,
                demon: false,
                barrel: false,
                rocket: false,
            },
        )
    }
    pub(crate) fn validate_type_perception(
        &self,
        ty: ActorTypeId,
        sectors: &[crate::SectorState],
    ) -> Result<(), ActorError> {
        let info = self.actor_type(ty).ok_or(ActorError::InvalidType)?;
        // A_Look's see-state action runs immediately. It must never call Look,
        // Chase, probes or any unsupported callback recursively.
        self.validate_state(info.states()[1])?;
        if self.demand(info.states()[1], 0)?.work >= ENTRY_LIMIT {
            return Err(ActorError::WorkLimit);
        }
        self.admit_type(
            ty,
            sectors,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut 0,
            Admission {
                flags: ALLOWED_FLAGS,
                look: true,
                chase: false,
                attacks: false,
                shotgun: false,
                chaingun: false,
                demon: false,
                barrel: false,
                rocket: false,
            },
        )
    }
    pub(crate) fn validate_state_pursuit(&self, state: StateId) -> Result<(), ActorError> {
        self.validate_state_with(state, true, true, false, false, SpecialActions::None)
    }
    pub(crate) fn validate_type_pursuit(
        &self,
        ty: ActorTypeId,
        sectors: &[crate::SectorState],
    ) -> Result<(), ActorError> {
        let info = self.actor_type(ty).ok_or(ActorError::InvalidType)?;
        self.validate_state_pursuit(info.states()[1])?;
        self.admit_type(
            ty,
            sectors,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut 0,
            Admission {
                flags: ALLOWED_FLAGS | 128 | 2097152,
                look: true,
                chase: true,
                attacks: false,
                shotgun: false,
                chaingun: false,
                demon: false,
                barrel: false,
                rocket: false,
            },
        )
    }
    pub(crate) fn validate_state_monsters(&self, state: StateId) -> Result<(), ActorError> {
        self.validate_state_with(state, true, true, true, false, SpecialActions::None)
    }
    pub(crate) fn validate_type_monsters(
        &self,
        ty: ActorTypeId,
        sectors: &[crate::SectorState],
    ) -> Result<(), ActorError> {
        self.admit_type(
            ty,
            sectors,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut 0,
            Admission {
                flags: ALLOWED_FLAGS | 64 | 128 | 524288 | 2097152,
                look: true,
                chase: true,
                attacks: true,
                shotgun: false,
                chaingun: false,
                demon: false,
                barrel: false,
                rocket: false,
            },
        )?;
        for state in self.actor_type(ty).unwrap().states() {
            self.validate_state_monsters(state)?;
        }
        Ok(())
    }
    pub(crate) fn validate_state_shotgunner(&self, state: StateId) -> Result<(), ActorError> {
        self.validate_state_with(state, true, true, false, true, SpecialActions::None)
    }
    pub(crate) fn validate_type_shotgunner(
        &self,
        ty: ActorTypeId,
        sectors: &[crate::SectorState],
    ) -> Result<(), ActorError> {
        self.admit_type(
            ty,
            sectors,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut 0,
            Admission {
                flags: ALLOWED_FLAGS | 64 | 128 | 524288 | 2097152,
                look: true,
                chase: true,
                attacks: false,
                shotgun: true,
                chaingun: false,
                demon: false,
                barrel: false,
                rocket: false,
            },
        )?;
        for state in self.actor_type(ty).unwrap().states() {
            self.validate_state_shotgunner(state)?;
        }
        Ok(())
    }
    pub(crate) fn validate_state_chaingunner(&self, state: StateId) -> Result<(), ActorError> {
        self.validate_state_with(state, true, true, false, false, SpecialActions::Chaingunner)
    }
    pub(crate) fn validate_type_chaingunner(
        &self,
        ty: ActorTypeId,
        sectors: &[crate::SectorState],
    ) -> Result<(), ActorError> {
        self.admit_type(
            ty,
            sectors,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut 0,
            Admission {
                flags: ALLOWED_FLAGS | 64 | 128 | 524288 | 2097152,
                look: true,
                chase: true,
                attacks: false,
                shotgun: false,
                chaingun: true,
                demon: false,
                barrel: false,
                rocket: false,
            },
        )?;
        for state in self.actor_type(ty).unwrap().states() {
            self.validate_state_chaingunner(state)?;
        }
        Ok(())
    }
    /// Shared original monster callbacks; per-type closures are proved by the
    /// original-ID registry policy, so this admits the union of attack actions.
    pub(crate) fn validate_state_original_monsters(
        &self,
        state: StateId,
    ) -> Result<(), ActorError> {
        self.validate_state_with(
            state,
            true,
            true,
            false,
            false,
            SpecialActions::OriginalMonsters,
        )
    }
    /// The barrel's state closure (M3h9b note §2.1): `S_BAR1`/`S_BAR2` without an
    /// action, the `S_BEXP` chain with `A_Scream` and `A_Explode` (in `S_BEXP4`
    /// only), no look, chase or attack.
    pub(crate) fn validate_state_barrel(&self, state: StateId) -> Result<(), ActorError> {
        self.validate_state_with(state, false, false, false, false, SpecialActions::Barrel)
    }
    /// The rocket's state closure (M4a plan §E.4): `S_ROCKET` and the
    /// `S_EXPLODE` chain, `A_Explode` in `S_EXPLODE1` only.
    pub(crate) fn validate_state_rocket(&self, state: StateId) -> Result<(), ActorError> {
        self.validate_state_with(state, false, false, false, false, SpecialActions::Rocket)
    }
    pub(crate) fn validate_state_demon_melee(&self, state: StateId) -> Result<(), ActorError> {
        self.validate_state_with(state, true, true, false, false, SpecialActions::Demon)
    }
    pub(crate) fn validate_type_demon_melee(
        &self,
        ty: ActorTypeId,
        sectors: &[crate::SectorState],
    ) -> Result<(), ActorError> {
        self.validate_type_demon_context(ty, sectors, false)
    }
    pub(crate) fn validate_type_demon_context(
        &self,
        ty: ActorTypeId,
        sectors: &[crate::SectorState],
        count_kill: bool,
    ) -> Result<(), ActorError> {
        self.admit_type(
            ty,
            sectors,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut 0,
            Admission {
                flags: ALLOWED_FLAGS
                    | 64
                    | 128
                    | 524288
                    | 2097152
                    | if count_kill && ty == ActorTypeId(136) {
                        4194304
                    } else {
                        0
                    },
                look: true,
                chase: true,
                attacks: false,
                shotgun: false,
                chaingun: false,
                demon: true,
                barrel: false,
                rocket: false,
            },
        )?;
        for state in self.actor_type(ty).unwrap().states() {
            self.validate_state_demon_melee(state)?;
        }
        Ok(())
    }
    pub(crate) fn validate_type_original(
        &self,
        ty: ActorTypeId,
        sectors: &[crate::SectorState],
    ) -> Result<(), ActorError> {
        if crate::original_registry::is_fireball(ty)
            || crate::original_registry::is_bruiser_shot(ty)
        {
            // Caller already required the PlayerMonstersImpV1 role (the Imp's
            // fireball) or wire 24's (the Baron's, the same flags and an action-free
            // closure). MF_MISSILE is admitted only here, for the types whose
            // lifecycle the checked pass owns.
            self.admit_type(
                ty,
                sectors,
                &mut BTreeSet::new(),
                &mut BTreeSet::new(),
                &mut 0,
                Admission {
                    flags: ALLOWED_FLAGS | crate::original_player::MF_MISSILE,
                    look: false,
                    chase: false,
                    attacks: false,
                    shotgun: false,
                    chaingun: false,
                    demon: false,
                    barrel: false,
                    rocket: false,
                },
            )?;
            return Ok(());
        }
        if crate::original_registry::is_rocket(ty) {
            // Caller already required the Episode 1 role. MF_MISSILE as for the
            // fireball; `A_Explode` in `S_EXPLODE1` alone.
            self.admit_type(
                ty,
                sectors,
                &mut BTreeSet::new(),
                &mut BTreeSet::new(),
                &mut 0,
                Admission {
                    flags: ALLOWED_FLAGS | crate::original_player::MF_MISSILE,
                    look: false,
                    chase: false,
                    attacks: false,
                    shotgun: false,
                    chaingun: false,
                    demon: false,
                    barrel: false,
                    rocket: true,
                },
            )?;
            return Ok(());
        }
        if crate::original_registry::is_extended_monster(ty) {
            // Caller already required the policy role; the original registry
            // policy proves the exact per-type closure and raise exclusion.
            self.admit_type(
                ty,
                sectors,
                &mut BTreeSet::new(),
                &mut BTreeSet::new(),
                &mut 0,
                Admission {
                    flags: ALLOWED_FLAGS | 64 | 128 | 524288 | 2097152 | 4194304,
                    look: true,
                    chase: true,
                    attacks: false,
                    shotgun: false,
                    chaingun: true,
                    demon: true,
                    barrel: false,
                    rocket: false,
                },
            )?;
            let info = self.actor_type(ty).ok_or(ActorError::InvalidType)?;
            for state in info.states().into_iter().take(7) {
                self.validate_state_original_monsters(state)?;
            }
            return Ok(());
        }
        if ty.0 == crate::identity::OriginalActorType::MT_BARREL.ordinal() {
            // Caller already required the barrels role (M3h9b note §2.1, decision
            // F4): flags `MF_SOLID|MF_SHOOTABLE|MF_NOBLOOD`, no look, chase or attack,
            // `A_Scream` in `S_BEXP2` and `A_Explode` in `S_BEXP4` alone, and the
            // compile-time definition is the info.c record.
            crate::barrels::validate_definition(ty)?;
            self.admit_type(
                ty,
                sectors,
                &mut BTreeSet::new(),
                &mut BTreeSet::new(),
                &mut 0,
                Admission {
                    flags: crate::original_player::MF_SOLID
                        | crate::original_player::MF_SHOOTABLE
                        | crate::original_player::MF_NOBLOOD,
                    look: false,
                    chase: false,
                    attacks: false,
                    shotgun: false,
                    chaingun: false,
                    demon: false,
                    barrel: true,
                    rocket: false,
                },
            )?;
            return Ok(());
        }
        if crate::e1m1_rules::listed_decoration(ty).is_some() {
            // Caller already required the E1M1 role. A static decoration (M3h9 note
            // §2.5): flags 0 or MF_SOLID, no look, chase, attack or melee action in
            // its spawn or death graph (its spawn state has tics −1 and no action,
            // its death state is S_NULL), and the compile-time definition satisfies
            // the static predicate.
            crate::e1m1_rules::validate_listed_definition(ty)?;
            self.admit_type(
                ty,
                sectors,
                &mut BTreeSet::new(),
                &mut BTreeSet::new(),
                &mut 0,
                Admission {
                    flags: crate::original_player::MF_SOLID,
                    look: false,
                    chase: false,
                    attacks: false,
                    shotgun: false,
                    chaingun: false,
                    demon: false,
                    barrel: false,
                    rocket: false,
                },
            )?;
            return Ok(());
        }
        self.admit_type(
            ty,
            sectors,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut 0,
            Admission {
                flags: ALLOWED_FLAGS
                    | 64
                    | 128
                    | 524288
                    | 2097152
                    | if ty == crate::original_registry::owner_slot() {
                        4194304
                    } else {
                        0
                    },
                look: true,
                chase: true,
                attacks: false,
                shotgun: false,
                chaingun: false,
                demon: true,
                barrel: false,
                rocket: false,
            },
        )?;
        let info = self.actor_type(ty).ok_or(ActorError::InvalidType)?;
        for state in info.states().into_iter().take(7) {
            self.validate_state_demon_melee(state)?;
        }
        Ok(())
    }
    pub(crate) fn validate_impact_source(&self) -> Result<(), crate::CombatError> {
        let mut todo = Vec::new();
        let mut seen = BTreeSet::new();
        for name in ["MT_PUFF", "MT_BLOOD"] {
            let id = source_type_id(name).unwrap();
            let info = self
                .actor_type(id)
                .ok_or(crate::CombatError::ImpactDefinition)?;
            if info.properties != SOURCE_TYPES[usize::from(id.0)].1 {
                return Err(crate::CombatError::ImpactDefinition);
            }
            todo.push(info.states()[0]);
        }
        for name in ["S_PUFF3", "S_BLOOD2", "S_BLOOD3"] {
            todo.push(source_state_id(name).unwrap());
        }
        while let Some(id) = todo.pop() {
            if id.0 == 0 || !seen.insert(id) {
                continue;
            }
            let state = self.state(id).ok_or(crate::CombatError::ImpactDefinition)?;
            let expected = SOURCE_STATES[usize::from(id.0)].1;
            if state.sprite != expected[0] as u16
                || state.frame != expected[1] as u32
                || state.tics != expected[2]
                || state.action != ActorAction::None
                || expected[3] != 0
                || state.next.0 != expected[4] as u16
                || state.misc != [expected[5], expected[6]]
            {
                return Err(crate::CombatError::ImpactDefinition);
            }
            todo.push(state.next);
        }
        Ok(())
    }
    pub(crate) fn demand(&self, state: StateId, tics: i32) -> Result<Demand, ActorError> {
        let mut d = Demand::default();
        self.demand_entry(state, tics, &mut d, &mut BTreeSet::new())?;
        Ok(d)
    }
    fn demand_entry(
        &self,
        state: StateId,
        mut tics: i32,
        d: &mut Demand,
        path: &mut BTreeSet<ActorTypeId>,
    ) -> Result<(), ActorError> {
        let mut work = 0;
        let mut spawns = Vec::new();
        self.flow(state, &mut tics, &mut work, &mut spawns)?;
        d.work += work;
        for ty in spawns {
            d.spawns += 1;
            if d.spawns > 1024 || d.work > STEP_LIMIT {
                return Err(ActorError::WorkLimit);
            }
            if !path.insert(ty) {
                return Err(ActorError::SpawnCycle);
            }
            let t = self.actor_type(ty).ok_or(ActorError::InvalidType)?;
            let initial = self.state(t.states()[0]).ok_or(ActorError::InvalidState)?;
            if initial.tics == 1 {
                self.demand_entry(initial.next, 0, d, path)?;
            }
            path.remove(&ty);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Demand {
    pub spawns: usize,
    pub work: usize,
}
impl Demand {
    pub fn add(&mut self, other: Self) -> Result<(), ActorError> {
        self.spawns += other.spawns;
        self.work += other.work;
        if self.spawns > 1024 || self.work > STEP_LIMIT {
            Err(ActorError::WorkLimit)
        } else {
            Ok(())
        }
    }
}
/// Immutable actor state observation; mutating copies cannot change simulation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActorLife {
    /// Registered type identity.
    pub actor_type: ActorTypeId,
    /// Current state; None after explicit null-state entry. Some(StateId(0))
    /// remains distinct for direct source spawn-state assignment.
    pub state: Option<StateId>,
    /// Current state duration/countdown.
    pub tics: i32,
    /// Inert sprite ID.
    pub sprite: u16,
    /// Inert frame/fullbright bits.
    pub frame: u32,
    /// Source initialization health; controlled state death does not apply damage.
    pub health: i32,
    /// Initialized reaction time; no unsupported AI timer service.
    pub reaction_time: i32,
    /// Source retarget threshold; damage sets 100, profile 9 admits A_Chase countdown.
    pub threshold: i32,
    /// Source direction: E, NE, N, NW, W, SW, S, SE, NODIR (0..8).
    /// Initialized to EAST by source zeroed spawn; serialized only in profile 9.
    pub movedir: u8,
    /// Source walking countdown, initialized to zero; profile 9 only.
    pub movecount: i32,
    /// Full admitted source flags.
    pub flags: u32,
    /// Validated assigned identity, retained even after target retirement.
    pub target: Option<ActorId>,
    /// Controlled death-state request marker, not a P_KillMobj/corpse flag.
    pub death_requested: bool,
    /// Source spawnpoint for map THINGS, or None for parameterized typed spawn.
    pub spawnpoint: Option<doom_map::Thing>,
}
/// State-machine observations, ordered relative to existing motion/lifecycle events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActorStateEvent {
    /// Direct source spawn-state assignment (no callback executed).
    Initialized {
        /// Actor identity.
        actor: ActorId,
        /// Source type.
        actor_type: ActorTypeId,
    },
    /// State/tics/sprite/frame assigned before any action; includes null entry.
    Entered {
        /// Actor identity.
        actor: ActorId,
        /// Entered state.
        state: StateId,
    },
    /// Supported action dispatched, before side effects.
    Action {
        /// Actor identity.
        actor: ActorId,
        /// Explicit original/probe identity.
        action: ActorAction,
    },
    /// Inert source sound request, with optional global origin.
    Sound {
        /// None means source boss-global origin.
        actor: Option<ActorId>,
        /// Source sound identifier.
        sound: u16,
    },
    /// Validated target assignment; stale identities are not auto-cleared.
    Target {
        /// Owning actor.
        actor: ActorId,
        /// Assigned identity.
        target: Option<ActorId>,
    },
    /// Explicit death-state request, distinct from damage/kill semantics.
    Death(ActorId),
    /// Synthetic probe observed an unavailable target; no stale access occurred.
    TargetUnavailable(ActorId),
    /// Implicit action gameplay lookup, separate from explicit external RNG calls.
    Random {
        /// Requesting actor.
        actor: ActorId,
        /// Source lookup result.
        value: u8,
    },
}

pub(crate) trait ActorAccess {
    fn get(&self, id: ActorId) -> Option<&Actor>;
    fn get_mut(&mut self, id: ActorId) -> Option<&mut Actor>;
    fn spawn(&mut self, actor: Actor) -> Result<ActorId, StoreError>;
    fn mark(&mut self, id: ActorId) -> Result<bool, StoreError>;
    /// Prove capacity for upcoming current-state spawns before any spawn.
    fn reserve(&mut self, count: usize) -> Result<(), StoreError>;
    /// Nonpending thinkers of one type, in the staged store.
    fn live_of_type(&self, ty: ActorTypeId) -> usize;
    /// The sector-thinker store the `EV_*` starters of a crossing or a monster's
    /// door use append to (the movers policy); `None` for an access without one.
    fn movers(&mut self) -> Option<&mut dyn crate::movers::MoverAccess> {
        None
    }
    /// The nonpending actors in thinker order (`thinkercap.next` onward, removed
    /// thinkers skipped), for `EV_Teleport`'s landing search (wire 24); `None` for
    /// an access without one, which that search refuses.
    fn live_order(&self) -> Option<Vec<ActorId>> {
        None
    }
}
impl ActorAccess for &mut ThinkerStore<Actor> {
    fn get(&self, id: ActorId) -> Option<&Actor> {
        ThinkerStore::get(self, id)
    }
    fn get_mut(&mut self, id: ActorId) -> Option<&mut Actor> {
        ThinkerStore::get_mut(self, id)
    }
    fn spawn(&mut self, a: Actor) -> Result<ActorId, StoreError> {
        ThinkerStore::spawn(self, a)
    }
    fn mark(&mut self, id: ActorId) -> Result<bool, StoreError> {
        ThinkerStore::mark_remove(self, id)
    }
    fn reserve(&mut self, count: usize) -> Result<(), StoreError> {
        ThinkerStore::reserve_spawns(self, count)
    }
    fn live_of_type(&self, ty: ActorTypeId) -> usize {
        ThinkerStore::live_of_type(self, ty)
    }
    fn movers(&mut self) -> Option<&mut dyn crate::movers::MoverAccess> {
        Some(&mut **self)
    }
    fn live_order(&self) -> Option<Vec<ActorId>> {
        Some(
            self.order()
                .filter(|id| self.is_pending(*id) == Some(false))
                .collect(),
        )
    }
}
impl ActorAccess for &mut ThinkerContext<'_, Actor> {
    fn get(&self, id: ActorId) -> Option<&Actor> {
        ThinkerContext::get(self, id)
    }
    fn get_mut(&mut self, id: ActorId) -> Option<&mut Actor> {
        ThinkerContext::get_mut(self, id)
    }
    fn spawn(&mut self, a: Actor) -> Result<ActorId, StoreError> {
        ThinkerContext::spawn(self, a)
    }
    fn mark(&mut self, id: ActorId) -> Result<bool, StoreError> {
        ThinkerContext::mark_remove(self, id)
    }
    fn reserve(&mut self, count: usize) -> Result<(), StoreError> {
        ThinkerContext::reserve_spawns(self, count)
    }
    fn live_of_type(&self, ty: ActorTypeId) -> usize {
        self.store().live_of_type(ty)
    }
    fn movers(&mut self) -> Option<&mut dyn crate::movers::MoverAccess> {
        Some(&mut **self)
    }
    fn live_order(&self) -> Option<Vec<ActorId>> {
        let store = self.store();
        Some(
            store
                .order()
                .filter(|id| store.is_pending(*id) == Some(false))
                .collect(),
        )
    }
}
pub(crate) fn make_actor(
    world: &StaticWorld,
    sectors: crate::staging::Sectors<'_>,
    registry: &ActorRegistry,
    options: ActorOptions,
    ty: ActorTypeId,
    position: [Fixed; 2],
    height: SpawnHeight,
) -> Actor {
    let info = registry.actor_type(ty).expect("admitted type");
    let initial = registry.state(info.states()[0]).expect("validated state");
    let flags = info.flags();
    let location = world.locate_validated(position);
    let sector = sectors[usize::from(location.sector)];
    let z = match height {
        SpawnHeight::Floor => sector.floor,
        SpawnHeight::Ceiling => sector.ceiling - info.height(),
        SpawnHeight::Absolute(z) => z,
    };
    Actor {
        spec: ActorSpec {
            position,
            radius: info.radius(),
            height: info.height(),
            ..Default::default()
        },
        floor: sector.floor,
        ceiling: sector.ceiling,
        z,
        lastlook: 0,
        solidity: if flags & 2 != 0 {
            Solidity::Solid
        } else {
            Solidity::NonSolid
        },
        momz: Fixed::ZERO,
        gravity: if flags & 512 != 0 {
            Gravity::None
        } else {
            Gravity::Normal
        },
        allow_dropoff: flags & 1024 != 0,
        location: Some(location),
        life: Some(ActorLife {
            actor_type: ty,
            state: Some(info.states()[0]),
            tics: initial.tics,
            sprite: initial.sprite,
            frame: initial.frame,
            health: info.spawn_health(),
            reaction_time: if options.skill == Skill::Nightmare {
                0
            } else {
                info.reaction_time()
            },
            flags,
            threshold: 0,
            movedir: 0,
            movecount: 0,
            target: None,
            death_requested: false,
            spawnpoint: None,
        }),
    }
}
pub(crate) struct LifeHost<'a, A> {
    pub statistics: Option<&'a mut crate::LevelStatistics>,
    pub player: Option<&'a mut crate::original_player::PlayerRuntime>,
    pub perception: Option<crate::perception::PerceptionInputs<'a>>,
    pub access: A,
    pub registry: &'a ActorRegistry,
    pub world: &'a StaticWorld,
    pub sectors: crate::staging::Sectors<'a>,
    pub options: ActorOptions,
    pub rng: &'a mut DoomRng,
    pub spatial: &'a mut crate::movement::SpatialIndex,
    pub bindings: &'a mut [Option<ActorId>; 4],
    pub events: &'a mut Vec<crate::Event>,
    /// The staged switch buttons a hitscan into a gun-triggered line (GR 46,
    /// Episode 1 policy) starts through `P_ShootSpecialLine`; `None` wherever no
    /// such shot can run.
    pub buttons: Option<&'a mut crate::use_lines::Buttons>,
    /// The staged runtime sector specials, read by the Episode 1 policy's
    /// special-11 damage cap (p_inter.c:837–842); `None` wherever the player
    /// body cannot be damaged under that policy.
    pub specials: Option<&'a [crate::sector_thinkers::SectorSpecial]>,
}
impl ThinkerStore<Actor> {
    pub(crate) fn live_of_type(&self, ty: ActorTypeId) -> usize {
        self.order()
            .filter(|id| {
                self.is_pending(*id) == Some(false)
                    && self
                        .get(*id)
                        .and_then(Actor::life)
                        .is_some_and(|l| l.actor_type == ty)
            })
            .count()
    }
}
impl<A: ActorAccess> LifeHost<'_, A> {
    pub(crate) fn sight_failed(&self) -> bool {
        self.perception
            .and_then(|p| p.checked)
            .is_some_and(|c| c.failure.get().is_some())
    }
    pub(crate) fn fail_sight(&self, error: crate::SimError) {
        self.perception
            .and_then(|p| p.checked)
            .expect("fallible source actions require a bounded operation plan")
            .failure
            .set(Some(error));
    }

    fn observe(&mut self, event: ActorStateEvent) {
        self.events.push(crate::Event::ActorState(event));
    }
    pub fn spawn(&mut self, ty: ActorTypeId, position: [Fixed; 2], height: SpawnHeight) -> ActorId {
        let actor = make_actor(
            self.world,
            self.sectors,
            self.registry,
            self.options,
            ty,
            position,
            height,
        );
        let id = self
            .access
            .spawn(actor)
            .expect("preflight reserved actor capacity");
        self.access.get_mut(id).expect("new typed actor").lastlook = self.rng.p_random() % 4;
        self.spatial.link(id);
        self.events.push(crate::Event::Spawned(id));
        self.observe(ActorStateEvent::Initialized {
            actor: id,
            actor_type: ty,
        });
        id
    }
    pub fn mark(&mut self, id: ActorId) {
        if self.access.mark(id).expect("validated callback identity") {
            self.spatial.unlink(id);
            for binding in &mut *self.bindings {
                if *binding == Some(id) {
                    *binding = None;
                }
            }
            self.events.push(crate::Event::RemovalMarked(id));
        }
    }
    pub fn target(&mut self, id: ActorId, target: Option<ActorId>) {
        self.access
            .get_mut(id)
            .expect("live reference owner")
            .life
            .as_mut()
            .expect("typed actor")
            .target = target;
        self.observe(ActorStateEvent::Target { actor: id, target });
    }
    fn draw(&mut self, id: ActorId) -> u8 {
        let value = self.rng.p_random();
        self.observe(ActorStateEvent::Random { actor: id, value });
        value
    }
    pub(crate) fn sound(&mut self, actor: Option<ActorId>, sound: u16) {
        self.observe(ActorStateEvent::Sound { actor, sound });
    }
    pub(crate) fn enter_counted(&mut self, id: ActorId, state: StateId, work: &mut usize) {
        self.enter_inner(id, state, work);
    }
    pub(crate) fn invoke_chase(&mut self, id: ActorId) {
        self.observe(ActorStateEvent::Action {
            actor: id,
            action: ActorAction::Original(source_action_id("A_Chase")),
        });
        crate::pursuit::chase::chase(self, id, &mut 0);
    }
    pub(crate) fn invoke_look(&mut self, id: ActorId) -> bool {
        self.observe(ActorStateEvent::Action {
            actor: id,
            action: ActorAction::Original(source_action_id("A_Look")),
        });
        crate::perception::look(self, id, &mut 0)
    }
    pub fn enter(&mut self, id: ActorId, state: StateId) {
        self.enter_inner(id, state, &mut 0);
    }
    fn enter_inner(&mut self, id: ActorId, mut state: StateId, work: &mut usize) {
        loop {
            if self.sight_failed() {
                return;
            }
            *work += 1;
            assert!(*work <= ENTRY_LIMIT, "validated state-entry bound");
            self.observe(ActorStateEvent::Entered { actor: id, state });
            if state.0 == 0 {
                self.access
                    .get_mut(id)
                    .expect("pending storage remains readable")
                    .life
                    .as_mut()
                    .unwrap()
                    .state = None;
                self.mark(id);
                return;
            }
            let definition = self.registry.state(state).expect("validated state");
            let (tics, sprite, frame, action, next) = (
                definition.tics,
                definition.sprite,
                definition.frame,
                definition.action,
                definition.next,
            );
            {
                let life = self
                    .access
                    .get_mut(id)
                    .expect("typed actor")
                    .life
                    .as_mut()
                    .unwrap();
                life.state = Some(state);
                life.tics = tics;
                life.sprite = sprite;
                life.frame = frame;
            }
            if action != ActorAction::None {
                self.observe(ActorStateEvent::Action { actor: id, action });
                self.action(id, action, work);
            }
            if self
                .access
                .get(id)
                .expect("deferred storage")
                .life
                .unwrap()
                .tics
                != 0
            {
                return;
            }
            state = next;
        }
    }
    fn action(&mut self, id: ActorId, action: ActorAction, work: &mut usize) {
        use crate::{actions::OriginalAction, player_monsters::Hitscan};
        match action {
            ActorAction::None => {}
            ActorAction::Original(action) => {
                let actor = *self.access.get(id).expect("typed actor");
                let life = actor.life.unwrap();
                let info = self.registry.actor_type(life.actor_type).unwrap();
                let sound_id = |name: &str| {
                    SOURCE_SOUNDS
                        .iter()
                        .position(|s| *s == name)
                        .expect("original sound") as u16
                };
                match OriginalAction::of(action) {
                    Some(OriginalAction::FaceTarget) => {
                        assert!(self.perception.is_some_and(|p| p.monsters));
                        crate::monsters::face(self, id);
                    }
                    Some(OriginalAction::SargAttack) => {
                        assert!(self.perception.is_some_and(|p| p.demon));
                        crate::demon::strike(self, id);
                    }
                    // Former humans in PlayerMonstersV1 share one current-state
                    // hitscan implementation; historical profiles keep their
                    // preflighted single-owner plans unchanged.
                    Some(OriginalAction::CPosAttack)
                        if crate::player_monsters::uses_current_monsters(self) =>
                    {
                        crate::player_monsters::attack(self, id, Hitscan::Chaingun);
                    }
                    Some(OriginalAction::CPosRefire)
                        if crate::player_monsters::uses_current_monsters(self) =>
                    {
                        crate::player_monsters::refire(self, id, work);
                    }
                    Some(OriginalAction::SPosAttack)
                        if crate::player_monsters::uses_current_monsters(self) =>
                    {
                        crate::player_monsters::attack(self, id, Hitscan::Shotgun);
                    }
                    Some(OriginalAction::PosAttack)
                        if crate::player_monsters::uses_current_monsters(self) =>
                    {
                        crate::player_monsters::attack(self, id, Hitscan::Pistol);
                    }
                    Some(OriginalAction::TroopAttack) => {
                        assert!(crate::player_monsters::uses_current_monsters(self));
                        crate::player_monsters::troop_attack(self, id);
                    }
                    // Wire 24 (M4b step 5): the Baron's role admits both, each at
                    // its one state.
                    Some(OriginalAction::BruisAttack) => {
                        assert!(crate::player_monsters::uses_current_monsters(self));
                        crate::player_monsters::bruis_attack(self, id);
                    }
                    Some(OriginalAction::BossDeath) => crate::boss::boss_death(self, id),
                    Some(OriginalAction::CPosAttack) => {
                        assert!(self.perception.is_some_and(|p| p.chaingunner));
                        crate::chaingunner::attack(self, id);
                    }
                    Some(OriginalAction::CPosRefire) => {
                        assert!(self.perception.is_some_and(|p| p.chaingunner));
                        crate::chaingunner::refire(self, id, work);
                    }
                    Some(OriginalAction::SPosAttack) => {
                        assert!(self.perception.is_some_and(|p| p.shotgunner));
                        crate::shotgunner::fire(self, id);
                    }
                    Some(OriginalAction::PosAttack) => {
                        assert!(self.perception.is_some_and(|p| p.monsters));
                        crate::monsters::pos_attack(self, id);
                    }
                    Some(OriginalAction::Chase) => {
                        assert!(self.perception.is_some_and(|p| p.pursuit));
                        crate::pursuit::chase::chase(self, id, work);
                    }
                    Some(OriginalAction::Look) => {
                        crate::perception::look(self, id, work);
                    }
                    Some(OriginalAction::Fall) => {
                        let actor = self.access.get_mut(id).unwrap();
                        actor.life.as_mut().unwrap().flags &= !2;
                        actor.solidity = Solidity::NonSolid;
                    }
                    Some(OriginalAction::Pain) => {
                        let sound = info.sounds()[2];
                        if sound != 0 {
                            self.sound(Some(id), sound);
                        }
                    }
                    Some(OriginalAction::XScream) => self.sound(Some(id), sound_id("sfx_slop")),
                    Some(OriginalAction::PlayerScream) => self.sound(
                        Some(id),
                        sound_id(if life.health < -50 {
                            "sfx_pdiehi"
                        } else {
                            "sfx_pldeth"
                        }),
                    ),
                    Some(OriginalAction::Scream) => {
                        let sound = info.sounds()[3];
                        if sound == 0 {
                            return;
                        }
                        let global = source_type_is(life.actor_type, "MT_SPIDER")
                            || source_type_is(life.actor_type, "MT_CYBORG");
                        let pod = sound_id("sfx_podth1");
                        let bg = sound_id("sfx_bgdth1");
                        let chosen = if (pod..=pod + 2).contains(&sound) {
                            pod + u16::from(self.draw(id) % 3)
                        } else if (bg..=bg + 1).contains(&sound) {
                            bg + u16::from(self.draw(id) % 2)
                        } else {
                            sound
                        };
                        self.sound(if global { None } else { Some(id) }, chosen);
                    }
                    // M3h9b decision F4(ii): admitted for MT_BARREL's S_BEXP4 alone.
                    Some(OriginalAction::Explode) => crate::barrels::explode(self, id),
                    _ => unreachable!("unsupported action rejected before mutation"),
                }
            }
            ActorAction::Probe(probe) => match probe {
                ProbeAction::Random => {
                    self.draw(id);
                }
                ProbeAction::Enter(state) => self.enter_inner(id, state, work),
                ProbeAction::RemoveSelf => self.mark(id),
                ProbeAction::TargetSelf => self.target(id, Some(id)),
                ProbeAction::RemoveTarget => {
                    let target = self.access.get(id).unwrap().life.unwrap().target;
                    if let Some(target) = target.filter(|target| self.access.get(*target).is_some())
                    {
                        self.mark(target);
                    } else {
                        self.observe(ActorStateEvent::TargetUnavailable(id));
                    }
                }
                ProbeAction::SpawnTarget(ty) => {
                    let actor = *self.access.get(id).unwrap();
                    let child = self.spawn(ty, actor.spec.position, SpawnHeight::Absolute(actor.z));
                    self.target(id, Some(child));
                    self.target(child, Some(id));
                }
            },
        }
    }
    pub fn tick(&mut self, id: ActorId) {
        let life = self
            .access
            .get_mut(id)
            .expect("live thinker")
            .life
            .as_mut()
            .expect("typed actor");
        if life.tics == -1 {
            return;
        }
        life.tics -= 1;
        if life.tics == 0 {
            let next = self
                .registry
                .state(life.state.expect("nonremoved state"))
                .unwrap()
                .next;
            self.enter(id, next);
        }
    }
}
pub(crate) fn blocks_cell_index(actor: &Actor) -> bool {
    actor.life.is_none_or(|life| life.flags & 16 == 0)
}
pub(crate) fn in_sector_list(actor: &Actor) -> bool {
    actor.life.is_none_or(|life| life.flags & 8 == 0)
}

pub(crate) fn source_sound_id(name: &str) -> u16 {
    SOURCE_SOUNDS
        .iter()
        .position(|s| *s == name)
        .expect("source sound") as u16
}
pub(crate) fn source_action_id(name: &str) -> ActionId {
    ActionId(
        SOURCE_ACTIONS
            .iter()
            .position(|s| *s == name)
            .expect("source action") as u16,
    )
}
