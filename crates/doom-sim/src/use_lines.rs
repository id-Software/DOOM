//! Use, switches and the exit of the movers policy (M3h8 note §2.1, §2.3, §2.8):
//! `P_UseLines`, `PTR_UseTraverse`, `P_UseSpecialLine`, `P_ChangeSwitchTexture`,
//! `P_StartButton`, the button countdown of `P_UpdateSpecials` and `G_ExitLevel`.
//!
//! Switch textures themselves are presentation and not simulated: load rule 9
//! (decision N9a, `mover_rules::switch_match`) guarantees that every 11 and 63 line
//! carries an episode-1 switch texture on its front side, so `P_ChangeSwitchTexture`
//! always finds its `switchlist` entry, makes its sound and (for 63) starts a button.
//! The button list is kept as far as the simulation needs it: a switch sound plays at
//! `buttonlist[0].soundorg` (the first slot's line front sector, or a global sound
//! when that slot is free) and a finished button's sound names its slot. Each slot's
//! `btimer`, and the line and `soundorg` sector of a running button, are DSIM35 state
//! (decision N9); `where` and `btexture` only pick the restored texture.
use crate::{
    ActorId, Event, MoverEvent, SimError, StaticWorld, StoreError,
    lifecycle::ActorStateEvent,
    mover_rules::{LINE_DOOR_DR, LINE_DOOR_SR, LINE_EXIT, MAX_BUTTONS},
    movers::{DoorKind, FloorKind, MoverAccess, PlatType, Starter},
    planes::SoundOrigin,
};
use doom_types::Fixed;

/// p_spec.h `BUTTONTIME`: tics a reusable switch stays pressed.
const BUTTONTIME: u8 = 35;
/// p_map.c `USERANGE` (64 units).
const USERANGE: i32 = 64;
/// doomdata.h `ML_SECRET`: monsters never use such a line.
const ML_SECRET: u16 = 32;

/// One running `button_t`, as far as the event stream needs it: the line (the
/// deduplication key), the remaining timer and the front sector its `soundorg`
/// names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Button {
    /// `line`.
    pub line: u16,
    /// `btimer`, 1..=35.
    pub timer: u8,
    /// `soundorg`: `&line->frontsector->soundorg`.
    pub sector: u16,
}
/// `buttonlist[MAXBUTTONS]`; a free slot is `None` (`btimer == 0`, the slot zeroed
/// by `P_UpdateSpecials`).
pub(crate) type Buttons = [Option<Button>; MAX_BUTTONS];

/// What a use requested beyond the store and the event stream, applied by the
/// caller with the staged tables: the S1 exit line's clear and `G_ExitLevel`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct UseOutcome {
    /// A line whose runtime special became 0 (`P_ChangeSwitchTexture` of a
    /// non-reusable switch).
    pub cleared: Option<u16>,
    /// `G_ExitLevel` ran (`gameaction = ga_completed`).
    pub exit: bool,
    /// `G_SecretExitLevel` ran (`secretexit = true`; `exit` is set too).
    pub secret: bool,
}

/// `S_StartSound` of a switch: at `buttonlist[0].soundorg`, a global sound when that
/// slot is free (p_switch.c:228, 240, 252: `buttonlist->soundorg`, read before
/// `P_StartButton` stores the new button). Policies before wire 25 only.
#[cfg(test)]
fn switch_sound(events: &mut Vec<Event>, buttons: &Buttons, sound: u16) {
    events.push(match buttons[0] {
        Some(button) => Event::Mover(MoverEvent::SectorSound {
            origin: SoundOrigin::Sector,
            index: button.sector,
            sound,
        }),
        None => Event::ActorState(ActorStateEvent::Sound { actor: None, sound }),
    });
}

