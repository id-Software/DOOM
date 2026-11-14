//! Explicit intersection of expanded static geometry and the native Demon closure.
use crate::*;

/// Which expanded-map service family a construction selects and the two
/// policy-decided admissions it needs before any type validation runs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ExpandedSelection {
    pub profile: Profile,
    /// Actor slots the proof needs (64 historically, 320 at map scale).
    pub actor_cap: u32,
    /// Admit type dimensions against the tallest sector (heights policy).
    pub type_envelope: bool,
    /// Format-sized static geometry with derived work values (real-map policy).
    pub real_map: bool,
    /// Sky ceilings and the static sector specials (specials policy).
    pub specials: bool,
    /// Doors, lifts, floors, switches and the exit (movers policy, a superset of
    /// the specials policy: `specials` is set too).
    pub movers: bool,
    /// Two-sided scrolling walls and static decorations (E1M1 policy, a superset
    /// of the movers policy: `movers` is set too).
    pub e1m1: bool,
    /// The per-map bounds of the Episode 1 policy (a superset of the E1M1 policy:
    /// `e1m1` is set too); `None` under every other policy.
    pub episode1: Option<crate::episode1::Episode1Bounds>,
    /// Line special 97 and the landing (wire 24, a superset of the Episode 1
    /// policy: `episode1` is set too); false under every other policy.
    pub teleport: bool,
}
impl Simulation {
    /// Profile 20: larger static maps with the exact map-context Demon registry.
    /// Geometry uses the extent proof; actors retain the narrower gameplay bounds.
    /// Selected THING angles require a source-defined signed BAM multiplication.
    pub fn with_demon_expanded_map_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
        context: MapContextOptions,
    ) -> Result<Self, SimError> {
        Self::with_demon_expanded_profile(
            map,
            limit,
            registry,
            options,
            context,
            ExpandedSelection {
                profile: Profile::DemonExpandedMap,
                actor_cap: 64,
                type_envelope: false,
                real_map: false,
                specials: false,
                movers: false,
                e1m1: false,
                episode1: None,
                teleport: false,
            },
        )
    }
    pub(crate) fn with_demon_expanded_profile(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
        context: MapContextOptions,
        selection: ExpandedSelection,
    ) -> Result<Self, SimError> {
        let ExpandedSelection {
            profile,
            actor_cap,
            type_envelope,
            real_map,
            specials,
            movers,
            e1m1,
            episode1,
            teleport,
        } = selection;
        if context.respawn_monsters {
            return Err(MapSpawnError::Respawn.into());
        }
        if options.mode != ActorGameMode::SinglePlayer {
            return Err(ActorError::UnsupportedMode.into());
        }
        if options.skill == Skill::Nightmare {
            return Err(MonsterError::Definition.into());
        }
        // The real-map policy takes the THING count from the lump (the parser's
        // format bound); spawned actors stay bounded by the roster.
        if !real_map && map.things().len() > MAX_DEMON_MAP_THINGS {
            return Err(MapSpawnError::WorkLimit.into());
        }
        let movers = if episode1.is_some() && teleport {
            Some(mover_rules::validate_episode1_teleport(&map)?)
        } else if episode1.is_some() {
            Some(mover_rules::validate_episode1(&map)?)
        } else if e1m1 {
            Some(mover_rules::validate_e1m1(&map)?)
        } else if movers {
            Some(mover_rules::validate_movers(&map)?)
        } else {
            if specials {
                validate_specials(&map)?;
            }
            None
        };
        if !specials
            && (map.linedefs().iter().any(|l| l.special != 0)
                || map.sectors().iter().any(|s| s.special != 0))
        {
            return Err(PursuitError::Specials.into());
        }
        // The Episode 1 policy replaces the ±512 static-height rule (the twin of
        // certificate clause (vii)) by the checked height arithmetic of the traces
        // and the per-tic vertical witnesses (decision H3); the format envelope of
        // `map_extent::validate_real_map` (±1,024) still applies.
        if episode1.is_none()
            && map
                .sectors()
                .iter()
                .any(|s| s.heights.iter().any(|h| h.unsigned_abs() > 512))
        {
            return Err(PerceptionError::Arithmetic.into());
        }
        if !specials
            && map
                .sectors()
                .iter()
                .any(|s| s.flats[1].as_bytes().eq_ignore_ascii_case(b"F_SKY1\0\0"))
        {
            return Err(CombatError::Sky.into());
        }
        if profile == Profile::OriginalDemon {
            OriginalExecutionRegistry::validate(&registry, OriginalExecutionProfile::DemonNormalV1)
                .map_err(|_| MonsterError::Definition)?;
        } else {
            let expected = ActorRegistry::demon_map_context_fixture();
            if registry.states() != expected.states() || registry.types() != expected.types() {
                return Err(MonsterError::Definition.into());
            }
            DemonExecutionBinding::admit(&registry).map_err(|_| MonsterError::Definition)?;
        }
        registry.validate_impact_source()?;
        let mut sim = if real_map {
            let sight_queries = episode1.map_or(64, |b| b.sight_queries());
            let admission = map_extent::validate_real_map(&map, actor_cap, sight_queries)?;
            let mut sim = Self::create_bounded_with(
                map,
                limit,
                profile,
                admission.references,
                GeometryCapacity {
                    specials,
                    ..GeometryCapacity::FORMAT
                },
            )?;
            sim.world.extent_sight_work = Some(admission.sight_work);
            sim.world.sight_work_budget = admission.sight_budget;
            if let Some(bounds) = episode1 {
                // One explosion's sight work (decision H4) must be representable.
                admission
                    .sight_work
                    .checked_add(5 * sim.world.lines.len())
                    .and_then(|w| w.checked_mul(bounds.explosion_sight_queries()))
                    .ok_or(GeometryError::WorkLimit)?;
                sim.world.episode1 = Some(bounds);
                sim.world.teleport = teleport;
                sim.world.sight_query_limit = sight_queries;
                sim.world.checked_arithmetic = true;
            }
            if specials {
                sim.install_specials(limit, movers)?;
            }
            sim
        } else {
            let limits = MapExtentLimits {
                max_actors: limit,
                ..Default::default()
            };
            let work = map_extent::validate_with_actor_cap(&map, limits, actor_cap)?;
            let mut sim =
                Self::create_bounded(map, limit, profile, limits.max_prepared_references)?;
            sim.world.extent_sight_work = Some(work);
            sim
        };
        if type_envelope {
            // Heights policy: windows and closed sectors may be shorter than any
            // actor; dimensions are admitted against the tallest sector and every
            // spawn proves its own fit. Earlier policies keep the map-wide fit.
            sim.type_envelope = sim
                .sectors
                .iter()
                .copied()
                .max_by_key(|s| s.ceiling - s.floor);
        }
        if profile != Profile::OriginalDemon {
            snapshot::registry_fields(Arc::make_mut(&mut sim.map_fields), &registry);
        }
        sim.registry = Some(Arc::new(registry));
        sim.actor_options = options;
        sim.rng = context.initial_rng;
        sim.statistics = Some(LevelStatistics::default());
        sim.sound_targets = vec![None; sim.sectors.len()];
        for name in ["MT_PUFF", "MT_BLOOD"] {
            sim.validate_combat_type(lifecycle::source_type_id(name).unwrap())?;
        }
        Ok(sim)
    }

    /// The specials policy's load-time state: a sector-thinker slot class bounded by
    /// the number of light sectors `L` (the only sector thinkers this policy
    /// creates, all at load), so the visit budget is the 320 actor slots plus that
    /// count; the per-sector sky identity and `blockbox`; and the runtime lights and
    /// specials, still the map's until `spawn_level_specials` runs.
    ///
    /// Under the movers policy the slot class holds `L + 2M` (`M` mover sectors: a
    /// removed mover keeps its slot until the next pass while a replacement for its
    /// sector can exist, decision N8), so the visit budget is `320 + L + 2M`; the
    /// runtime line specials start as the map's, and the admission is kept.
    fn install_specials(
        &mut self,
        limit: u32,
        movers: Option<mover_rules::MoverAdmission>,
    ) -> Result<(), SimError> {
        let map = self.world.map();
        let lights = map
            .sectors()
            .iter()
            .filter(|s| matches!(s.special, 1 | 2 | 3 | 4 | 8 | 12 | 13 | 17))
            .count();
        // Under the Episode 1 policy stair and donut sectors are not tagged, so
        // `M` is not known statically: the class holds `L + 2·numsectors` (the
        // capacity is not DSIM state; only the high-water `slot_count` is).
        let movers_slots = if self.world.episode1.is_some() {
            map.sectors().len().checked_mul(2)
        } else {
            movers.as_ref().map_or(Some(0), |m| m.movers.checked_mul(2))
        };
        let lights = movers_slots
            .and_then(|m| m.checked_add(lights))
            .and_then(|slots| u32::try_from(slots).ok())
            .ok_or(StoreError::InvalidLimit)?;
        let sky = map
            .sectors()
            .iter()
            .map(|s| is_sky_flat(&s.flats[1]))
            .collect();
        let sectors = map
            .sectors()
            .iter()
            .map(|s| sector_thinkers::SectorSpecial {
                light: s.light,
                special: s.special,
                floorpic: sector_thinkers::normalized_flat(&s.flats[0]),
            })
            .collect();
        let blockboxes = multisector::sector_block_boxes(map)?;
        let line_specials = movers
            .is_some()
            .then(|| map.linedefs().iter().map(|l| l.special).collect());
        self.actors = ThinkerStore::with_sector_capacity(limit, lights)?;
        self.world.sky_ceilings = Some(sky);
        self.world.sector_block_boxes = Some(blockboxes);
        self.specials = Some(sector_thinkers::LevelSpecials {
            sectors,
            spawned: false,
        });
        if let Some(lines) = line_specials {
            self.line_specials = lines;
        }
        self.movers = movers;
        Ok(())
    }
    /// Derived prepared storage metrics; excluded from canonical state.
    pub fn demon_expanded_map_metrics(&self) -> Result<MapScaleMetrics, SimError> {
        if self.profile != Profile::DemonExpandedMap {
            return Err(MonsterError::Profile.into());
        }
        Ok(self.prepared_geometry_metrics())
    }
}

