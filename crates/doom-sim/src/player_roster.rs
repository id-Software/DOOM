//! Fixed source activity; player fields and the actor store remain authoritative.
use crate::{identity::OriginalActorType, *};

/// DSIM22 policy5: active player0, grounded commands, visual Demon AI, no melee.
pub const PLAYER_DEMON_CONTEXT_SCHEMA: &[u8] = b"PLAYER-DEMON-1-MEDIUM-GROUNDED-NOMELEE-v1";
/// DSIM22 policy6: the fixed active roster with source-timed physical SARG melee.
pub const PLAYER_DEMON_MELEE_CONTEXT_SCHEMA: &[u8] = b"PLAYER-DEMON-1-MEDIUM-GROUNDED-MELEE-v1";

pub(crate) fn view(player: &OriginalPlayer) -> [PerceptionPlayer; 4] {
    let mut roster = [PerceptionPlayer::default(); 4];
    roster[usize::from(PlayerSlot::One.ordinal())] = PerceptionPlayer {
        active: true,
        health: player.health,
        actor: player.mo,
    };
    roster
}

impl Simulation {
    /// Genuine active player0 plus at most one map-THING MT_SERGEANT.
    /// Setup must bind the current player body before any tic/search. Visual
    /// acquisition and bounded pursuit execute; reaching melee rejects atomically.
    pub fn original_player_demon_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let registry =
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonAcquisitionV1)?;
        Self::with_original_registry(map, limit, registry, ActorOptions::default(), context)
            .map_err(Into::into)
    }

    pub(crate) fn uses_active_player_roster(&self) -> bool {
        matches!(
            self.player_policy(),
            OriginalExecutionProfile::PlayerDemonAcquisitionV1
                | OriginalExecutionProfile::PlayerDemonMeleeV1
                | OriginalExecutionProfile::PlayerDemonPistolV1
                | OriginalExecutionProfile::PlayerDemonWeaponsV1
                | OriginalExecutionProfile::PlayerDemonPickupsV1
                | OriginalExecutionProfile::PlayerDemonArmorV1
                | OriginalExecutionProfile::PlayerDemonBonusesV1
                | OriginalExecutionProfile::PlayerMonstersV1
                | OriginalExecutionProfile::PlayerMonstersImpV1
                | OriginalExecutionProfile::PlayerMonstersMapScaleV1
                | OriginalExecutionProfile::PlayerMonstersHeightsV1
                | OriginalExecutionProfile::PlayerMonstersDiagonalV1
                | OriginalExecutionProfile::PlayerMonstersRealMapV1
                | OriginalExecutionProfile::PlayerMonstersSpecialsV1
                | OriginalExecutionProfile::PlayerMonstersMoversV1
                | OriginalExecutionProfile::PlayerMonstersE1M1V1
                | OriginalExecutionProfile::PlayerMonstersBarrelsV1
                | OriginalExecutionProfile::PlayerMonstersLevelV1
                | OriginalExecutionProfile::PlayerMonstersEpisode1V1
                | OriginalExecutionProfile::PlayerMonstersEpisode1V2
        )
    }

    /// Fixed genuine player0 and at most one map-THING Demon, including autonomous
    /// original melee. Existing grounded movement/travel and ownership bounds apply.
    pub fn original_player_demon_melee_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let registry =
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonMeleeV1)?;
        Self::with_original_registry(map, limit, registry, ActorOptions::default(), context)
            .map_err(Into::into)
    }

    /// Derived source roster observation for the fixed-active capability.
    /// Activity persists after death. Incomplete setup can have an absent body;
    /// this observation grants neither scanning nor execution permission.
    /// Historical fixture registration remains available through its old API.
    pub fn original_player_roster(&self) -> Result<[PerceptionPlayer; 4], SimError> {
        if !self.uses_active_player_roster() {
            return Err(PlayerBodyError::Profile.into());
        }
        Ok(view(&self.original_player(PlayerSlot::One)?))
    }

    pub(crate) fn map_demon_limit(&self) -> usize {
        if let Some(bounds) = self.world.episode1 {
            // The Episode 1 policy's roster is the map's own `K` (decision H4).
            bounds.monsters
        } else if self.player_policy().has_monsters() {
            MAX_PLAYER_MONSTERS
        } else if self.uses_active_player_roster() {
            1
        } else {
            MAX_MAP_DEMONS
        }
    }

    pub(crate) fn preflight_original_roster(&self) -> Result<(), SimError> {
        let player = self.original_player(PlayerSlot::One)?;
        let id = player.mo.ok_or(PerceptionError::Roster)?;
        // This is the roster relationship guard. The existing map/operation
        // preflight validates every actor's immutable definition and state role;
        // do not repeat that allocating graph validation at each roster check.
        let actor = self.perception_actor(id)?;
        let life = actor.life().ok_or(PerceptionError::Roster)?;
        if life.actor_type.0 != OriginalActorType::MT_PLAYER.ordinal()
            || player.health != life.health.max(0)
            || (player.state == PlayerBodyState::Dead) != (life.health <= 0)
            || self.sound_targets.iter().any(|target| {
                target.is_some()
                    && !(self.player_policy().has_player_weapons() && *target == Some(id))
            })
        {
            return Err(PerceptionError::Roster.into());
        }
        Ok(())
    }
}

