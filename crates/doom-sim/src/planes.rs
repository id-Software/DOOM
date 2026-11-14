//! Moving planes of the movers policy (M3h8 note §2.7): `T_MovePlane`,
//! `P_ChangeSector`, `PIT_ChangeSector` and `P_ThingHeightClip`.
//!
//! Heights are written only through the plan's staged copy
//! ([`StagedWrites::write_sector`]); every clip after a write reads the fresh view, so
//! later thinkers of the same pass see the moved plane and a refused tic drops it.
//! `crush` is false for every admitted mover, so the crush damage and blood branch of
//! `PIT_ChangeSector` is unreachable and not implemented. No path draws `P_Random`.
use crate::{
    Actor, ActorId, Event, SectorState, SimError, Solidity, StaticWorld, StoreError,
    ThinkerContext,
    lifecycle::LifeHost,
    movement::{self, SpatialIndex},
    original_player::{MF_SHOOTABLE, MF_SOLID, PlayerRuntime},
    staging::StagedWrites,
};
use doom_types::{DoomRng, Fixed};

/// `T_MovePlane`'s `floorOrCeiling`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Plane {
    /// 0: the floor.
    Floor = 0,
    /// 1: the ceiling.
    Ceiling = 1,
}
/// `result_e` of `T_MovePlane`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveResult {
    /// The plane moved by its speed.
    Ok = 0,
    /// The step was refused by `P_ChangeSector` and undone.
    Crushed = 1,
    /// The plane reached its destination (or stopped short of it, refused).
    PastDest = 2,
}
/// `PIT_ChangeSector`'s outcome for one thing (the DSIM35 clip outcome codes).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipOutcome {
    /// 0: `P_ThingHeightClip` succeeded.
    Fits = 0,
    /// 1: a thing with `health <= 0` became `S_GIBS` (height and radius 0).
    Gibs = 1,
    /// 2: an `MF_DROPPED` thing was removed.
    DroppedRemoved = 2,
    /// 3: a non-shootable thing was left as it is.
    LeftStuck = 3,
    /// 4: a shootable thing does not fit: `nofit`.
    NoFit = 4,
}

/// `p_mobj.h` `MF_DROPPED`.
const MF_DROPPED: u32 = crate::world_pickups::MF_DROPPED;

/// Whether `actor` is in exactly the state `PIT_ChangeSector` leaves a corpse it
/// crunches to giblets (p_map.c:1269–1279): `health <= 0`, `S_GIBS` (sprite, frame
/// and `tics == -1` from its descriptor), `MF_SOLID` cleared, height and radius 0.
/// Nothing else in the admitted domain makes a height or radius 0, and `S_GIBS`
/// has no action and no successor (info.c `S_GIBS`: `-1` tics, `S_NULL`), so a
/// gibbed thing keeps this state for the rest of the level. The movers policy's
/// witnesses admit it in place of the type's own death states and dimensions.
pub(crate) fn is_gibs(actor: &Actor) -> bool {
    let Some(life) = actor.life() else {
        return false;
    };
    let gibs = crate::identity::OriginalState::S_GIBS;
    let descriptor = gibs.descriptor();
    life.health <= 0
        && life.state == Some(crate::StateId(gibs.ordinal()))
        && life.sprite == descriptor.sprite.ordinal()
        && life.frame == descriptor.frame
        && life.tics == -1
        && life.flags & MF_SOLID == 0
        && actor.solidity == Solidity::NonSolid
        && actor.spec.height == Fixed::ZERO
        && actor.spec.radius == Fixed::ZERO
}

