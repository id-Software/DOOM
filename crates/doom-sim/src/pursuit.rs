//! Non-attacking source pursuit, using the existing collision and lifecycle host.
use crate::lifecycle::{ActorAccess, LifeHost};
use crate::*;
use std::{error::Error, fmt};
pub(crate) mod chase;
mod directions;
use directions::new_direction;
const NODIR: u8 = 8;
const FLAGS: u32 = 2 | 4 | 32 | 64 | 128 | 1024 | 2097152 | 2048 | 33554432;
/// Pursuit admission or preflight rejection, before authoritative mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PursuitError {
    /// Explicit profile 9 is required.
    Profile,
    /// Unsupported actor, action closure, attack state or fixture definition.
    Definition,
    /// Direction is outside source 0..8.
    Direction,
    /// Arithmetic margin or countdown cannot be proved safe.
    Arithmetic,
    /// Target is absent for NewChaseDir, pending, retired or stale.
    Target,
    /// Door/crossing or sector specials require a later milestone.
    Specials,
}
impl fmt::Display for PursuitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pursuit: {self:?}")
    }
}
impl Error for PursuitError {}
/// Explicit between-tic fixture setup, not a player-damage API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PursuitSetup {
    /// Source direction, checked before indexing tables.
    pub movedir: u8,
    /// Walking countdown.
    pub movecount: i32,
    /// Source reaction counter.
    pub reaction_time: i32,
    /// Target retention threshold.
    pub threshold: i32,
    /// Actor health, distinct from the perception player record.
    pub health: i32,
    /// Supported current source flags, independent of immutable type defaults.
    pub flags: u32,
    /// Unsigned BAM angle.
    pub angle: u32,
}
impl PursuitSetup {
    /// Copy the supported fields of an existing typed actor for fixture edits.
    pub fn from_actor(actor: &Actor) -> Option<Self> {
        let a = actor.life()?;
        Some(Self {
            movedir: a.movedir,
            movecount: a.movecount,
            reaction_time: a.reaction_time,
            threshold: a.threshold,
            health: a.health,
            flags: a.flags,
            angle: actor.angle(),
        })
    }
}
/// Source call identities in chronological observations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PursuitOperation {
    /// P_Move.
    Move = 0,
    /// P_TryWalk.
    TryWalk = 1,
    /// P_NewChaseDir.
    NewChaseDir = 2,
    /// A_Chase.
    Chase = 3,
}
/// Ordered source observations; no observation is an input to pursuit decisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PursuitEvent {
    /// Enter an original operation.
    Start {
        /// Actor identity.
        actor: ActorId,
        /// Source function.
        operation: PursuitOperation,
    },
    /// Return; result is meaningful for Move and TryWalk only.
    Return {
        /// Actor identity.
        actor: ActorId,
        /// Source function.
        operation: PursuitOperation,
        /// Source boolean, or zero for void routines.
        result: bool,
    },
    /// A source direction assignment, before its walking attempt.
    Direction {
        /// Actor identity.
        actor: ActorId,
        /// New source direction.
        direction: u8,
    },
    /// A source gameplay draw at its original position.
    Random {
        /// Actor identity.
        actor: ActorId,
        /// 0=walk count,1=axis priority,2=fallback order,3=active sound.
        kind: u8,
        /// Original lookup value.
        value: u8,
    },
}
fn observe<A: ActorAccess>(h: &mut LifeHost<'_, A>, e: PursuitEvent) {
    h.events.push(Event::Pursuit(e));
}
fn start<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId, operation: PursuitOperation) {
    observe(
        h,
        PursuitEvent::Start {
            actor: id,
            operation,
        },
    );
}
fn end<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    operation: PursuitOperation,
    result: bool,
) {
    observe(
        h,
        PursuitEvent::Return {
            actor: id,
            operation,
            result,
        },
    );
}
fn direction<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId, d: u8) {
    h.access.get_mut(id).unwrap().life.as_mut().unwrap().movedir = d;
    observe(
        h,
        PursuitEvent::Direction {
            actor: id,
            direction: d,
        },
    );
}
fn draw<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId, kind: u8) -> u8 {
    let value = h.rng.p_random();
    observe(
        h,
        PursuitEvent::Random {
            actor: id,
            kind,
            value,
        },
    );
    value
}
/// Genuine supported P_Move path: the shared query retains thing/line order.
pub(crate) fn move_actor<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) -> bool {
    start(h, id, PursuitOperation::Move);
    let actor = *h.access.get(id).unwrap();
    let life = actor.life.unwrap();
    if life.movedir == NODIR {
        end(h, id, PursuitOperation::Move, false);
        return false;
    }
    let speed = h.registry.actor_type(life.actor_type).unwrap().speed();
    let vector = MoveDirection::from_raw(life.movedir).unwrap().components();
    let target = std::array::from_fn(|i| actor.position()[i] + Fixed::from_bits(speed * vector[i]));
    let attempt = multisector::check_move(
        (h.world, h.sectors),
        id,
        &actor,
        target,
        h.spatial,
        &|id| h.access.get(id).copied(),
        false,
    );
    // The movers policy's spechit (empty for every other policy's view).
    let Ok(hit) = line_triggers::spechit(h.sectors, &attempt, &[]) else {
        h.events.push(Event::SectorMovement { actor: id, attempt });
        h.fail_sight(MapScaleError::SpecialCross.into());
        end(h, id, PursuitOperation::Move, false);
        return false;
    };
    let allowed = attempt.allowed();
    if allowed {
        let a = h.access.get_mut(id).unwrap();
        a.spec.position = target;
        a.floor = attempt.floor;
        a.ceiling = attempt.ceiling;
        a.location = Some(attempt.location);
        h.spatial.link(id);
    }
    h.events.push(Event::SectorMovement { actor: id, attempt });
    if allowed && !hit.is_empty() {
        cross_lines(h, id, &hit, actor.position(), target);
    }
    if allowed {
        // p_enemy.c:319–328, after P_TryMove has returned, so after its crossing
        // loop (p_map.c:500–514): `MF_INFLOAT` cleared and `z = floorz` (no admitted
        // floater). `EV_Teleport` inside that loop reads the pre-move z for fog 1
        // (p_telept.c:100–111; M4b step 14, review finding F1), and a teleport has set
        // `floorz` to the landing's (p_map.c:169). Nothing before wire 24 reads the
        // crosser's z or flags in the loop: a mover `Starter` holds no actor store and
        // `teleport::cross` refuses before reading the actor unless the world teleports,
        // so every older policy ends with the same record as before this order.
        let a = h.access.get_mut(id).unwrap();
        a.z = a.floor;
        a.life.as_mut().unwrap().flags &= !2097152;
    }
    // p_enemy.c:310–324 (no admitted floater): a failed move with special lines in
    // spechit resets movedir and uses each of them, in reverse, from side 0.
    let result = allowed || (!hit.is_empty() && use_lines_blocked(h, id, &hit));
    end(h, id, PursuitOperation::Move, result);
    result
}
/// The `P_Move` failure branch with `numspechit > 0`: `movedir = DI_NODIR` (no
/// direction event: the source assigns it outside any observed call), then
/// `P_UseSpecialLine(actor, line, 0)` for every entry in reverse; true if any use
/// returned true. A monster's DR use always returns true, so pressing against a door
/// succeeds every time; any other special line still resets `movedir`.
fn use_lines_blocked<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    hit: &line_triggers::SpecHit,
) -> bool {
    h.events.push(Event::Mover(MoverEvent::SpechitBlocked {
        actor: id,
        count: hit.len() as u8,
    }));
    h.access.get_mut(id).unwrap().life.as_mut().unwrap().movedir = NODIR;
    let mut good = false;
    for &line in hit.lines().iter().rev() {
        let special = h.sectors.line_special(line);
        let Some(store) = h.access.movers() else {
            h.fail_sight(StoreError::UnsupportedThinker.into());
            return false;
        };
        let mut starter = movers::Starter {
            world: h.world,
            sectors: h.sectors,
            store,
            events: &mut *h.events,
            specials: None,
            player: None,
        };
        match use_lines::use_special_line(
            &mut starter,
            None,
            id,
            use_lines::User::Monster,
            line,
            0,
            special,
            &mut use_lines::UseOutcome::default(),
        ) {
            Ok(used) => good |= used,
            Err(error) => {
                h.fail_sight(error);
                return false;
            }
        }
    }
    good
}
/// The crossing loop of a monster's successful `P_TryMove` (a monster never clears
/// a line, so the view stays current); a refused starter refuses the tic.
fn cross_lines<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    hit: &line_triggers::SpecHit,
    old: [Fixed; 2],
    new: [Fixed; 2],
) {
    for crossing in line_triggers::crossings(h.world, h.sectors, hit, old, new, &[]) {
        // Wire 24: `EV_Teleport` inside the loop (p_map.c:511); once it reached
        // `P_TeleportMove`, `numspechit` is 0 and the loop ends (p_map.c:152).
        if crossing.special == teleport::LINE_TELEPORT {
            match teleport::cross(h, id, line_triggers::Crosser::Monster, crossing) {
                Ok(true) => return,
                Ok(false) => continue,
                Err(error) => {
                    h.fail_sight(error);
                    return;
                }
            }
        }
        let Some(store) = h.access.movers() else {
            h.fail_sight(StoreError::UnsupportedThinker.into());
            return;
        };
        let mut starter = movers::Starter {
            world: h.world,
            sectors: h.sectors,
            store,
            events: &mut *h.events,
            specials: None,
            player: None,
        };
        if let Err(error) =
            line_triggers::cross(&mut starter, id, line_triggers::Crosser::Monster, crossing)
        {
            h.fail_sight(error);
            return;
        }
    }
}
pub(crate) fn try_walk<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) -> bool {
    start(h, id, PursuitOperation::TryWalk);
    let allowed = move_actor(h, id);
    if allowed {
        let count = i32::from(draw(h, id, 0) & 15);
        h.access
            .get_mut(id)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .movecount = count;
    }
    end(h, id, PursuitOperation::TryWalk, allowed);
    allowed
}