/// p_switch.c `P_ChangeSwitchTexture` (201–263) with the switch texture found
/// (always, by rule 9, except under the Episode 1 policy, decision D3): a non-reusable line's special is cleared **before** the exit-sound test,
/// so the exit switch plays `sfx_swtchn`; the sound plays at `buttonlist[0]`; a
/// reusable line then starts its button (`P_StartButton`). Returns the special after.
/// The unit tests' entry; the starters run [`change_switch_texture_at`].
#[cfg(test)]
pub(crate) fn change_switch_texture(
    world: &StaticWorld,
    events: &mut Vec<Event>,
    buttons: &mut Buttons,
    line: u16,
    special: i16,
    use_again: bool,
) -> Result<i16, SimError> {
    change_switch(
        world,
        buttons,
        line,
        special,
        use_again,
        |buttons, sound| {
            switch_sound(events, buttons, sound);
        },
    )
}
/// [`change_switch_texture`] for a starter: under wire 25 a busy slot 0 places
/// the press at that slot's line's front sector origin, read now (the value of
/// `buttonlist[0].soundorg` before `P_StartButton`, M5c decision T12).
fn change_switch_texture_at<M: MoverAccess + ?Sized>(
    starter: &mut Starter<'_, M>,
    buttons: &mut Buttons,
    line: u16,
    special: i16,
    use_again: bool,
) -> Result<i16, SimError> {
    let world = starter.world;
    change_switch(
        world,
        buttons,
        line,
        special,
        use_again,
        |buttons, sound| match buttons[0] {
            Some(button) => starter.sector_sound_id(button.sector, sound),
            None => starter
                .events
                .push(Event::ActorState(ActorStateEvent::Sound {
                    actor: None,
                    sound,
                })),
        },
    )
}
fn change_switch(
    world: &StaticWorld,
    buttons: &mut Buttons,
    line: u16,
    special: i16,
    use_again: bool,
    sound_at: impl FnOnce(&Buttons, u16),
) -> Result<i16, SimError> {
    let after = if use_again { special } else { 0 };
    // Decision D3 (Episode 1 policy): a front side with no episode-1 switch
    // texture is searched in vain (p_switch.c:224–262): the special is cleared
    // above for a non-reusable line, and there is no sound and no button.
    if world.episode1.is_some()
        && crate::mover_rules::line_switch_match(world.map(), usize::from(line)).is_none()
    {
        return Ok(after);
    }
    let sound = crate::lifecycle::source_sound_id(if after == LINE_EXIT {
        "sfx_swtchx"
    } else {
        "sfx_swtchn"
    });
    sound_at(buttons, sound);
    if use_again {
        start_button(world, buttons, line)?;
    }
    Ok(after)
}
/// p_switch.c `P_StartButton` (155–190): nothing if a running button holds `line`;
/// otherwise the first free slot, timer `BUTTONTIME`, sound origin the line's front
/// sector; `I_Error` without a free slot (unreachable under rule 5).
fn start_button(world: &StaticWorld, buttons: &mut Buttons, line: u16) -> Result<(), SimError> {
    if buttons.iter().flatten().any(|b| b.line == line) {
        return Ok(());
    }
    let map = world.map();
    let front = map
        .linedefs()
        .get(usize::from(line))
        .and_then(|l| l.sides[0])
        .ok_or(crate::GeometryError::InvalidReference)?;
    let sector = map.sidedefs()[usize::from(front)].sector;
    let slot = buttons
        .iter_mut()
        .find(|b| b.is_none())
        .ok_or(crate::MapScaleError::MoverCount)?;
    *slot = Some(Button {
        line,
        timer: BUTTONTIME,
        sector,
    });
    Ok(())
}
/// The button half of p_spec.c `P_UpdateSpecials` (1127–1153): every running
/// button counts down; at 0 its texture is restored with `sfx_swtchn` at the address
/// of the slot's `soundorg` field and the slot is zeroed.
///
/// Under wire 25 (`records`) the release is placed by the declared 64-bit reading
/// of that address (M5c decision T12; `button_t` is 32 bytes with `soundorg` at 24,
/// and `mobj_t`'s `x`, `y` at 24, 28, note §1.2.6.3): x is slot i+1's `btimer` as
/// it stands at this point of the loop (slots are visited in increasing order, so
/// slot i+1 is not yet decremented, p_spec.c:1129-1132) and y is 0 (the padding
/// after it); the listener is read at the end of the tic. The release of slot 15
/// would read past `buttonlist` and refuses the tic
/// ([`crate::sound::SoundError::ButtonSlot15`]); a refusal leaves `buttons` to be
/// dropped with the plan.
pub(crate) fn update_buttons(
    buttons: &mut Buttons,
    events: &mut Vec<Event>,
    records: bool,
    listener: Option<crate::sound::Listener>,
) -> Result<(), SimError> {
    for slot in 0..buttons.len() {
        // The value slot `slot + 1` holds before this loop reaches it.
        let next_timer = buttons.get(slot + 1).map(|b| b.map_or(0, |b| b.timer));
        let Some(button) = buttons[slot].as_mut() else {
            continue;
        };
        button.timer -= 1;
        if button.timer == 0 {
            let sound = crate::lifecycle::source_sound_id("sfx_swtchn");
            let index = slot as u16;
            events.push(Event::Mover(MoverEvent::SectorSound {
                origin: SoundOrigin::Button,
                index,
                sound,
            }));
            if records {
                let next = next_timer.ok_or(crate::sound::SoundError::ButtonSlot15)?;
                let at = listener.and_then(|l| {
                    l.place(|_| None, [Fixed::from_bits(i32::from(next)), Fixed::ZERO])
                });
                crate::sound::push_placed(
                    events,
                    sound,
                    crate::sound::SoundOrigin::Button(slot as u8),
                    at,
                );
            }
            buttons[slot] = None;
        }
    }
    Ok(())
}