/// Everything a moving plane touches during the thinker pass: the committed sector
/// table with the plan's staged writes, and the actor plan its clips change.
pub(crate) struct PlaneHost<'a, 'b> {
    pub committed: &'a [SectorState],
    /// The committed runtime line specials, read through `writes`.
    pub committed_lines: &'a [i16],
    pub writes: &'a mut StagedWrites,
    pub access: &'a mut ThinkerContext<'b, Actor>,
    pub player: Option<&'a mut PlayerRuntime>,
    pub statistics: Option<&'a mut crate::LevelStatistics>,
    pub perception: Option<crate::perception::PerceptionInputs<'a>>,
    pub registry: &'a crate::ActorRegistry,
    pub world: &'a StaticWorld,
    pub options: crate::ActorOptions,
    pub rng: &'a mut DoomRng,
    pub spatial: &'a mut SpatialIndex,
    pub bindings: &'a mut [Option<ActorId>; 4],
    pub events: &'a mut Vec<Event>,
    pub boundary: Option<&'a crate::player_map_scale::Boundary>,
    /// The staged runtime sector specials and floor flats (the specials policy),
    /// written by a finished `donutRaise` floor; `None` without them.
    pub specials: Option<&'a mut [crate::sector_thinkers::SectorSpecial]>,
}
impl<'b> PlaneHost<'_, 'b> {
    /// The current heights of `sector` under the plan's writes.
    pub(crate) fn sector(&self, sector: u16) -> Result<SectorState, SimError> {
        self.writes
            .view(self.committed, self.committed_lines)
            .get(usize::from(sector))
            .copied()
            .ok_or_else(|| crate::GeometryError::InvalidReference.into())
    }
    /// Stages `height` for `plane` of `sector`. Under the Episode 1 policy
    /// (decision H3) a height outside the ±1,024-unit sector domain
    /// (`map_extent::HEIGHT_UNITS`, the wire-23 bound the dropped ±512 reach
    /// proof kept heights inside) refuses the tic with
    /// [`crate::MapScaleError::Arithmetic`] before the write.
    fn set(&mut self, sector: u16, plane: Plane, height: Fixed) -> Result<(), SimError> {
        if self.world.checked_arithmetic
            && !crate::sight::within(height, u32::from(crate::map_extent::HEIGHT_UNITS))
        {
            return Err(crate::MapScaleError::Arithmetic.into());
        }
        let mut state = self.sector(sector)?;
        match plane {
            Plane::Floor => state.floor = height,
            Plane::Ceiling => state.ceiling = height,
        }
        self.writes.write_sector(self.committed, sector, state)
    }
    /// The shared lifecycle services over the fresh sector view.
    fn life(&mut self) -> LifeHost<'_, &mut ThinkerContext<'b, Actor>> {
        LifeHost {
            specials: self.specials.as_deref(),
            buttons: None,
            statistics: self.statistics.as_deref_mut(),
            player: self.player.as_deref_mut(),
            perception: self.perception,
            access: &mut *self.access,
            registry: self.registry,
            world: self.world,
            sectors: self.writes.view(self.committed, self.committed_lines),
            options: self.options,
            rng: &mut *self.rng,
            spatial: &mut *self.spatial,
            bindings: &mut *self.bindings,
            events: &mut *self.events,
        }
    }

    /// p_floor.c `T_MovePlane` with `crush == false`: the last step sets the plane
    /// to `dest` and, if `P_ChangeSector` refuses, restores it and clips again,
    /// returning `pastdest` either way (a blocked last step stops short); an
    /// ordinary step refused is undone and returns `crushed`, except a rising
    /// ceiling, which never restores (the `#if 0` block). Emits the plane-step
    /// event after its sector-changed events.
    pub(crate) fn move_plane(
        &mut self,
        sector: u16,
        speed: Fixed,
        dest: Fixed,
        plane: Plane,
        direction: i32,
    ) -> Result<MoveResult, SimError> {
        self.move_plane_witnessed(sector, speed, dest, plane, direction, None)
    }
    /// [`Self::move_plane`], refusing with `witness` (when given) as soon as the
    /// step's first `P_ChangeSector` reports `nofit`, before the restore: the stair
    /// witness of decision H7, where the source passes a `crush` it never wrote.
    pub(crate) fn move_plane_witnessed(
        &mut self,
        sector: u16,
        speed: Fixed,
        dest: Fixed,
        plane: Plane,
        direction: i32,
        witness: Option<SimError>,
    ) -> Result<MoveResult, SimError> {
        let height = |s: SectorState| match plane {
            Plane::Floor => s.floor,
            Plane::Ceiling => s.ceiling,
        };
        let before = height(self.sector(sector)?);
        let result = match direction {
            -1 | 1 => {
                // `floorheight -= speed` / `+= speed` (and the same sum in the
                // `pastdest` test), checked under the Episode 1 policy.
                let next = if direction == -1 {
                    height_sum(
                        self.world,
                        before,
                        Fixed::from_bits(speed.bits().wrapping_neg()),
                    )?
                } else {
                    height_sum(self.world, before, speed)?
                };
                let last = direction == -1 && next < dest || direction == 1 && next > dest;
                if last {
                    self.set(sector, plane, dest)?;
                    #[cfg(test)]
                    injected_fault(FaultSite::PlaneStep)?;
                    if self.change_sector(sector)? {
                        if let Some(error) = witness {
                            return Err(error);
                        }
                        self.set(sector, plane, before)?;
                        self.change_sector(sector)?;
                    }
                    MoveResult::PastDest
                } else {
                    self.set(sector, plane, next)?;
                    let nofit = self.change_sector(sector)?;
                    #[cfg(test)]
                    injected_fault(FaultSite::PlaneStep)?;
                    if let (true, Some(error)) = (nofit, witness) {
                        return Err(error);
                    }
                    if nofit && !(plane == Plane::Ceiling && direction == 1) {
                        self.set(sector, plane, before)?;
                        self.change_sector(sector)?;
                        MoveResult::Crushed
                    } else {
                        MoveResult::Ok
                    }
                }
            }
            _ => MoveResult::Ok,
        };
        let after = height(self.sector(sector)?);
        self.events.push(Event::Mover(crate::MoverEvent::PlaneStep {
            sector,
            plane,
            direction,
            speed,
            dest,
            before,
            after,
            result,
        }));
        Ok(result)
    }

    /// p_map.c `P_ChangeSector` with `crunch == false`: `PIT_ChangeSector` for every
    /// BLOCKMAP-linked thing whose center lies in the sector's `blockbox` cells, `x`
    /// outer and `y` inner, each cell in link order (most recent first). A thing
    /// removed by an earlier clip of the same call is skipped, as the source's
    /// unlinked `bnext` chain does. Returns `nofit`.
    pub(crate) fn change_sector(&mut self, sector: u16) -> Result<bool, SimError> {
        let bbox = self
            .world
            .sector_block_boxes
            .as_ref()
            .and_then(|b| b.get(usize::from(sector)).copied().flatten())
            .ok_or(crate::MapSpawnError::Context)?;
        let mut visit = Vec::new();
        for x in bbox.left..=bbox.right {
            for y in bbox.bottom..=bbox.top {
                for &id in &self.spatial.order {
                    let actor = self.access.get(id).ok_or(StoreError::InvalidHandle)?;
                    if crate::lifecycle::blocks_cell_index(actor)
                        && movement::cell(self.world, actor.position()) == [x, y]
                    {
                        visit.try_reserve(1).map_err(|_| StoreError::Capacity)?;
                        visit.push(id);
                    }
                }
            }
        }
        let mut nofit = false;
        for id in visit {
            if self.access.store().is_pending(id) != Some(false) {
                continue;
            }
            if self.change_thing(id)? == ClipOutcome::NoFit {
                nofit = true;
            }
        }
        self.events
            .push(Event::Mover(crate::MoverEvent::SectorChanged {
                sector,
                nofit,
            }));
        Ok(nofit)
    }

    /// p_map.c `PIT_ChangeSector` (1257–1311) without the unreachable crush branch.
    fn change_thing(&mut self, id: ActorId) -> Result<ClipOutcome, SimError> {
        let outcome = if self.height_clip(id)? {
            ClipOutcome::Fits
        } else {
            let life = self
                .access
                .get(id)
                .and_then(Actor::life)
                .ok_or(crate::ActorError::InvalidType)?;
            if life.health <= 0 {
                let gibs = crate::StateId(crate::identity::OriginalState::S_GIBS.ordinal());
                self.life().enter(id, gibs);
                let actor = self.access.get_mut(id).ok_or(StoreError::InvalidHandle)?;
                if let Some(life) = actor.life.as_mut() {
                    life.flags &= !MF_SOLID;
                }
                actor.solidity = Solidity::NonSolid;
                actor.spec.height = Fixed::ZERO;
                actor.spec.radius = Fixed::ZERO;
                ClipOutcome::Gibs
            } else if life.flags & MF_DROPPED != 0 {
                self.life().mark(id);
                ClipOutcome::DroppedRemoved
            } else if life.flags & MF_SHOOTABLE == 0 {
                ClipOutcome::LeftStuck
            } else {
                ClipOutcome::NoFit
            }
        };
        let actor = self.access.get(id).ok_or(StoreError::InvalidHandle)?;
        self.events
            .push(Event::Mover(crate::MoverEvent::ThingClipped {
                actor: id,
                outcome,
                z: actor.z,
                floor: actor.floor,
                ceiling: actor.ceiling,
            }));
        Ok(outcome)
    }

    /// p_map.c `P_ThingHeightClip` (530–557): a full `P_CheckPosition` at the
    /// thing's own position, its result ignored. The things pass runs first with
    /// the clipped thing as `tmthing`, so a re-clipped player touches every special
    /// thing its box overlaps through the existing touch path; an overlapping solid
    /// thing stops the check, leaving the center subsector's heights, and a
    /// blocking line leaves the heights of the lines before it. An onfloor thing
    /// follows the new floor, any other is only pushed down under the ceiling;
    /// the clip fails when the thing does not fit.
    pub(crate) fn height_clip(&mut self, id: ActorId) -> Result<bool, SimError> {
        let mut actor = *self.access.get(id).ok_or(StoreError::InvalidHandle)?;
        let onfloor = actor.z == actor.floor;
        let position = actor.position();
        let things = {
            let access = &*self.access;
            movement::check_things(self.world, id, &actor, position, self.spatial, &|i| {
                access.get(i).copied()
            })
        };
        let player = self.player.as_ref().is_some_and(|p| p.mo == Some(id));
        if player
            && self
                .player
                .as_ref()
                .is_some_and(|p| p.policy.has_world_pickups())
        {
            let mut touches = Vec::new();
            let sectors = self.writes.view(self.committed, self.committed_lines);
            let p = self
                .player
                .as_deref_mut()
                .ok_or(crate::PlayerBodyError::Binding)?;
            let mut context: &mut ThinkerContext<'b, Actor> = &mut *self.access;
            let mut contacts = crate::world_pickups::Contacts {
                access: &mut context,
                player: p,
                sectors,
                boundary: self.boundary,
            };
            movement::MotionAccess::contacts(
                &mut contacts,
                id,
                &mut actor,
                &things,
                self.spatial,
                &mut touches,
            )?;
            for event in touches {
                if let movement::MotionEvent::Contact(event) = event {
                    self.events.push(event.event());
                }
            }
        }
        let attempt = crate::multisector::check_move_from_things(
            (
                self.world,
                self.writes.view(self.committed, self.committed_lines),
            ),
            &actor,
            position,
            things,
            player,
            false,
        );
        // P_CheckPosition fills spechit here too: only the ninth-entry witness reads
        // it (the clip's result and its entries are otherwise ignored).
        crate::line_triggers::spechit(
            self.writes.view(self.committed, self.committed_lines),
            &attempt,
            &[],
        )?;
        actor.floor = attempt.floor;
        actor.ceiling = attempt.ceiling;
        let height = actor.spec.height;
        if onfloor {
            actor.z = actor.floor;
        } else if actor.z + height > actor.ceiling {
            actor.z = actor.ceiling - height;
        }
        *self.access.get_mut(id).ok_or(StoreError::InvalidHandle)? = actor;
        Ok(actor.ceiling - actor.floor >= height)
    }
}

