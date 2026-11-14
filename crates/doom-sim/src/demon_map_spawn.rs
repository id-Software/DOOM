//! Bounded commercial single-player THING admission, reusing the profile15 fixture.
use crate::*;

/// Maximum validated classic THING records accepted by profile16.
pub const MAX_DEMON_MAP_THINGS: usize = 256;
/// Maximum native map Demons before the unbound target/effect thinker suffix.
pub const MAX_MAP_DEMONS: usize = 4;

/// Map-spawn input or explicit actor-admission restriction, before mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapSpawnError {
    /// New combined profile excludes an overflowing original signed BAM product.
    Angle(i16),
    /// Requested source respawn behavior is not admitted.
    Respawn,
    /// Operation requires the explicit map-context profile.
    Context,
    /// Checked source statistics would exceed signed 32-bit range.
    StatisticsOverflow,
    /// Batch setup requires tic zero and a never-allocated actor store.
    Setup,
    /// The bounded record or Demon count would be exceeded.
    WorkLimit,
    /// Player/deathmatch starts and invalid nonpositive types are excluded.
    StartOrInvalidType(i16),
    /// A recognized enabled type is outside Demon admission.
    UnsupportedType(i16),
    /// Native Demons enter only through the atomic THING batch.
    DirectSpawn,
    /// A live Demon or perception input needs an unbound body target.
    Target,
    /// Generalized cross-Demon damage/retargeting is outside this slice.
    Infighting,
    /// The owner-prefix or bounded late-damage planning proof does not hold.
    Ownership,
    /// Under the specials policy, `spawn_level_specials` called under another
    /// policy, after a tic, a second time or before the player body exists; a tic
    /// stepped before it ran; or a map-THING or player-body spawn after it ran
    /// (M3h7 decision 12). The state is unchanged.
    LevelSpecials,
    /// Under the E1M1 policy, a 33rd static decoration in one map
    /// ([`crate::MAX_DECORATIONS`], M3h9 note §5.1): the batch spawns nothing and
    /// the state is unchanged.
    DecorationLimit,
    /// Under the barrels policy, a 17th exploding barrel in one map
    /// ([`crate::MAX_BARRELS`], M3h9b note §5.1): the batch spawns nothing and the
    /// state is unchanged.
    BarrelLimit,
}
impl fmt::Display for MapSpawnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Demon map spawning: {self:?}")
    }
}
impl Error for MapSpawnError {}

impl Simulation {
    /// Empty profile16 using the exact profile15 native fixture and source
    /// DoomEd lookup. Call `spawn_map_things` explicitly before adding targets.
    /// Single-player DOOM II, normal timings; no player starts/scoring/raise.
    pub fn with_demon_map_spawn_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if map.things().len() > MAX_DEMON_MAP_THINGS {
            return Err(MapSpawnError::WorkLimit.into());
        }
        Self::with_demon_lifecycle_profile(map, limit, registry, options, Profile::DemonMapSpawn)
    }
}