/// Who uses a line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum User {
    /// The player (`thing->player`), with the button list its switches use.
    Player,
    /// A monster (the `P_Move` failure branch).
    Monster,
}

/// p_switch.c `P_UseSpecialLine` (276–653) for the admitted specials: a back-side use
/// returns false; a monster returns false on an `ML_SECRET` line or anything but 1
/// (and, Episode 1 policy, the locked D1 lines 32–34, where `EV_VerticalDoor`
/// returns at once for it); 1 (and 26–28, 31–34) runs `EV_VerticalDoor`; 11 changes
/// the switch (clearing the line) and runs `G_ExitLevel`; 63 runs
/// `EV_DoDoor(normal)` and changes the switch (starting its button) only if a door
/// was created. Under the Episode 1 policy the S1 switches 7 (stairs), 9 (donut),
/// 18 (`raiseFloorToNearest`), 20 (`raiseToNearestAndChange`), 23
/// (`lowerFloorToLowest`) and 103 (`open`) change their switch (clearing the line)
/// only if their call returned nonzero, 51 changes it unconditionally and runs
/// `G_SecretExitLevel`, and the SR switches 62 (`downWaitUpStay`) and 70
/// (`turboLower`) change theirs (starting the button) only on success. Every other
/// front-side use does nothing and returns true. Records the use (sub-tag 2) after
/// the line action's records.
#[expect(
    clippy::too_many_arguments,
    reason = "the source call's arguments plus the staged button list"
)]
pub(crate) fn use_special_line<M: MoverAccess + ?Sized>(
    starter: &mut Starter<'_, M>,
    buttons: Option<&mut Buttons>,
    user: ActorId,
    kind: User,
    line: u16,
    side: u8,
    special: i16,
    outcome: &mut UseOutcome,
) -> Result<bool, SimError> {
    let flags = starter
        .world
        .map()
        .linedefs()
        .get(usize::from(line))
        .ok_or(crate::GeometryError::InvalidReference)?
        .flags;
    let mut after = special;
    let monster_line =
        special == LINE_DOOR_DR || (starter.world.episode1.is_some() && matches!(special, 32..=34));
    let result =
        if side != 0 || (kind == User::Monster && (flags & ML_SECRET != 0 || !monster_line)) {
            false
        } else {
            // A switch whose call succeeded: `P_ChangeSwitchTexture(line, use_again)`.
            let mut switch = |starter: &mut Starter<'_, M>,
                              buttons: Option<&mut Buttons>,
                              use_again: bool|
             -> Result<(), SimError> {
                let buttons = buttons.ok_or(crate::PlayerBodyError::Profile)?;
                after = change_switch_texture_at(starter, buttons, line, special, use_again)?;
                if !use_again {
                    outcome.cleared = Some(line);
                }
                Ok(())
            };
            if starter.world.episode1.is_none()
                && !matches!(special, LINE_DOOR_DR | LINE_EXIT | LINE_DOOR_SR)
            {
                // Rule 1 admits nothing else before wire 23: no action.
            } else {
                match special {
                    LINE_DOOR_DR | 26..=28 | 31..=34 => {
                        if starter.vertical_door(line, kind == User::Player)? {
                            after = 0;
                            outcome.cleared = Some(line);
                        }
                    }
                    LINE_EXIT | 51 => {
                        switch(starter, buttons, false)?;
                        // g_game.c G_ExitLevel / G_SecretExitLevel (1009–1018):
                        // secretexit, then gameaction = ga_completed.
                        let secret = special == 51;
                        starter
                            .events
                            .push(Event::Mover(MoverEvent::Exit { secret }));
                        outcome.exit = true;
                        outcome.secret |= secret;
                    }
                    LINE_DOOR_SR => {
                        // The switch changes only if EV_DoDoor created a door.
                        if starter.do_door(line, DoorKind::Normal)? {
                            switch(starter, buttons, true)?;
                        }
                    }
                    7 | 9 | 18 | 20 | 23 | 103 | 62 | 70 => {
                        let done = match special {
                            7 => starter.build_stairs(line)?,
                            9 => starter.do_donut(line)?,
                            18 => starter.do_floor(line, FloorKind::RaiseFloorToNearest)?,
                            20 => starter.do_plat(line, PlatType::RaiseToNearestAndChange)?,
                            23 => starter.do_floor(line, FloorKind::LowerFloorToLowest)?,
                            103 => starter.do_door(line, DoorKind::Open)?,
                            62 => starter.do_plat(line, PlatType::DownWaitUpStay)?,
                            _ => starter.do_floor(line, FloorKind::TurboLower)?,
                        };
                        if done {
                            switch(starter, buttons, matches!(special, 62 | 70))?;
                        }
                    }
                    _ => {}
                }
            }
            true
        };
    #[cfg(test)]
    crate::planes::injected_fault(crate::planes::FaultSite::SpecialUse)?;
    starter.events.push(Event::Mover(MoverEvent::SpecialUse {
        user,
        line,
        side,
        before: special,
        result,
        after,
    }));
    Ok(result)
}