/// `ceilingpic == skyflatnum`: `skyflatnum = R_FlatNumForName("F_SKY1")` and a
/// sector's `ceilingpic = R_FlatNumForName(name)` are the same flat exactly when
/// `W_CheckNumForName` resolves both names to one lump, that is when the eight
/// name bytes up to the first NUL equal `F_SKY1` ignoring ASCII case
/// (`strncpy` then `strupr`, w_wad.c). The declared flat table of the C evidence
/// (`skyflatnum` 2, the index of `F_SKY1`) is one instance of this rule.
pub(crate) fn is_sky_flat(name: &doom_types::LumpName) -> bool {
    let bytes = name.as_bytes();
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    bytes[..end].eq_ignore_ascii_case(b"F_SKY1")
}
/// The specials policy's load rules (M3h7 note §4.2): every sector special is one
/// of `ADMITTED_SECTOR_SPECIALS` or 0 (10, 11, 14 and unknown values stay
/// rejected); every line special is 0 or 48, a 48 is one-sided (a two-sided
/// special line would enter `spechit`, Stage B) and there are at most 64 of them
/// (`MAXLINEANIMS`, filled without a bound check by the source): all
/// `PursuitError::Specials`. Every sector light lies in `LIGHT_RANGE`
/// (`MapScaleError::Light`). Tags are kept: no admitted special reads them.
pub(crate) fn validate_specials(map: &Map) -> Result<(), SimError> {
    use sector_thinkers::{MAX_LINE_ANIMS, SCROLLING_WALL};
    validate_sector_specials(map)?;
    let mut scrollers = 0;
    for line in map.linedefs() {
        match line.special {
            0 => {}
            SCROLLING_WALL if line.sides[1].is_none() => scrollers += 1,
            _ => return Err(PursuitError::Specials.into()),
        }
    }
    if scrollers > MAX_LINE_ANIMS {
        return Err(PursuitError::Specials.into());
    }
    validate_lights(map)
}
/// Every sector special is one of `ADMITTED_SECTOR_SPECIALS` or 0
/// (`PursuitError::Specials`): the sector rule of the specials and movers policies.
pub(crate) fn validate_sector_specials(map: &Map) -> Result<(), SimError> {
    validate_sector_specials_for(map, false)
}
/// [`validate_sector_specials`]; `episode1` also admits special 11 (the
/// Episode 1 policy's EXIT SUPER DAMAGE, p_spec.c:1054–1063).
pub(crate) fn validate_sector_specials_for(map: &Map, episode1: bool) -> Result<(), SimError> {
    use sector_thinkers::ADMITTED_SECTOR_SPECIALS;
    if map.sectors().iter().any(|s| {
        s.special != 0
            && !ADMITTED_SECTOR_SPECIALS.contains(&s.special)
            && !(episode1 && s.special == 11)
    }) {
        return Err(PursuitError::Specials.into());
    }
    Ok(())
}
/// Every sector light lies in `LIGHT_RANGE` (`MapScaleError::Light`).
pub(crate) fn validate_lights(map: &Map) -> Result<(), SimError> {
    if map
        .sectors()
        .iter()
        .any(|s| !sector_thinkers::LIGHT_RANGE.contains(&s.light))
    {
        return Err(MapScaleError::Light.into());
    }
    Ok(())
}

