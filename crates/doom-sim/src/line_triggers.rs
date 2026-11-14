//! Line triggers of the movers policy (M3h8 note §2.2, §6): the `spechit` window of
//! every `P_CheckPosition`, the reverse crossing loop of `P_TryMove` and
//! `P_CrossSpecialLine` for the admitted line specials.
//!
//! `spechit` is not a separate array: it is the subsequence of a position check's
//! `openings` (the two-sided lines that did not block, in visit order) whose runtime
//! special is nonzero, exactly the lines `PIT_CheckLine` appends. A ninth entry is
//! the source's write past `spechit[8]` (undefined), refused as
//! [`MapScaleError::SpecialCross`]; rule 6 makes it unreachable on admitted maps.
//! A view without line specials (every policy before movers) collects nothing.
use crate::{
    ActorId, Event, MapScaleError, MoverEvent, SimError, StaticWorld,
    mover_rules::{
        LINE_DOOR_BLAZE, LINE_FLOOR_LOWEST, LINE_FLOOR_TURBO, LINE_LIFT, MAX_SPECIAL_CROSS,
    },
    movers::{DoorKind, FloorKind, MoverAccess, PlatType, Starter},
    multisector::SectorMoveAttempt,
    staging::Sectors,
};
use doom_types::Fixed;

/// The `spechit` entries of one `P_CheckPosition` (at most `MAXSPECIALCROSS`), in
/// append order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SpecHit {
    lines: [u16; MAX_SPECIAL_CROSS],
    len: u8,
}
impl SpecHit {
    /// `numspechit`.
    pub(crate) const fn len(&self) -> usize {
        self.len as usize
    }
    /// Whether `numspechit` is 0.
    pub(crate) const fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// The entries in append order.
    pub(crate) fn lines(&self) -> &[u16] {
        &self.lines[..self.len()]
    }
}

/// The ninth `spechit` write of one position check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SpecialCrossOverflow;
impl From<SpecialCrossOverflow> for SimError {
    fn from(_: SpecialCrossOverflow) -> Self {
        MapScaleError::SpecialCross.into()
    }
}

/// `spechit` of `attempt`: its openings whose runtime special (read through
/// `sectors`, with the lines in `cleared` read as 0) is nonzero. Empty for a view
/// without line specials.
pub(crate) fn spechit(
    sectors: Sectors<'_>,
    attempt: &SectorMoveAttempt,
    cleared: &[u16],
) -> Result<SpecHit, SpecialCrossOverflow> {
    let mut hit = SpecHit::default();
    if !sectors.has_line_specials() {
        return Ok(hit);
    }
    for visit in &attempt.openings {
        if special(sectors, visit.line, cleared) == 0 {
            continue;
        }
        let len = hit.len();
        let Some(slot) = hit.lines.get_mut(len) else {
            crate::level_stats::observe_spechit(len + 1);
            return Err(SpecialCrossOverflow);
        };
        *slot = visit.line;
        hit.len += 1;
    }
    crate::level_stats::observe_spechit(hit.len());
    Ok(hit)
}
/// The runtime special of `line`, 0 if it is in `cleared`.
pub(crate) fn special(sectors: Sectors<'_>, line: u16, cleared: &[u16]) -> i16 {
    if cleared.contains(&line) {
        0
    } else {
        sectors.line_special(line)
    }
}

/// One crossing found by the reverse loop of `P_TryMove` (p_map.c:502–512).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Crossing {
    /// The line.
    pub line: u16,
    /// `oldside`: `P_PointOnLineSide` of the position before the move.
    pub side: u8,
    /// The line's runtime special when the loop reached it (nonzero).
    pub special: i16,
}
/// The crossing loop of a successful `P_TryMove` from `old` to `new`: every
/// `spechit` entry in reverse whose side differs between the two centers and whose
/// special is still nonzero, in call order. `cleared` holds lines cleared by earlier
/// crossings of the same motion that are not yet staged.
pub(crate) fn crossings<'a>(
    world: &'a StaticWorld,
    sectors: Sectors<'a>,
    hit: &'a SpecHit,
    old: [Fixed; 2],
    new: [Fixed; 2],
    cleared: &'a [u16],
) -> impl Iterator<Item = Crossing> + 'a {
    hit.lines().iter().rev().filter_map(move |&line| {
        let geometry = &world.lines[usize::from(line)];
        let side = geometry.point_side(old);
        let special = special(sectors, line, cleared);
        (geometry.point_side(new) != side && special != 0).then_some(Crossing {
            line,
            side,
            special,
        })
    })
}