/// p_spec.c `P_ShootSpecialLine` (959–1000) under the Episode 1 policy, from
/// `PTR_ShootTraverse` for a line with a nonzero special (p_map.c:907–908): a
/// non-player acts only on 46; 46 (GR) runs `EV_DoDoor(open)` and then
/// `P_ChangeSwitchTexture(line, 1)` whatever the call returned, so every hit
/// restarts the button when none holds the line (decision D3: nothing without an
/// episode-1 switch texture). The other admitted specials have no case.
pub(crate) fn shoot_special_line<M: MoverAccess + ?Sized>(
    starter: &mut Starter<'_, M>,
    buttons: Option<&mut Buttons>,
    line: u16,
    special: i16,
) -> Result<(), SimError> {
    if special != 46 {
        return Ok(());
    }
    starter.do_door(line, DoorKind::Open)?;
    let buttons = buttons.ok_or(crate::PlayerBodyError::Profile)?;
    change_switch_texture_at(starter, buttons, line, special, true)?;
    Ok(())
}

/// The `P_ShootSpecialLine` calls of one `P_LineAttack` (Episode 1 policy): every
/// line intercept the trace visited (up to and including the one that stopped
/// it) whose runtime special is nonzero, in visit order, after the trace and
/// before the puff or blood it spawns (the door thinker precedes that actor in the
/// shared order). Nothing under every other policy.
pub(crate) fn shoot_lines<A: crate::lifecycle::ActorAccess>(
    host: &mut crate::lifecycle::LifeHost<'_, A>,
    player: bool,
    trace: &crate::HitscanTrace,
    visited: u32,
) -> Result<(), SimError> {
    if host.world.episode1.is_none() {
        return Ok(());
    }
    for intercept in trace.intercepts.iter().take(visited as usize) {
        let crate::TraceObject::Line(line) = intercept.object else {
            continue;
        };
        let special = host.sectors.line_special(line);
        if special == 0 || (!player && special != 46) {
            continue;
        }
        let listener = host.listener();
        let store = host.access.movers().ok_or(StoreError::UnsupportedThinker)?;
        shoot_special_line(
            &mut Starter {
                world: host.world,
                sectors: host.sectors,
                store,
                events: &mut *host.events,
                specials: None,
                player: None,
                listener,
            },
            host.buttons.as_deref_mut(),
            line,
            special,
        )?;
    }
    Ok(())
}