use crate::lifecycle::LifeHost;
use std::cell::Cell;

/// Transient per-operation work/error control. Never part of canonical state.
pub(crate) struct SightControl {
    pub failure: Cell<Option<SimError>>,
    queries: Cell<usize>,
    work: Cell<usize>,
}
impl Default for SightControl {
    fn default() -> Self {
        Self {
            failure: Cell::new(None),
            queries: Cell::new(0),
            work: Cell::new(0),
        }
    }
}
pub(crate) fn action_sight(
    control: Option<&SightControl>,
    world: &StaticWorld,
    sectors: crate::staging::Sectors<'_>,
    observer: Actor,
    target: Actor,
) -> Result<SightResult, SimError> {
    if let Some(control) = control {
        if let Some(error) = control.failure.get() {
            return Err(error);
        }
        let count = control.queries.get() + 1;
        let work = control
            .work
            .get()
            .checked_add(world.extent_sight_work.expect("checked geometry") + 5 * world.lines.len())
            .ok_or(PerceptionError::WorkLimit)?;
        if count > world.sight_query_limit || work > world.sight_work_budget {
            return Err(PerceptionError::WorkLimit.into());
        }
        control.queries.set(count);
        control.work.set(work);
    }
    sight::checked_query(world, sectors, observer, target)
}