/// Select a roster source without copying historical arrays or persisting a
/// second mutable player record. Original fields are read at each action; the
/// immutable capability is copied from PlayerRuntime only for this operation.
#[derive(Clone, Copy)]
pub(crate) enum PerceptionRoster<'a> {
    Historical(&'a [PerceptionPlayer; 4]),
    OriginalPlayer(OriginalExecutionProfile),
}

impl PerceptionRoster<'_> {
    pub(crate) fn allows_player_melee(self) -> bool {
        matches!(
            self,
            Self::OriginalPlayer(
                OriginalExecutionProfile::PlayerDemonMeleeV1
                    | OriginalExecutionProfile::PlayerDemonPistolV1
                    | OriginalExecutionProfile::PlayerDemonWeaponsV1
                    | OriginalExecutionProfile::PlayerDemonPickupsV1
                    | OriginalExecutionProfile::PlayerDemonArmorV1
                    | OriginalExecutionProfile::PlayerDemonBonusesV1
                    | OriginalExecutionProfile::PlayerMonstersV1
                    | OriginalExecutionProfile::PlayerMonstersImpV1
                    | OriginalExecutionProfile::PlayerMonstersMapScaleV1
                    | OriginalExecutionProfile::PlayerMonstersHeightsV1
                    | OriginalExecutionProfile::PlayerMonstersDiagonalV1
                    | OriginalExecutionProfile::PlayerMonstersRealMapV1
                    | OriginalExecutionProfile::PlayerMonstersSpecialsV1
                    | OriginalExecutionProfile::PlayerMonstersMoversV1
                    | OriginalExecutionProfile::PlayerMonstersE1M1V1
                    | OriginalExecutionProfile::PlayerMonstersBarrelsV1
                    | OriginalExecutionProfile::PlayerMonstersLevelV1
                    | OriginalExecutionProfile::PlayerMonstersEpisode1V1
                    | OriginalExecutionProfile::PlayerMonstersEpisode1V2
            )
        )
    }
}

pub(crate) fn scan_current<A: lifecycle::ActorAccess>(
    host: &lifecycle::LifeHost<'_, A>,
    id: ActorId,
    allaround: bool,
) -> Result<perception::ScanPlan, SimError> {
    let input = host.perception.ok_or(PerceptionError::Profile)?;
    let derived;
    let roster = match input.players {
        PerceptionRoster::Historical(roster) => roster,
        PerceptionRoster::OriginalPlayer(_) => {
            derived = view(host.player.as_deref().ok_or(PerceptionError::Roster)?);
            &derived
        }
    };
    perception::scan(
        host.world,
        host.sectors,
        roster,
        &|actor| host.access.get(actor).copied(),
        id,
        allaround,
        input.checked,
    )
}

#[cfg(test)]
#[path = "player_roster_tests.rs"]
mod tests;