/// The `fixed_t` height sum `a + b` of a mover (`T_MovePlane`'s step, an
/// `EV_*` destination offset): wrapping as before under the older policies,
/// whose load proof keeps every height within ±512; under the Episode 1 policy
/// (decision H3) an `i32` overflow refuses the tic with
/// [`crate::MapScaleError::Arithmetic`].
pub(crate) fn height_sum(world: &StaticWorld, a: Fixed, b: Fixed) -> Result<Fixed, SimError> {
    match a.bits().checked_add(b.bits()) {
        Some(sum) => Ok(Fixed::from_bits(sum)),
        None if world.checked_arithmetic => Err(crate::MapScaleError::Arithmetic.into()),
        None => Ok(a + b),
    }
}

/// Movers-policy observations (DSIM35 event tag 26, STATE_FORMAT "DSIM v35"). The
/// clip's own position-check candidates are not events; pickups, `S_GIBS` entries
/// and removals keep their existing events, before the clip's record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoverEvent {
    /// Sub-tag 0: `P_UseLines` entry, in the players phase where the use block of
    /// `P_PlayerThink` runs (after the weapon change, before `P_MovePsprites`).
    Use {
        /// The player's body.
        player: ActorId,
    },
    /// Sub-tag 1: `PTR_UseTraverse` on a line, after the records of what that line's
    /// use did.
    UseIntercept {
        /// The line.
        line: u16,
        /// The intercept fraction (16.16).
        frac: Fixed,
        /// Whether the trace continues (a special line or `sfx_noway` stops it).
        continues: bool,
    },
    /// Sub-tag 2: `P_UseSpecialLine`, after the records of the line action.
    SpecialUse {
        /// The user.
        user: ActorId,
        /// The line.
        line: u16,
        /// 0 front, 1 back.
        side: u8,
        /// The runtime special at entry.
        before: i16,
        /// The return value.
        result: bool,
        /// The runtime special at exit.
        after: i16,
    },
    /// Sub-tag 3: `P_CrossSpecialLine` from the crossing loop of a successful
    /// `P_TryMove`, right after that move's attempt record and after the records of
    /// the line action; several crossings of one move follow in reverse `spechit`
    /// order.
    Crossed {
        /// The crosser.
        actor: ActorId,
        /// The line.
        line: u16,
        /// The side of the position before the move.
        side: u8,
        /// The runtime special at entry.
        before: i16,
        /// The runtime special at exit (0 once a W1 line is cleared).
        after: i16,
    },
    /// Sub-tag 7: one `T_MovePlane` call, after its sector-changed records.
    PlaneStep {
        /// Sector index.
        sector: u16,
        /// Floor or ceiling.
        plane: Plane,
        /// −1 down, 1 up.
        direction: i32,
        /// Speed (16.16).
        speed: Fixed,
        /// Destination (16.16).
        dest: Fixed,
        /// Height before the call.
        before: Fixed,
        /// Height after the call.
        after: Fixed,
        /// The call's result.
        result: MoveResult,
    },
    /// Sub-tag 8: a `P_ChangeSector` exit, after its things' clip records.
    SectorChanged {
        /// Sector index.
        sector: u16,
        /// Whether a shootable thing did not fit.
        nofit: bool,
    },
    /// Sub-tag 9: one `PIT_ChangeSector`, with the thing's heights after
    /// `P_ThingHeightClip`.
    ThingClipped {
        /// The clipped thing.
        actor: ActorId,
        /// What `PIT_ChangeSector` did.
        outcome: ClipOutcome,
        /// `z` (16.16).
        z: Fixed,
        /// `floorz` (16.16).
        floor: Fixed,
        /// `ceilingz` (16.16).
        ceiling: Fixed,
    },
    /// Sub-tag 4: the `P_Move` failure branch with a nonempty `spechit` (`movedir =
    /// DI_NODIR`), after the failed attempt and before that monster's uses.
    SpechitBlocked {
        /// The monster.
        actor: ActorId,
        /// `numspechit` (1–8).
        count: u8,
    },
    /// Sub-tag 13: `G_ExitLevel`, inside the exit switch's use after the switch
    /// sound and before that use's sub-tag 2 record.
    Exit {
        /// `secretexit`: true for the S1 51 secret exit (Episode 1 policy).
        secret: bool,
    },
    /// Sub-tag 14: a dead player held `BT_USE` (`PST_REBORN`), the last record of
    /// the players phase.
    Reborn {
        /// The player slot (0).
        slot: u8,
    },
    /// Sub-tag 5: a mover created by an `EV_*` call, at the end of that call, after
    /// its sounds, one per created mover in creation order.
    Started {
        /// The new mover.
        mover: crate::SectorThinkerId,
        /// 5 door, 6 plat, 7 floor.
        kind: u8,
        /// Its sector.
        sector: u16,
    },
    /// Sub-tag 6: an `EV_*` call, right after its sub-tag 5 records.
    Call {
        /// Which starter.
        function: MoverFunction,
        /// The line.
        line: u16,
        /// Door, plat or floor type; the line special for `EV_VerticalDoor`; 0 for
        /// `EV_BuildStairs` and `EV_DoDonut`.
        argument: i32,
        /// The return value (−1 for `EV_VerticalDoor`, which returns nothing).
        result: i32,
        /// Movers created.
        created: u8,
    },
    /// Sub-tag 10: a mover finished (`P_RemoveThinker`); its entry stays until the
    /// next pass.
    Removed {
        /// The mover.
        mover: crate::SectorThinkerId,
    },
    /// Sub-tag 11: a removed mover's entry unlinked by the pass that reached it.
    Unlinked {
        /// The mover.
        mover: crate::SectorThinkerId,
    },
    /// Sub-tag 15 (DSIM40, wire 24): `EV_Teleport` returned 1, after the records
    /// of what the teleport caused (the stomp's combat records, then the two fog
    /// spawns with their sounds) and before the crossing's sub-tag 3 record.
    Teleported {
        /// The teleported thing.
        actor: ActorId,
        /// The line.
        line: u16,
        /// The landing (`MT_TELEPORTMAN`) it moved to.
        landing: ActorId,
        /// Its x, y and z before the teleport (16.16).
        old: [Fixed; 3],
    },
    /// Sub-tag 16 (DSIM40, wire 24): `EV_Teleport` returned 0, before the
    /// crossing's sub-tag 3 record.
    TeleportNotTaken {
        /// The crossing thing.
        actor: ActorId,
        /// The line.
        line: u16,
        /// 0 crossed from the back side, 1 no landing in any sector with the
        /// line's tag, 2 `P_TeleportMove` refused (a non-player met a shootable
        /// thing).
        reason: u8,
    },
    /// Sub-tag 17 (DSIM40, wire 24): `A_BossDeath` ran, on the Baron's entry into
    /// `S_BOSS_DIE7`; for outcome 3 after the floor call's sub-tag 5 and 6 records.
    BossDeath {
        /// The Baron.
        actor: ActorId,
        /// 0 `level.map` is not 8, 1 no live player, 2 another Baron alive, 3 the
        /// tag-666 floor call was made.
        outcome: u8,
    },
    /// Sub-tag 12: a sound request with a sector or button origin.
    SectorSound {
        /// The origin kind.
        origin: SoundOrigin,
        /// The sector index (or button slot).
        index: u16,
        /// The `sfxenum_t` sound.
        sound: u16,
    },
}
/// The `EV_*` starter of a mover-call record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoverFunction {
    /// 1: `EV_DoDoor`.
    DoDoor = 1,
    /// 2: `EV_VerticalDoor`.
    VerticalDoor = 2,
    /// 3: `EV_DoPlat`.
    DoPlat = 3,
    /// 4: `EV_DoFloor`.
    DoFloor = 4,
    /// 5: `EV_BuildStairs` (Episode 1 policy).
    BuildStairs = 5,
    /// 6: `EV_DoDonut` (Episode 1 policy).
    DoDonut = 6,
}
/// The origin of a sector sound request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoundOrigin {
    /// 1: `&sector->soundorg`.
    Sector = 1,
    /// 2: the address of `buttonlist[index].soundorg`.
    Button = 2,
}