impl Simulation {
    /// Profile 9 with exact original metadata; original monsters remain unadmitted.
    pub fn monster_pursuit(map: Map) -> Result<Self, SimError> {
        Self::with_pursuit_registry(map, 64, ActorRegistry::vanilla(), ActorOptions::default())
    }
    /// Explicit labelled native look/chase fixtures; no attacks, float AI or specials.
    pub fn with_pursuit_registry(
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
        let mut sim = Self::with_actor_registry(map, limit, registry, options)?;
        sim.profile = Profile::Pursuit;
        sim.sound_targets = vec![None; sim.sectors.len()];
        Ok(sim)
    }
    pub(crate) fn validate_pursuit_type(&self, ty: ActorTypeId) -> Result<(), SimError> {
        if ![0, 19, 21, 136].contains(&ty.0) {
            return Err(PursuitError::Definition.into());
        }
        let r = self.registry.as_deref().unwrap();
        let info = r.actor_type(ty).ok_or(PursuitError::Definition)?;
        if (ty != ActorTypeId(0) && info.states()[3..5] != [StateId(0); 2])
            || info.flags() & !FLAGS != 0
        {
            return Err(PursuitError::Definition.into());
        }
        if ty != ActorTypeId(0) && info.states()[..2] != [StateId(1), StateId(2)] {
            return Err(PursuitError::Definition.into());
        }
        r.validate_type_pursuit(ty, &self.sectors)?;
        Ok(())
    }
    fn pursuit_actor(&self, id: ActorId, margin: u32) -> Result<Actor, SimError> {
        if self.profile.uses_monsters() {
            return self.monster_actor(id, margin);
        }
        if self.profile != Profile::Pursuit {
            return Err(PursuitError::Profile.into());
        }
        let a = self.perception_actor(id)?;
        let life = a.life.unwrap();
        self.validate_pursuit_type(life.actor_type)?;
        if MoveDirection::from_raw(life.movedir).is_none() {
            return Err(PursuitError::Direction.into());
        }
        if life.reaction_time == i32::MIN + 1
            || [life.movecount, life.reaction_time, life.threshold].contains(&i32::MIN)
            || a.position()
                .iter()
                .any(|v| !crate::sight::within(*v, margin))
        {
            return Err(PursuitError::Arithmetic.into());
        }
        if life.flags & !FLAGS != 0 {
            return Err(PursuitError::Definition.into());
        }
        let native = life.actor_type != ActorTypeId(0);
        if !life
            .state
            .is_some_and(|s| (149..=173).contains(&s.0) || native && [1, 2].contains(&s.0))
        {
            return Err(PursuitError::Definition.into());
        }
        Ok(a)
    }
    fn native_pursuer(&self, id: ActorId) -> Result<Actor, SimError> {
        let a = self.pursuit_actor(id, 1008)?;
        if a.life.unwrap().actor_type == ActorTypeId(0)
            || self.profile.uses_monsters() && !self.monster_is_owner(id)
        {
            return Err(PursuitError::Definition.into());
        }
        Ok(a)
    }
    pub(crate) fn preflight_pursuit_callback(&self, id: ActorId) -> Result<(), SimError> {
        self.native_pursuer(id)?;
        self.pursuit_dependencies(id)
    }
    fn pursuit_dependencies(&self, id: ActorId) -> Result<(), SimError> {
        if self.profile.uses_monsters() {
            return self.preflight_monster_callback(id);
        }
        self.preflight_perception_roster()?;
        if let Some(target) = self.actor(id).unwrap().life.unwrap().target {
            self.validate_perception_reference(target)
                .map_err(|_| PursuitError::Target)?;
        }
        for target in self.sound_targets.iter().flatten() {
            self.validate_perception_reference(*target)
                .map_err(|_| PerceptionError::SoundTarget)?;
        }
        Ok(())
    }
    /// Between-tic setup. Invalid values preserve all canonical state.
    pub fn configure_pursuit_actor(
        &mut self,
        id: ActorId,
        setup: PursuitSetup,
    ) -> Result<(), SimError> {
        if self.profile.uses_monsters() {
            return self.configure_monster_actor(id, setup);
        }
        if self.profile != Profile::Pursuit {
            return Err(PursuitError::Profile.into());
        }
        self.perception_actor(id)?;
        if MoveDirection::from_raw(setup.movedir).is_none() {
            return Err(PursuitError::Direction.into());
        }
        if setup.reaction_time == i32::MIN + 1
            || [setup.movecount, setup.reaction_time, setup.threshold].contains(&i32::MIN)
        {
            return Err(PursuitError::Arithmetic.into());
        }
        if setup.flags & !FLAGS != 0 {
            return Err(PursuitError::Definition.into());
        }
        let a = self.actors.get_mut(id).unwrap();
        let l = a.life.as_mut().unwrap();
        l.movedir = setup.movedir;
        l.movecount = setup.movecount;
        l.reaction_time = setup.reaction_time;
        l.threshold = setup.threshold;
        l.health = setup.health;
        l.flags = setup.flags;
        a.spec.angle = setup.angle;
        a.solidity = if setup.flags & 2 != 0 {
            Solidity::Solid
        } else {
            Solidity::NonSolid
        };
        a.allow_dropoff = setup.flags & 1024 != 0;
        Ok(())
    }
    pub(crate) fn preflight_pursuit_entry(
        &self,
        id: ActorId,
        state: StateId,
    ) -> Result<(), SimError> {
        let a = self.pursuit_actor(id, 1008)?;
        if [1, 2].contains(&state.0) {
            if a.life.unwrap().actor_type == ActorTypeId(0) {
                return Err(PursuitError::Definition.into());
            }
            self.pursuit_dependencies(id)?;
        } else if !(149..=173).contains(&state.0) {
            return Err(PursuitError::Definition.into());
        }
        Ok(())
    }
    pub(crate) fn preflight_pursuit_step(&self) -> Result<(), SimError> {
        self.preflight_perception_step()?;
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            self.pursuit_actor(id, 978)?;
            self.pursuit_dependencies(id)?;
        }
        Ok(())
    }
    fn pursuit_operation(&mut self, id: ActorId, op: PursuitOperation) -> Result<bool, SimError> {
        let a = self.native_pursuer(id)?;
        if op == PursuitOperation::NewChaseDir {
            self.validate_perception_reference(a.life.unwrap().target.ok_or(PursuitError::Target)?)
                .map_err(|_| PursuitError::Target)?;
        }
        if op == PursuitOperation::Chase {
            self.pursuit_dependencies(id)?;
        }
        if self.profile.uses_checked_demons() {
            return self.checked_demon_operation(|host| match op {
                PursuitOperation::Move => move_actor(host, id),
                PursuitOperation::TryWalk => try_walk(host, id),
                PursuitOperation::NewChaseDir => {
                    new_direction(host, id);
                    false
                }
                PursuitOperation::Chase => {
                    host.invoke_chase(id);
                    false
                }
            });
        }
        let mut host = LifeHost {
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
                monsters: self.profile.uses_monsters(),
                shotgunner: self.profile == Profile::Shotgunner,
                chaingunner: self.profile == Profile::Chaingunner,
                demon: self.profile.uses_demon(),
                map_demons: self.profile.uses_demon_map(),
                checked: None,
                melee: None,
                shotgun: None,
                shot: None,
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
        };
        Ok(match op {
            PursuitOperation::Move => move_actor(&mut host, id),
            PursuitOperation::TryWalk => try_walk(&mut host, id),
            PursuitOperation::NewChaseDir => {
                new_direction(&mut host, id);
                false
            }
            PursuitOperation::Chase => {
                host.invoke_chase(id);
                false
            }
        })
    }
    /// Source P_Move: positional movement, without momentum conversion.
    pub fn move_pursuer(&mut self, id: ActorId) -> Result<bool, SimError> {
        if self.original_player.is_some() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.pursuit_operation(id, PursuitOperation::Move)
    }
    /// Source P_TryWalk: only success draws the movement count.
    pub fn try_walk(&mut self, id: ActorId) -> Result<bool, SimError> {
        if self.original_player.is_some() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.pursuit_operation(id, PursuitOperation::TryWalk)
    }
    /// Source P_NewChaseDir, requiring a valid current target.
    pub fn new_chase_direction(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.original_player.is_some() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.pursuit_operation(id, PursuitOperation::NewChaseDir)
            .map(|_| ())
    }
    /// Explicit supported A_Chase, sharing the same thinker/state callback.
    pub fn chase(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.original_player.is_some() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.pursuit_operation(id, PursuitOperation::Chase)
            .map(|_| ())
    }
}
fn validate_registry(r: &ActorRegistry) -> Result<(), SimError> {
    let source = ActorRegistry::vanilla();
    if r.states() == source.states() && r.types() == source.types() {
        return Ok(());
    }
    if r.states().len() != source.states().len() || r.types().len() != source.types().len() {
        return Err(PursuitError::Definition.into());
    }
    let see = &r.states()[2];
    if see.name != "FIXTURE_CHASE"
        || see.sprite != 29
        || see.frame != 1
        || !(1..=35).contains(&see.tics)
        || see.next != StateId(2)
        || see.misc != [0; 2]
        || see.action != ActorAction::Original(crate::lifecycle::source_action_id("A_Chase"))
    {
        return Err(PursuitError::Definition.into());
    }
    let mut projected = r.clone();
    projected.states[2].name = "FIXTURE_SEE".into();
    projected.states[2].tics = -1;
    projected.states[2].next = StateId(0);
    projected.states[2].action = ActorAction::None;
    for i in [19, 21, 136] {
        let t = &r.types()[i];
        if t.states()[3..5] != [StateId(0); 2] {
            return Err(PursuitError::Definition.into());
        }
        if !(1..=16).contains(&t.speed())
            || t.radius() < Fixed::ZERO
            || t.radius() > Fixed::from_map_units(32)
            || t.flags() & !FLAGS != 0
        {
            return Err(PursuitError::Definition.into());
        }
        for j in [11, 15, 16, 20] {
            projected.types[i].properties[j] = source.types()[0].properties[j];
        }
    }
    crate::perception::validate_registry(&projected, &source)
}

pub(crate) fn event_bytes(b: &mut Vec<u8>, e: &PursuitEvent) {
    let (kind, id) = match e {
        PursuitEvent::Start { actor, .. } => (0, *actor),
        PursuitEvent::Return { actor, .. } => (1, *actor),
        PursuitEvent::Direction { actor, .. } => (2, *actor),
        PursuitEvent::Random { actor, .. } => (3, *actor),
    };
    b.push(kind);
    b.extend(id.slot().to_le_bytes());
    b.extend(id.generation().to_le_bytes());
    match e {
        PursuitEvent::Start { operation, .. } => b.push(*operation as u8),
        PursuitEvent::Return {
            operation, result, ..
        } => b.extend([*operation as u8, u8::from(*result)]),
        PursuitEvent::Direction { direction, .. } => b.push(*direction),
        PursuitEvent::Random { kind, value, .. } => b.extend([*kind, *value]),
    }
}