/// p_map.c `P_UseLines` (1130–1148) and `PTR_UseTraverse` (1095–1123): a 64-unit
/// line trace along the player's angle (`P_PathTraverse`, lines only, by increasing
/// fraction up to 1). A line without a special continues if its opening is positive
/// and otherwise plays `sfx_noway` at the player and stops; a line with any special
/// is used from the side the player's center is on, and the trace stops whatever
/// the use returned.
pub(crate) fn use_lines<M: MoverAccess + ?Sized>(
    starter: &mut Starter<'_, M>,
    buttons: &mut Buttons,
    player: ActorId,
    position: [Fixed; 2],
    angle: u32,
    limit: Option<usize>,
) -> Result<UseOutcome, SimError> {
    let mut outcome = UseOutcome::default();
    starter
        .events
        .push(Event::Mover(MoverEvent::Use { player }));
    let fine = (angle >> 19) as usize;
    let reach = |table: i32| Fixed::from_bits(USERANGE.wrapping_mul(table));
    let end = [
        position[0] + reach(crate::FINE_SINE[fine + 2048]),
        position[1] + reach(crate::FINE_SINE[fine]),
    ];
    let hits = crate::movement::path(
        starter.world,
        position,
        end,
        limit,
        starter.world.checked_arithmetic,
    )
    .map_err(|e| match e {
        crate::movement::PathError::Overflow => crate::MapScaleError::Intercepts,
        crate::movement::PathError::Arithmetic => crate::MapScaleError::Arithmetic,
    })?;
    for (frac, line) in hits {
        if frac > Fixed::ONE {
            break;
        }
        let line = line as u16;
        let special = starter.sectors.line_special(line);
        let continues = if special == 0 {
            // P_LineOpening: openrange 0 for a one-sided line.
            let range =
                crate::multisector::opening(starter.world, starter.sectors, usize::from(line))
                    .map_or(Fixed::ZERO, |o| o.range);
            if range <= Fixed::ZERO {
                starter
                    .events
                    .push(Event::ActorState(ActorStateEvent::Sound {
                        actor: Some(player),
                        sound: crate::lifecycle::source_sound_id("sfx_noway"),
                    }));
                false
            } else {
                true
            }
        } else {
            let side = starter.world.lines[usize::from(line)].point_side(position);
            use_special_line(
                starter,
                Some(&mut *buttons),
                player,
                User::Player,
                line,
                side,
                special,
                &mut outcome,
            )?;
            false
        };
        starter.events.push(Event::Mover(MoverEvent::UseIntercept {
            line,
            frac,
            continues,
        }));
        if !continues {
            break;
        }
    }
    Ok(outcome)
}

#[cfg(test)]
#[path = "use_lines_tests.rs"]
mod tests;