/// Where a test may inject a refusal inside a tic (never outside `cfg(test)`): the
/// rollback tests of the movers policy refuse a tic after a plane step has written
/// its height and run `P_ChangeSector`, or after a use has started its line's action.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FaultSite {
    /// In `T_MovePlane`, after the step's write and its `P_ChangeSector`.
    PlaneStep,
    /// In `P_UseSpecialLine`, after the line's action, before the use record.
    SpecialUse,
}
#[cfg(test)]
thread_local! {
    static INJECTED_FAULT: std::cell::Cell<Option<(FaultSite, usize)>> = const { std::cell::Cell::new(None) };
}
/// Arms a refusal (`StoreError::Capacity`) at the `nth` (1-based) next visit of
/// `site` on this thread; `None` disarms.
#[cfg(test)]
pub(crate) fn inject_fault(fault: Option<(FaultSite, usize)>) {
    INJECTED_FAULT.with(|f| f.set(fault));
}
/// Whether an injected refusal is still armed on this thread.
#[cfg(test)]
pub(crate) fn fault_armed() -> bool {
    INJECTED_FAULT.with(|f| f.get().is_some())
}
#[cfg(test)]
pub(crate) fn injected_fault(site: FaultSite) -> Result<(), SimError> {
    INJECTED_FAULT.with(|f| match f.get() {
        Some((armed, n)) if armed == site => {
            if n <= 1 {
                f.set(None);
                Err(StoreError::Capacity.into())
            } else {
                f.set(Some((armed, n - 1)));
                Ok(())
            }
        }
        _ => Ok(()),
    })
}

#[cfg(test)]
#[path = "planes_tests.rs"]
pub(crate) mod tests;