/// Who crosses a line, as `P_CrossSpecialLine` distinguishes it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Crosser {
    /// `thing->player`.
    Player,
    /// Any other non-missile thing (monsters, corpses). The admitted missile
    /// (`MT_TROOPSHOT`) returns before the switch: its crossings are recorded by
    /// `projectiles::try_move` with the special unchanged.
    Monster,
}
/// Whether `P_CrossSpecialLine` clears `special` for `crosser` (the admitted W1
/// lines crossed by a player; cleared even when their `EV_*` started nothing):
/// 36, 38 and 109, and under the Episode 1 policy 2, 5, 8, 16, 22 and 35.
pub(crate) fn clears(crosser: Crosser, special: i16) -> bool {
    crosser == Crosser::Player
        && matches!(
            special,
            LINE_FLOOR_TURBO | LINE_FLOOR_LOWEST | LINE_DOOR_BLAZE | 2 | 5 | 8 | 16 | 22 | 35
        )
}

/// p_spec.c `P_CrossSpecialLine` (492–950) for the admitted line specials and a
/// non-missile crosser: a monster acts only on 88 (`EV_DoPlat`); a player's switch has
/// 36 (`turboLower`), 38 (`lowerFloorToLowest`) and 109 (`blazeOpen`), each cleared
/// after its `EV_*`, and 88, kept; 1, 63, 11 and 48 have no case. Under the Episode
/// 1 policy the player's switch also has the W1 lines 2 (`open`), 5
/// (`raiseFloor`), 8 (`EV_BuildStairs`), 16 (`close30ThenOpen`), 22
/// (`raiseToNearestAndChange`) and 35 (`EV_LightTurnOn(35)`), each cleared after
/// its call, and the WR lines 76 (`close30ThenOpen`), 82 (`lowerFloorToLowest`),
/// 86 (`open`), 90 (`normal`), 91 (`raiseFloor`) and 98 (`turboLower`), kept.
/// Records the crossing (DSIM35 sub-tag 3) after the line action's records and
/// returns the special after the call, which the caller stages when it changed.
pub(crate) fn cross<M: MoverAccess + ?Sized>(
    starter: &mut Starter<'_, M>,
    actor: ActorId,
    crosser: Crosser,
    crossing: Crossing,
) -> Result<i16, SimError> {
    let Crossing {
        line,
        side,
        special,
    } = crossing;
    // Special 97 is `crate::teleport::cross`'s alone (wire 24): it needs the actor
    // store, the damage service and the RNG, which a starter does not hold.
    if special == crate::teleport::LINE_TELEPORT {
        return Err(crate::PursuitError::Specials.into());
    }
    if matches!(
        special,
        2 | 5 | 8 | 16 | 22 | 35 | 76 | 82 | 86 | 90 | 91 | 98
    ) && starter.world.episode1.is_none()
    {
        // Defence in depth: rule 1 admits these lines only under wire 23.
        return Err(crate::PursuitError::Specials.into());
    }
    match (crosser, special) {
        (_, LINE_LIFT) => {
            starter.do_plat(line, PlatType::DownWaitUpStay)?;
        }
        (Crosser::Monster, _) => {}
        (Crosser::Player, LINE_FLOOR_TURBO | 98) => {
            starter.do_floor(line, FloorKind::TurboLower)?;
        }
        (Crosser::Player, LINE_FLOOR_LOWEST | 82) => {
            starter.do_floor(line, FloorKind::LowerFloorToLowest)?;
        }
        (Crosser::Player, LINE_DOOR_BLAZE) => {
            starter.do_door(line, DoorKind::BlazeOpen)?;
        }
        (Crosser::Player, 2 | 86) => {
            starter.do_door(line, DoorKind::Open)?;
        }
        (Crosser::Player, 16 | 76) => {
            starter.do_door(line, DoorKind::Close30ThenOpen)?;
        }
        (Crosser::Player, 90) => {
            starter.do_door(line, DoorKind::Normal)?;
        }
        (Crosser::Player, 5 | 91) => {
            starter.do_floor(line, FloorKind::RaiseFloor)?;
        }
        (Crosser::Player, 8) => {
            starter.build_stairs(line)?;
        }
        (Crosser::Player, 22) => {
            starter.do_plat(line, PlatType::RaiseToNearestAndChange)?;
        }
        (Crosser::Player, 35) => {
            starter.light_turn_on(line, 35)?;
        }
        (Crosser::Player, _) => {}
    }
    let after = if clears(crosser, special) { 0 } else { special };
    starter.events.push(Event::Mover(MoverEvent::Crossed {
        actor,
        line,
        side,
        before: special,
        after,
    }));
    Ok(after)
}

#[cfg(test)]
#[path = "line_triggers_tests.rs"]
mod tests;