impl Simulation {
    /// No recursion into owner callbacks: this bounded global proof also runs
    /// for direct calls before any action, state, RNG, movement or sound change.
    pub(crate) fn preflight_map_demons(&self) -> Result<(), SimError> {
        if !self.profile.uses_demon_map() {
            return Ok(());
        }
        if self.bindings.iter().any(Option::is_some) || self.weapon_player.is_some() {
            return Err(CombatError::Player.into());
        }
        self.preflight_perception_roster()?;
        for target in self.sound_targets.iter().flatten() {
            self.validate_perception_reference(*target)?;
        }
        self.preflight_world_pickups()?;
        let mut suffix = false;
        let mut owners = 0;
        let mut decorations = 0;
        let mut barrels = 0;
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            let a = self.monster_actor(id, 1008)?;
            let life = a.life.unwrap();
            // Validate complete future actions, not only this tic's next state.
            self.validate_combat_type(life.actor_type)?;
            if e1m1_rules::is_decoration(self.player_policy(), life.actor_type) {
                // A static decoration (M3h9 note §2.5–§2.7) interleaves with the
                // monsters and pickups in THINGS order (P_SpawnMapThing spawns the
                // lump in order) and is never the suffix; nothing acts on it per
                // tic, so the witness is its spawn record.
                decorations += 1;
                if decorations > self.decoration_limit() {
                    return Err(MapSpawnError::DecorationLimit.into());
                }
                e1m1_rules::validate_decoration(self.room(), a)?;
            } else if crate::barrels::is_barrel(self.player_policy(), life.actor_type) {
                // An exploding barrel (M3h9b note §2.1) interleaves with the other
                // THINGs in lump order and is never the suffix; its witness covers
                // every state it can hold, `S_GIBS` after a crunch included.
                barrels += 1;
                if barrels > self.barrel_limit() {
                    return Err(MapSpawnError::BarrelLimit.into());
                }
                crate::barrels::validate_barrel(self.room(), a, self.gibbed(&a))?;
            } else if crate::teleport::is_landing(self.player_policy(), life.actor_type) {
                // The teleport landing (wire 24): spawned by `P_SpawnMapThing` in
                // THINGS order like a decoration, never the suffix; it holds its
                // spawn record forever (`S_NULL`, tics −1, no momentum; p_mobj.c:
                // 497–501, 441–449; M4b note §2 T7).
                crate::teleport::validate_landing(self.room(), a)?;
            } else if self.player_policy().has_world_pickups()
                && world_pickups::kind_for(self.player_policy(), life.actor_type).is_some()
            {
                world_pickups::validate_item(self.room(), self.item_sectors(), a)?;
                // P_KillMobj drops are appended after the player/impact suffix.
                let dropped = self.player_policy().has_monsters()
                    && life.flags & world_pickups::MF_DROPPED != 0;
                if suffix && !dropped {
                    return Err(MapSpawnError::Ownership.into());
                }
            } else if self.is_roster_monster(life.actor_type) {
                owners += 1;
                if suffix || owners > self.map_demon_limit() {
                    return Err(MapSpawnError::Ownership.into());
                }
                if life.health > 0 {
                    if let Some(target) = life.target {
                        self.validate_perception_reference(target)?;
                    }
                }
            } else {
                // The level policy's body is spawned at its THINGS record (decision
                // G4, p_mobj.c:729–736): it interleaves with the THINGs in lump
                // order and does not start the player/impact suffix.
                let level_body = self.player_policy().has_level()
                    && self.original_player.is_some_and(|p| p.mo == Some(id));
                if !level_body {
                    suffix = true;
                }
                if !self.position_admitted(a.position(), 994) {
                    return Err(MonsterError::Arithmetic.into());
                }
                if life.actor_type == self.demon_body_type()
                    && a.momentum().iter().any(|v| !crate::sight::within(*v, 64))
                {
                    return Err(MapSpawnError::Ownership.into());
                }
            }
            if !crate::player_map_scale::vertical_witness(self.player_policy(), &a) {
                return Err(MonsterError::Arithmetic.into());
            }
        }
        Ok(())
    }
    pub(crate) fn preflight_map_damage(&self, request: DamageRequest) -> Result<(), SimError> {
        if !self.profile.uses_demon_map() {
            return Ok(());
        }
        if self
            .actor(request.target)
            .and_then(Actor::life)
            .is_some_and(|l| l.actor_type == self.demon_owner_type())
            && request.source.is_some_and(|source| {
                source != request.target
                    && self
                        .actor(source)
                        .and_then(Actor::life)
                        .is_some_and(|l| l.actor_type == self.demon_owner_type())
            })
        {
            let target = self.actor(request.target).unwrap().life.unwrap();
            // Lethal/ignored damage returns before source retargeting. Only this
            // narrow synthetic branch is admitted; nonlethal infighting rejects.
            if !self.profile.uses_map_statistics()
                || target.flags & 4 != 0 && target.health > 0 && request.amount < target.health
            {
                return Err(MapSpawnError::Infighting.into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "demon_map_spawn_tests.rs"]
mod tests;