/// Only the bounded mutable actor projection is staged. World, map, registry,
/// sectors, sound targets, roster, canonical caches and old events are borrowed.
/// Staged per-sector sound targets: the flat policies stage their one sector without
/// allocating (the tic plan keeps exactly its two historical allocations); the
/// heights policy copies every sector's target.
enum StagedSound {
    One(Option<ActorId>),
    Many(Vec<Option<ActorId>>),
}
impl StagedSound {
    fn as_slice(&self) -> &[Option<ActorId>] {
        match self {
            Self::One(target) => std::slice::from_ref(target),
            Self::Many(targets) => targets,
        }
    }
    fn as_mut_slice(&mut self) -> &mut [Option<ActorId>] {
        match self {
            Self::One(target) => std::slice::from_mut(target),
            Self::Many(targets) => targets,
        }
    }
}
struct DemonPlan {
    actors: ThinkerStore<Actor>,
    spatial: movement::SpatialIndex,
    rng: DoomRng,
    statistics: Option<LevelStatistics>,
    player: Option<original_player::PlayerRuntime>,
    bindings: [Option<ActorId>; 4],
    /// Sound targets staged by the weapon noise.
    sound_targets: StagedSound,
    /// Under the specials policy, the runtime sector lights and specials written
    /// by the light thinkers and secrets of this tic (empty otherwise).
    specials: Vec<sector_thinkers::SectorSpecial>,
    /// The sector heights and line specials written in this tic, each table in a
    /// lazy full copy taken on its first write (M3h8 note §9), read through by every
    /// later query of the plan; nothing staged under every policy without movers.
    writes: staging::StagedWrites,
    /// Under the movers policy, `G_ExitLevel` ran in this tic or before.
    exit: bool,
    /// Under the Episode 1 policy, `G_SecretExitLevel` ran (`secretexit`).
    secret_exit: bool,
    /// Under the movers policy, the staged switch buttons.
    buttons: use_lines::Buttons,
    events: Vec<Event>,
}
impl Simulation {
    fn demon_plan(&mut self) -> Result<DemonPlan, SimError> {
        let mut slots = line_work::reserved(self.actors.slots.len())?;
        slots.extend_from_slice(&self.actors.slots);
        // The shared order lives in the slot links plus head/tail, so staging both
        // classes stages it. The sector class is staged in the spare buffer of the
        // last committed plan, as for the sound targets: no allocation once its
        // capacity matches the sector-thinker slot count. Without sector thinkers it
        // is empty and never allocates: the tic plan keeps its two historical
        // allocations.
        let mut sector_slots = std::mem::take(&mut self.spare_sector_slots);
        sector_slots.clear();
        sector_slots
            .try_reserve(self.actors.sector_slots.len())
            .map_err(|_| StoreError::Capacity)?;
        sector_slots.extend_from_slice(&self.actors.sector_slots);
        let mut order = line_work::reserved(self.actors.max_slots as usize)?;
        order.extend_from_slice(&self.spatial.order);
        let sound_targets = if self.player_policy().has_heights() {
            // The spare buffer swapped out by the last committed plan: no
            // allocation once its capacity matches the sector count. It is never
            // read, so a failed tic drops it and the next plan allocates once.
            let mut targets = std::mem::take(&mut self.spare_sound_targets);
            targets.clear();
            targets
                .try_reserve(self.sound_targets.len())
                .map_err(|_| StoreError::Capacity)?;
            targets.extend_from_slice(&self.sound_targets);
            StagedSound::Many(targets)
        } else {
            StagedSound::One(self.sound_targets.first().copied().flatten())
        };
        // The spare buffer of the last committed plan, as for the sound targets:
        // no allocation once its capacity matches the sector count; empty (and
        // never allocating) without the specials policy.
        let mut specials = std::mem::take(&mut self.spare_specials);
        specials.clear();
        if let Some(level) = &self.specials {
            specials
                .try_reserve(level.sectors.len())
                .map_err(|_| StoreError::Capacity)?;
            specials.extend_from_slice(&level.sectors);
        }
        // The staging buffers of the last committed plan: copied into only on a
        // plan's first write, so no allocation without writes and none once their
        // capacity matches the tables.
        let mut writes = std::mem::take(&mut self.spare_writes);
        writes.clear();
        Ok(DemonPlan {
            actors: ThinkerStore {
                slots,
                sector_slots,
                head: self.actors.head,
                tail: self.actors.tail,
                max_slots: self.actors.max_slots,
                max_sector_slots: self.actors.max_sector_slots,
            },
            spatial: movement::SpatialIndex { order },
            rng: self.rng,
            statistics: self.statistics,
            player: self.original_player,
            bindings: self.bindings,
            sound_targets,
            specials,
            writes,
            exit: self.level_exit,
            secret_exit: self.level_secret_exit,
            buttons: self.buttons,
            events: Vec::new(),
        })
    }
    fn checked_demon_inputs<'a>(
        &'a self,
        control: &'a SightControl,
    ) -> perception::PerceptionInputs<'a> {
        perception::PerceptionInputs {
            checked: Some(control),
            players: if self.uses_active_player_roster() {
                player_roster::PerceptionRoster::OriginalPlayer(self.player_policy())
            } else {
                player_roster::PerceptionRoster::Historical(&self.perception_players)
            },
            sound_targets: &self.sound_targets,
            pursuit: true,
            monsters: true,
            shotgunner: false,
            chaingunner: false,
            demon: true,
            map_demons: true,
            melee: None,
            shotgun: None,
            shot: None,
        }
    }
    fn preflight_expanded_demons(&self) -> Result<(), SimError> {
        self.preflight_expanded_demons_for(false)
    }
    fn preflight_expanded_demons_for(&self, stepping: bool) -> Result<(), SimError> {
        if self.statistics.is_none() || self.sound_targets.len() != self.sectors.len() {
            return Err(MapSpawnError::Ownership.into());
        }
        let mut linked = std::collections::BTreeSet::new();
        for &id in &self.spatial.order {
            if self.actors.is_pending(id) != Some(false) || !linked.insert(id) {
                return Err(MapSpawnError::Ownership.into());
            }
        }
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            let actor = self.actor(id).unwrap();
            sight::actor_bounds_within(actor, self.sight_margin())?;
            let life = actor.life.ok_or(MonsterError::Definition)?;
            if life.tics < -1 {
                return Err(ActorError::InvalidState.into());
            }
            // P_CheckMissileSpawn keeps a blocked half-step without relinking: an
            // exploding fireball's coordinates may lead its source location.
            let unlinked_blast = original_registry::is_missile_type(life.actor_type)
                && life.flags & original_player::MF_MISSILE == 0;
            // Under the E1M1 policy a static decoration may hold `floorz > ceilingz`:
            // a door closing onto one whose box straddles a higher neighbouring floor
            // closes fully (PIT_ChangeSector returns at the MF_SHOOTABLE test,
            // p_map.c:1290–1294, so nofit is never set) and P_ThingHeightClip stores
            // P_CheckPosition's tmfloorz above its tmceilingz (530–558). That pair is
            // inert for a decoration (M3h9 note §12, independent review): it stays on
            // its floor (`z == floorz`, V′ case (b)) with no momentum, so P_MobjThinker
            // runs neither P_XYMovement nor P_ZMovement (p_mobj.c:418–436), no other
            // body reads a thing's own floorz or ceilingz (PIT_CheckThing reads z and
            // height, 252–345; hitscan, aim and radius attacks skip non-shootables,
            // 1013–1014, 1170–1171), and the next clip recomputes both. Every other
            // actor, and every older policy, keeps the refusal.
            // The Episode 1 policy widens the same rule to every non-shootable
            // thing (M4a step 5 ruling): PIT_ChangeSector returns true for
            // `!(thing->flags & MF_SHOOTABLE)` ("assume it is bloody gibs or
            // something", p_map.c:1290–1294) after gibbing corpses and crunching
            // dropped items, so an undropped pickup (or a corpse, or a spent
            // fireball) left under a mover with `floorz > ceilingz` is a legal,
            // inert state on its floor. Shootable things keep the refusal.
            let inert_decoration =
                (e1m1_rules::is_decoration(self.player_policy(), life.actor_type)
                    || (self.world.episode1.is_some()
                        && life.flags & original_player::MF_SHOOTABLE == 0))
                    && actor.z == actor.floor
                    && actor.momz == Fixed::ZERO
                    && actor.momentum() == [Fixed::ZERO; 2];
            if !linked.contains(&id)
                || (!unlinked_blast
                    && actor.location != Some(self.world.locate_validated(actor.position())))
                || (actor.floor > actor.ceiling && !inert_decoration)
                || (actor.height() <= Fixed::ZERO && !self.gibbed(actor))
            {
                return Err(MapSpawnError::Ownership.into());
            }
            if self.room().is_some() {
                player_map_scale::momentum_admitted(actor.momentum())?;
            } else {
                map_scale::bounded(actor.momentum(), 64)?;
            }
            let policy = self.player_policy();
            if policy.has_heights()
                && !original_registry::is_impact(life.actor_type)
                && !unlinked_blast
            {
                // Invariant V: impacts spawn at the source's absolute z, and a
                // fireball exploded by its own half-step is clipped by its next
                // P_ZMovement (in the same pass for an in-tic launch), so only
                // their z/momz witnesses apply.
                self.vertical_invariant(actor)?;
            } else {
                map_scale::bounded([actor.momz], player_map_scale::momz_bound(policy))?;
                if policy.has_heights() {
                    map_scale::bounded([actor.z], player_map_scale::HEIGHTS_Z_UNITS)?;
                }
            }
        }
        self.preflight_player_body(stepping)?;
        self.preflight_monster_owner()?;
        self.preflight_live_missiles()?;
        Ok(())
    }
    fn commit_demon_plan(&mut self, plan: DemonPlan) -> Vec<Event> {
        let committed = std::mem::replace(&mut self.actors, plan.actors);
        self.spare_sector_slots = committed.sector_slots;
        self.spatial = plan.spatial;
        self.rng = plan.rng;
        self.statistics = plan.statistics;
        self.original_player = plan.player;
        if self.player_policy().has_player_weapons() {
            match plan.sound_targets {
                StagedSound::One(target) => {
                    if let Some(first) = self.sound_targets.first_mut() {
                        *first = target;
                    }
                }
                StagedSound::Many(targets) => {
                    self.spare_sound_targets = std::mem::replace(&mut self.sound_targets, targets);
                }
            }
        }
        self.bindings = plan.bindings;
        if let Some(level) = &mut self.specials {
            self.spare_specials = std::mem::replace(&mut level.sectors, plan.specials);
        }
        let mut writes = plan.writes;
        writes.commit(&mut self.sectors, &mut self.line_specials);
        self.spare_writes = writes;
        self.level_exit = plan.exit;
        self.level_secret_exit = plan.secret_exit;
        self.buttons = plan.buttons;
        plan.events
    }
    /// New-policy THING setup stages the same bounded actor/RNG/statistics plan.
    /// It runs before player binding, so it does not require a live-player preflight.
    pub(crate) fn checked_map_spawn_batch(
        &mut self,
        count: usize,
        action: impl FnOnce(
            &mut LifeHost<'_, &mut ThinkerStore<Actor>>,
        ) -> Result<Vec<ActorId>, SimError>,
    ) -> Result<Vec<ActorId>, SimError> {
        if !self.player_policy().has_player_bonuses() || self.statistics.is_none() {
            return Err(MapSpawnError::Context.into());
        }
        let mut plan = self.demon_plan()?;
        plan.actors.reserve_spawns(count)?;
        plan.spatial
            .order
            .try_reserve(count)
            .map_err(|_| StoreError::Capacity)?;
        let result = action(&mut LifeHost {
            specials: None,
            buttons: None,
            player: None,
            statistics: plan.statistics.as_mut(),
            perception: None,
            access: &mut plan.actors,
            registry: self.registry.as_deref().ok_or(ActorError::InvalidType)?,
            world: &self.world,
            sectors: plan.writes.view(&self.sectors, &self.line_specials),
            options: self.actor_options,
            rng: &mut plan.rng,
            spatial: &mut plan.spatial,
            bindings: &mut plan.bindings,
            events: &mut plan.events,
        })?;
        let events = self.commit_demon_plan(plan);
        self.pending.extend(events);
        Ok(result)
    }
    /// One bounded shared action execution; successful plans are committed once.
    pub(crate) fn checked_demon_operation<T>(
        &mut self,
        action: impl FnOnce(&mut LifeHost<'_, &mut ThinkerStore<Actor>>) -> T,
    ) -> Result<T, SimError> {
        self.preflight_expanded_demons()?;
        let mut plan = self.demon_plan()?;
        let control = SightControl::default();
        let result = action(&mut LifeHost {
            specials: None,
            buttons: None,
            player: plan.player.as_mut(),
            statistics: plan.statistics.as_mut(),
            perception: Some(self.checked_demon_inputs(&control)),
            access: &mut plan.actors,
            registry: self.registry.as_deref().unwrap(),
            world: &self.world,
            sectors: plan.writes.view(&self.sectors, &self.line_specials),
            options: self.actor_options,
            rng: &mut plan.rng,
            spatial: &mut plan.spatial,
            bindings: &mut plan.bindings,
            events: &mut plan.events,
        });
        if let Some(error) = control.failure.get() {
            return Err(self.policy_error(error));
        }
        let events = self.commit_demon_plan(plan);
        self.pending.extend(events);
        Ok(result)
    }
    /// The real-map policy names the source's 129th intercept of a trace, reached
    /// by a hitscan or a slide inside a refused tic, `MapScaleError::Intercepts`
    /// (M3h6 note §4.4); every other error and policy is unchanged.
    fn policy_error(&self, error: SimError) -> SimError {
        if self.player_policy().has_real_map()
            && error == SimError::Combat(CombatError::InterceptLimit)
        {
            MapScaleError::Intercepts.into()
        } else if error == SimError::Combat(CombatError::Arithmetic) {
            // Only the Episode 1 policy's checked traces raise it (decision H3).
            MapScaleError::Arithmetic.into()
        } else {
            error
        }
    }
    pub(crate) fn advance_expanded_demons(
        &mut self,
        mut commands: [TicCommand; 4],
    ) -> Result<(), SimError> {
        let command_policy = self.player_policy().has_player_commands();
        let weapon_policy = self.player_policy().has_player_weapons();
        let specials = self.specials.is_some();
        if weapon_policy
            && self
                .original_player
                .as_ref()
                .is_none_or(|p| p.weapon.is_none())
        {
            return Err(WeaponError::Player.into());
        }
        let command_body = if command_policy {
            self.preflight_player_commands(&commands)?
        } else {
            None
        };
        self.preflight_expanded_demons_for(true)?;
        self.preflight_actor_step()?;
        let mut plan = self.demon_plan()?;
        let control = SightControl::default();
        let movers = self.player_policy().has_movers();
        // What the use block requested, applied with the staged tables after it.
        let mut used = use_lines::UseOutcome::default();
        // Whether sector special 11 requested `G_ExitLevel` (Episode 1 policy).
        let mut special_exit = false;
        if command_policy {
            plan.events.push(Event::Phase(Phase::Players));
            // G_Ticker, before P_Ticker (g_game.c:671–679): the turbo message on
            // every 128th gametic when the read command's forward exceeds 50.
            // Only the Episode 1 domain admits such a command.
            if self.player_policy().has_episode1()
                && player_commands::turbo_message(
                    self.tic,
                    commands[usize::from(PlayerSlot::One.ordinal())],
                )
            {
                plan.player
                    .as_mut()
                    .and_then(|p| p.pickups.as_mut())
                    .ok_or(crate::PlayerBodyError::Profile)?
                    .message = PickupMessage::Turbo;
            }
            // p_user.c:249–255, before the dead test: `A_Saw`'s `MF_JUSTATTACKED`
            // rewrites the command to the chainsaw lunge (turn 0, forward
            // 0xc800/512 = 100, side 0; buttons kept) and clears the flag. The
            // rewritten command is the one every later reader sees, the stand test
            // of `P_XYMovement` and the DSIM39 capture included (decision D5); the
            // preflight checked the input command.
            if self.player_policy().has_episode1() {
                let body = plan.player.as_ref().and_then(|p| p.mo);
                if let Some(life) = body
                    .and_then(|id| plan.actors.get_mut(id))
                    .and_then(|a| a.life.as_mut())
                    .filter(|l| l.flags & player_commands::MF_JUSTATTACKED != 0)
                {
                    life.flags &= !player_commands::MF_JUSTATTACKED;
                    let c = &mut commands[usize::from(PlayerSlot::One.ordinal())];
                    c.angle_turn = 0;
                    c.forward = 100;
                    c.side = 0;
                }
            }
            // Whether `P_PlayerThink` took its live branch this tic (p_user.c:
            // 258–262), read by the level policy's counter decays.
            let think_live;
            {
                let mut host = LifeHost {
                    specials: None,
                    buttons: movers.then_some(&mut plan.buttons),
                    player: plan.player.as_mut(),
                    statistics: plan.statistics.as_mut(),
                    perception: Some(self.checked_demon_inputs(&control)),
                    access: &mut plan.actors,
                    registry: self.registry.as_deref().ok_or(ActorError::InvalidType)?,
                    world: &self.world,
                    sectors: plan.writes.view(&self.sectors, &self.line_specials),
                    options: self.actor_options,
                    rng: &mut plan.rng,
                    spatial: &mut plan.spatial,
                    bindings: &mut plan.bindings,
                    events: &mut plan.events,
                };
                if let Some(id) = command_body {
                    player_commands::stage(
                        &mut host,
                        id,
                        commands[usize::from(PlayerSlot::One.ordinal())],
                    )
                    .map_err(|error| self.policy_error(error))?;
                }
                let live = host
                    .player
                    .as_ref()
                    .is_some_and(|p| p.state == PlayerBodyState::Live);
                think_live = live;
                if weapon_policy && specials && live {
                    // P_PlayerThink: P_CalcHeight, then P_PlayerInSpecialSector,
                    // then the weapon change and P_MovePsprites (M3h7 step 3 order).
                    // The source leveltime of this tic is the completed-tic count.
                    let command = commands[usize::from(PlayerSlot::One.ordinal())];
                    player_pistol::stage_operation(
                        &mut host,
                        weapons::PlayerWeaponOperation::CalcHeight,
                        command,
                        self.tic,
                        plan.sound_targets.as_mut_slice(),
                    )
                    .map_err(|error| self.policy_error(error))?;
                    special_exit = sector_thinkers::player_in_special_sector(
                        &mut host,
                        &mut plan.specials,
                        self.tic,
                    )?;
                    if let Some(error) = control.failure.get() {
                        return Err(self.policy_error(error));
                    }
                    if movers {
                        // The weapon change, the use block, then P_MovePsprites.
                        player_pistol::stage_operation(
                            &mut host,
                            weapons::PlayerWeaponOperation::WeaponChange,
                            command,
                            self.tic,
                            plan.sound_targets.as_mut_slice(),
                        )
                        .map_err(|error| self.policy_error(error))?;
                        used = use_block(&mut host, &mut plan.specials, command)?;
                        // The weapons' damage to the player body (a rocket
                        // exploding at once) reads the sector special-11 cap.
                        host.specials = Some(plan.specials.as_slice());
                        player_pistol::stage_operation(
                            &mut host,
                            weapons::PlayerWeaponOperation::MovePsprites,
                            command,
                            self.tic,
                            plan.sound_targets.as_mut_slice(),
                        )
                        .map_err(|error| self.policy_error(error))?;
                    } else {
                        host.specials = Some(plan.specials.as_slice());
                        player_pistol::stage_operation(
                            &mut host,
                            weapons::PlayerWeaponOperation::AfterSpecials,
                            command,
                            self.tic,
                            plan.sound_targets.as_mut_slice(),
                        )
                        .map_err(|error| self.policy_error(error))?;
                    }
                } else if weapon_policy {
                    let command = commands[usize::from(PlayerSlot::One.ordinal())];
                    player_pistol::stage(
                        &mut host,
                        command,
                        self.tic,
                        plan.sound_targets.as_mut_slice(),
                    )
                    .map_err(|error| self.policy_error(error))?;
                    // The level policy runs the rest of `P_DeathThink` (p_user.c:
                    // 182–228) after its `P_MovePsprites` (187): `P_CalcHeight`'s
                    // bob (198), then the turn toward the attacker and the dead
                    // `damagecount` decay (200–224).
                    if self.player_policy().has_level() && !live {
                        player_pistol::stage_operation(
                            &mut host,
                            weapons::PlayerWeaponOperation::CalcHeight,
                            command,
                            self.tic,
                            plan.sound_targets.as_mut_slice(),
                        )
                        .map_err(|error| self.policy_error(error))?;
                        crate::level::death_turn(&mut host)?;
                    }
                    // P_DeathThink's last statement: BT_USE while dead requests a
                    // reborn (decision N3); the tic completes and the next is refused.
                    if movers && !live && command.buttons & weapons::BT_USE != 0 {
                        let p = host.player.as_deref_mut().ok_or(PlayerBodyError::Profile)?;
                        p.reborn = true;
                        host.events
                            .push(Event::Mover(MoverEvent::Reborn { slot: 0 }));
                    }
                }
            }
            if let Some(line) = used.cleared {
                plan.writes
                    .write_line_special(&self.line_specials, line, 0)?;
            }
            plan.exit |= used.exit || special_exit;
            plan.secret_exit |= used.secret;
            crate::episode1::powers_tick(plan.player.as_mut(), &mut plan.actors, think_live)?;
            world_pickups::bonus_tick(plan.player.as_mut(), think_live)?;
            plan.events.push(Event::Phase(Phase::Thinkers));
        } else {
            plan.events
                .extend([Event::Phase(Phase::Players), Event::Phase(Phase::Thinkers)]);
        }
        let mut perception_inputs = self.checked_demon_inputs(&control);
        if weapon_policy {
            perception_inputs.sound_targets = plan.sound_targets.as_slice();
        }
        thinkers::run(
            &mut plan.actors,
            thinkers::Inputs {
                world: &self.world,
                committed: &self.sectors,
                committed_lines: &self.line_specials,
                writes: &mut plan.writes,
                leveltime: self.tic,
                profile: self.profile,
                bindings: plan.bindings,
                live_bindings: &mut plan.bindings,
                registry: &self.registry,
                options: self.actor_options,
                statistics: &mut plan.statistics,
                player: plan.player.as_mut(),
                rng: &mut plan.rng,
                spatial: &mut plan.spatial,
                events: &mut plan.events,
                commands,
                perception_inputs: Some(perception_inputs),
                boundary: self.boundary.as_deref(),
                specials: specials.then_some(plan.specials.as_mut_slice()),
                buttons: movers.then_some(&mut plan.buttons),
            },
        );
        if let Some(error) = control.failure.get() {
            return Err(self.policy_error(error));
        }
        if command_policy {
            player_commands::validate_result(&plan.actors, plan.player.as_ref())?;
            if weapon_policy {
                for id in plan.actors.order() {
                    if let Some(actor) = plan.actors.get(id) {
                        if actor.life().is_some_and(|l| {
                            original_registry::is_policy_monster(self.player_policy(), l.actor_type)
                                || crate::barrels::is_barrel(self.player_policy(), l.actor_type)
                        }) {
                            player_map_scale::travel_admitted(
                                self.room(),
                                actor.position(),
                                actor.momentum(),
                            )?;
                        }
                    }
                }
            }
        }
        // P_UpdateSpecials: the button countdown (movers policy), in its phase.
        let special_events = movers.then(|| {
            let mut events = Vec::new();
            use_lines::update_buttons(&mut plan.buttons, &mut events);
            events
        });
        let events = self.commit_demon_plan(plan);
        self.events = std::mem::take(&mut self.pending);
        self.events.extend(events);
        match special_events {
            Some(special_events) => {
                self.events.push(Event::Phase(Phase::Specials));
                self.events.extend(special_events);
                self.events.push(Event::Phase(Phase::Respawn));
            }
            None => self
                .events
                .extend([Event::Phase(Phase::Specials), Event::Phase(Phase::Respawn)]),
        }
        self.commands = commands;
        self.tic += 1;
        Ok(())
    }
}

