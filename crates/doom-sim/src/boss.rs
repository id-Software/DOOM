//! `A_BossDeath` under wire 24 (`PlayerMonstersEpisode1V2`, M4b step 5; decision
//! I9): p_enemy.c:1609–1757 for the registered/retail branch, episode 1.
//!
//! The action runs on the Baron's entry into `S_BOSS_DIE7` (info.c:684; tics −1,
//! so once per corpse). The source's checks, in order, as they apply to the only
//! type and episode wire 24 admits:
//!
//! 1. `gamemode == commercial` is false (the genuine engine runs the four-episode
//!    `DOOM.WAD` as registered; M4b note §7); `switch (gameepisode)` case 1:
//!    `gamemap != 8` returns, then `mo->type != MT_BRUISER` returns
//!    (p_enemy.c:1629–1635). Only the Baron's `S_BOSS_DIE7` names the action, so
//!    the type test cannot fail for an admitted actor.
//! 2. The first player in game with `players[i].health > 0` (the `player_t`
//!    health, p_enemy.c:1683–1688); none returns.
//! 3. A walk of the whole thinker list for another mobj of the same type with
//!    `health > 0` (p_enemy.c:1692–1705; removed thinkers skipped); one returns.
//!    The walk has no side effect, so only its answer matters.
//! 4. `junk.tag = 666; EV_DoFloor (&junk, lowerFloorToLowest); return;`
//!    (p_enemy.c:1729–1734): never `G_ExitLevel` on episode 1.
//!
//! The work is bounded by the actor store (one read per live actor) and the
//! sectors tagged 666 (one floor each, in the sector-thinker class the Episode 1
//! policy already sizes for one mover per sector). A refusal latches its error and
//! the staged tic is discarded, as for every other mid-tic refusal.
use crate::lifecycle::{ActorAccess, LifeHost};
use crate::{
    ActorId, Event, MoverEvent, PlayerBodyError, SimError, StoreError, identity::OriginalActorType,
};

/// The map whose last Baron's death lowers the tag-666 floor (p_enemy.c:1630).
pub(crate) const BOSS_MAP: u8 = 8;

/// What one `A_BossDeath` call did (DSIM40 mover sub-tag 17).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Outcome {
    /// `gamemap != 8`.
    NotBossMap = 0,
    /// No player in game has `health > 0`.
    NoLivePlayer = 1,
    /// Another Baron in the thinker list has `health > 0`.
    OtherBossAlive = 2,
    /// The tag-666 `EV_DoFloor (lowerFloorToLowest)` call was made.
    FloorCalled = 3,
}

/// `A_BossDeath` (wire 24). Any refusal is latched and discards the staged tic.
pub(crate) fn boss_death<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) {
    if let Err(error) = boss_death_checked(h, id) {
        if !h.sight_failed() {
            h.fail_sight(error);
        }
    }
}

fn boss_death_checked<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
) -> Result<Outcome, SimError> {
    let player = h.player.as_deref().ok_or(PlayerBodyError::Profile)?;
    if !player.policy.has_boss_death() {
        return Err(PlayerBodyError::Unsupported.into());
    }
    let map = h.world.map_number.ok_or(PlayerBodyError::Profile)?;
    let life = h
        .access
        .get(id)
        .and_then(crate::Actor::life)
        .ok_or(PlayerBodyError::Invariant)?;
    let ty = life.actor_type;
    if ty.0 != OriginalActorType::MT_BRUISER.ordinal() {
        // Only `S_BOSS_DIE7` names the action among the admitted types.
        return Err(PlayerBodyError::Unsupported.into());
    }
    let health = player.health;
    let outcome = decide(map, health, || other_alive(h, id, ty))?;
    if outcome == Outcome::FloorCalled {
        let store = h.access.movers().ok_or(StoreError::UnsupportedThinker)?;
        let mut starter = crate::movers::Starter {
            world: h.world,
            sectors: h.sectors,
            store,
            events: &mut *h.events,
            specials: None,
            player: None,
        };
        starter.boss_floor()?;
    }
    h.events.push(Event::Mover(MoverEvent::BossDeath {
        actor: id,
        outcome: outcome as u8,
    }));
    Ok(outcome)
}

/// The source's checks in order (p_enemy.c:1629–1705): the map, then the live
/// player (`players[0].health`), then the thinker walk, run only when the first
/// two pass, as the source returns before it.
fn decide(
    map: u8,
    player_health: i32,
    other_alive: impl FnOnce() -> Result<bool, SimError>,
) -> Result<Outcome, SimError> {
    Ok(if map != BOSS_MAP {
        Outcome::NotBossMap
    } else if player_health <= 0 {
        Outcome::NoLivePlayer
    } else if other_alive()? {
        Outcome::OtherBossAlive
    } else {
        Outcome::FloorCalled
    })
}

/// Whether a mobj other than `id` of type `ty` with `health > 0` is in the
/// thinker list (p_enemy.c:1692–1705): the nonpending actors in thinker order, as
/// a removed thinker's function is −1 (p_tick.c:83) and fails the
/// `P_MobjThinker` test.
fn other_alive<A: ActorAccess>(
    h: &LifeHost<'_, A>,
    id: ActorId,
    ty: crate::ActorTypeId,
) -> Result<bool, SimError> {
    let order = h
        .access
        .live_order()
        .ok_or(StoreError::UnsupportedThinker)?;
    Ok(order.into_iter().any(|other| {
        other != id
            && h.access
                .get(other)
                .and_then(crate::Actor::life)
                .is_some_and(|l| l.actor_type == ty && l.health > 0)
    }))
}

#[cfg(test)]
#[path = "baron_tests.rs"]
mod tests;