/// The use block of `P_PlayerThink` (p_user.c:322–331) under the movers policy:
/// with `BT_USE` held and `usedown` clear, `P_UseLines` and set `usedown`; without
/// `BT_USE`, clear it. A door the use opens joins the order's tail and steps in this
/// tic's pass. The line clear and the exits are returned for the staged tables;
/// the Episode 1 starters write the staged sector `specials` and the player's
/// message directly.
fn use_block(
    host: &mut LifeHost<'_, &mut ThinkerStore<Actor>>,
    specials: &mut [sector_thinkers::SectorSpecial],
    command: TicCommand,
) -> Result<use_lines::UseOutcome, SimError> {
    let p = host.player.as_deref_mut().ok_or(PlayerBodyError::Profile)?;
    if command.buttons & weapons::BT_USE == 0 {
        p.use_down = false;
        return Ok(use_lines::UseOutcome::default());
    }
    if p.use_down {
        return Ok(use_lines::UseOutcome::default());
    }
    p.use_down = true;
    let id = p.mo.ok_or(PlayerBodyError::Binding)?;
    let actor = *host.access.get(id).ok_or(PlayerBodyError::Binding)?;
    let buttons = host
        .buttons
        .as_deref_mut()
        .ok_or(PlayerBodyError::Profile)?;
    use_lines::use_lines(
        &mut movers::Starter {
            world: host.world,
            sectors: host.sectors,
            store: &mut *host.access,
            events: &mut *host.events,
            specials: Some(specials),
            player: host.player.as_deref_mut(),
        },
        buttons,
        id,
        actor.position(),
        actor.angle(),
        Some(player_map_scale::MAX_INTERCEPTS),
    )
}

#[cfg(test)]
#[path = "demon_expanded_map_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "transaction_cost_tests.rs"]
mod transaction_cost_tests;
