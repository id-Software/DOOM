//! Doors, lifts and floors of the movers policy (M3h8 note §2.4–§2.6): the
//! `SectorThinker::{Door, Plat, Floor}` records, their `EV_*` starters and their
//! `T_*` thinkers.
//!
//! A mover joins the shared thinker order at its tail (`P_AddThinker`) in the lowest
//! free sector-thinker slot. When it finishes it is marked (`P_RemoveThinker`): its
//! slot and its place in the order stay until the next pass reaches it, while the
//! sector's `specialdata` (derived here: the one live mover naming the sector) is
//! cleared at once, so a later thinker of the same pass may start a replacement. The
//! unlink frees the slot and grows its generation. Mover fields the source leaves
//! uninitialised and never reads for the admitted types start at 0 (decision N6).
//! Sounds are recorded as sector-origin sound requests; none draws `P_Random`.
use crate::{
    Actor, Event, PickupMessage, SectorState, SectorThinker, SectorThinkerId, SimError,
    StaticWorld, StoreError, ThinkerContext, ThinkerStore,
    mover_rules::{self, LINE_DOOR_DR},
    original_player::PlayerRuntime,
    planes::{MoveResult, Plane, PlaneHost},
    sector_thinkers::SectorSpecial,
    staging::Sectors,
};
use doom_types::Fixed;

/// p_spec.h `VDOORSPEED` (2 units) and `VDOORWAIT` (150 tics).
const VDOORSPEED: i32 = 2;
const VDOORWAIT: i32 = 150;
/// p_doors.c `T_VerticalDoor`: a `close30ThenOpen` door waits `35*30` tics at the
/// bottom.
const CLOSE30_WAIT: i32 = 35 * 30;
/// p_spec.h `PLATWAIT` (3 seconds) and `PLATSPEED` (1 unit): the `downWaitUpStay`
/// lift waits `35 * PLATWAIT` tics and moves at `PLATSPEED * 4`.
const PLAT_WAIT: i32 = 35 * 3;
const PLAT_SPEED: i32 = 4;
/// p_spec.c `P_FindHighestFloorSurrounding`'s start, `-500*FRACUNIT` (map units).
pub(crate) const HIGHEST_FLOOR_START: i16 = -500;
/// p_spec.h `FLOORSPEED` (1 unit, `FRACUNIT`); `turboLower` moves at four times it.
const FLOORSPEED: i32 = 1;
/// p_spec.c `P_FindNextHighestFloor`'s `MAX_ADJOINING_SECTORS`.
const MAX_ADJOINING_SECTORS: usize = 20;
/// p_floor.c `EV_BuildStairs(build8)`: speed `FLOORSPEED/4`, step 8 units.
const STAIR_SPEED: i32 = 65536 / 4;
const STAIR_SIZE: i32 = 8 * 65536;
/// p_spec.c `EV_DoDonut`: both floors move at `FLOORSPEED/2`.
const DONUT_SPEED: i32 = 65536 / 2;
/// p_plats.c `raiseToNearestAndChange`: speed `PLATSPEED/2`.
const RTNAC_SPEED: i32 = 65536 / 2;

/// `vldoor_e` of the admitted doors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DoorKind {
    /// `normal` (0): DR (1, 26–28), SR (63) and WR (90) doors; speed 2, waits 150
    /// tics at the top.
    Normal = 0,
    /// `close30ThenOpen` (1): W1 16 and WR 76 (Episode 1 policy); speed 2, closes,
    /// waits `35*30` tics at the bottom, reopens to its old ceiling and is removed.
    Close30ThenOpen = 1,
    /// `open` (3): W1 2, WR 86, S1 103, GR 46 and D1 31–34 (Episode 1 policy);
    /// speed 2, removed at the top.
    Open = 3,
    /// `blazeOpen` (6): the W1 109 door; speed 8, removed at the top.
    BlazeOpen = 6,
}
/// `vldoor_t` (p_spec.h:343); `speed` and `topwait` are implied by the kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Door {
    /// Sector index.
    pub sector: u16,
    /// `type`.
    pub kind: DoorKind,
    /// −1 down, 0 waiting, 1 up.
    pub direction: i32,
    /// `topheight` (16.16).
    pub topheight: Fixed,
    /// `topcountdown`: 0 until the door first waits (decision N6).
    pub topcountdown: i32,
}
impl Door {
    fn speed(self) -> Fixed {
        Fixed::from_map_units(match self.kind {
            DoorKind::Normal | DoorKind::Close30ThenOpen | DoorKind::Open => VDOORSPEED as i16,
            DoorKind::BlazeOpen => (VDOORSPEED * 4) as i16,
        })
    }
}
/// `plat_e` of the admitted lift.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatStatus {
    /// `up` (0).
    Up = 0,
    /// `down` (1).
    Down = 1,
    /// `waiting` (2).
    Waiting = 2,
}
/// `plattype_e` (p_spec.h:274–281) of the admitted plats.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatType {
    /// `downWaitUpStay` (1): WR 88 and SR 62; speed 4, waits 105 tics.
    DownWaitUpStay = 1,
    /// `raiseToNearestAndChange` (3): S1 20 and W1 22 (Episode 1 policy); speed
    /// 1/2, no wait, `low` never written (0, decision N6).
    RaiseToNearestAndChange = 3,
}
/// `plat_t` (p_spec.h:285); `speed`, `wait` (implied by the type), `crush`
/// (false), `tag` and `oldstatus` (never written) are not kept.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Plat {
    /// Sector index.
    pub sector: u16,
    /// `type`.
    pub kind: PlatType,
    /// `status`.
    pub status: PlatStatus,
    /// `count`: tics left to wait; 0 until the lift first waits.
    pub count: i32,
    /// `low` (16.16).
    pub low: Fixed,
    /// `high` (16.16).
    pub high: Fixed,
}
/// `floor_e` of the admitted floors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FloorKind {
    /// `lowerFloor` (0): the donut hole (`EV_DoDonut`) and every stair step, whose
    /// `type` `EV_BuildStairs` never writes (zero, decision N6).
    LowerFloor = 0,
    /// `lowerFloorToLowest` (1): W1 38, S1 23, WR 82; speed 1.
    LowerFloorToLowest = 1,
    /// `turboLower` (2): W1 36, SR 70, WR 98; speed 4.
    TurboLower = 2,
    /// `raiseFloor` (3): W1 5, WR 91; speed 1.
    RaiseFloor = 3,
    /// `raiseFloorToNearest` (4): S1 18; speed 1.
    RaiseFloorToNearest = 4,
    /// `donutRaise` (11): the donut ring; speed 1/2.
    DonutRaise = 11,
}
/// `floormove_t` (p_spec.h:584); `crush` is false (or, for a stair step, never
/// written: the `StairBlocked` witness keeps it unobservable).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FloorMove {
    /// Sector index.
    pub sector: u16,
    /// `type`.
    pub kind: FloorKind,
    /// `floordestheight` (16.16).
    pub floordestheight: Fixed,
    /// `direction`: −1 down, 1 up.
    pub direction: i32,
    /// `speed` (16.16).
    pub speed: Fixed,
    /// `texture`: the normalized flat a `donutRaise` floor gives its sector at the
    /// end (zero for every other type, never read).
    pub texture: [u8; 8],
    /// `newspecial`: the special a `donutRaise` floor gives its sector (0).
    pub newspecial: i16,
    /// Rust-only: an `EV_BuildStairs` step, whose blocked plane step is the
    /// `StairBlocked` witness (decision H7). Never encoded.
    pub stair: bool,
}
impl Plat {
    /// `plattype_e` as DSIM39 encodes it.
    pub(crate) const fn plat_type(self) -> u8 {
        self.kind as u8
    }
    fn speed(self) -> Fixed {
        match self.kind {
            PlatType::DownWaitUpStay => Fixed::from_map_units(PLAT_SPEED as i16),
            PlatType::RaiseToNearestAndChange => Fixed::from_bits(RTNAC_SPEED),
        }
    }
    const fn wait(self) -> i32 {
        match self.kind {
            PlatType::DownWaitUpStay => PLAT_WAIT,
            PlatType::RaiseToNearestAndChange => 0,
        }
    }
}
impl FloorMove {
    /// A `EV_DoFloor` floor of `kind` (`newspecial`, `texture` zero).
    fn new(sector: u16, kind: FloorKind, floordestheight: Fixed) -> Self {
        let (direction, speed) = match kind {
            FloorKind::LowerFloor | FloorKind::LowerFloorToLowest => (-1, FLOORSPEED),
            FloorKind::TurboLower => (-1, FLOORSPEED * 4),
            FloorKind::RaiseFloor | FloorKind::RaiseFloorToNearest => (1, FLOORSPEED),
            FloorKind::DonutRaise => (1, 0),
        };
        Self {
            sector,
            kind,
            floordestheight,
            direction,
            speed: Fixed::from_map_units(speed as i16),
            texture: [0; 8],
            newspecial: 0,
            stair: false,
        }
    }
}

/// `S_StartSound` names of the admitted sector sounds.
fn sound(name: &str) -> u16 {
    crate::lifecycle::source_sound_id(name)
}

/// The sector-thinker store as the `EV_*` starters see it: the committed store
/// between tics and in the players phase, the traversal context in the pass.
pub(crate) trait MoverAccess {
    fn mover_store(&self) -> &ThinkerStore<Actor>;
    fn spawn_mover(&mut self, mover: SectorThinker) -> Result<SectorThinkerId, StoreError>;
    fn mover_mut(&mut self, id: SectorThinkerId) -> Option<&mut SectorThinker>;
}
impl MoverAccess for ThinkerStore<Actor> {
    fn mover_store(&self) -> &ThinkerStore<Actor> {
        self
    }
    fn spawn_mover(&mut self, mover: SectorThinker) -> Result<SectorThinkerId, StoreError> {
        self.spawn_sector(mover)
    }
    fn mover_mut(&mut self, id: SectorThinkerId) -> Option<&mut SectorThinker> {
        self.sector_get_mut(id)
    }
}
impl MoverAccess for ThinkerContext<'_, Actor> {
    fn mover_store(&self) -> &ThinkerStore<Actor> {
        self.store()
    }
    fn spawn_mover(&mut self, mover: SectorThinker) -> Result<SectorThinkerId, StoreError> {
        self.spawn_sector(mover)
    }
    fn mover_mut(&mut self, id: SectorThinkerId) -> Option<&mut SectorThinker> {
        self.sector_get_mut(id)
    }
}

/// `sector->specialdata`: the live (not removed) door, plat or floor naming
/// `sector`; lights never set it. At most one exists (the starters skip a sector
/// that has one).
pub(crate) fn special_data(store: &ThinkerStore<Actor>, sector: u16) -> Option<SectorThinkerId> {
    store.sector_slots.iter().enumerate().find_map(|(slot, s)| {
        let entry = s.entry.as_ref()?;
        (!entry.pending && entry.value.mover_sector() == Some(sector))
            .then(|| SectorThinkerId::from_parts(slot as u32, s.generation))
    })
}
/// The live `downWaitUpStay` plats, the occupied `activeplats` slots.
pub(crate) fn active_plats(store: &ThinkerStore<Actor>) -> usize {
    store
        .sector_slots
        .iter()
        .filter_map(|s| s.entry.as_ref())
        .filter(|e| !e.pending && matches!(e.value, SectorThinker::Plat(_)))
        .count()
}

/// The `p_spec.c` neighbour searches over `getNextSector`, reading `sectors`.
struct Surrounding<'a> {
    world: &'a StaticWorld,
    sectors: Sectors<'a>,
}
impl Surrounding<'_> {
    fn neighbors(&self, sector: u16) -> Result<impl Iterator<Item = SectorState> + '_, SimError> {
        let lines = self
            .world
            .sector_lines
            .as_ref()
            .and_then(|l| l.get(usize::from(sector)))
            .ok_or(crate::MapSpawnError::Context)?;
        let map = self.world.map();
        Ok(lines.iter().filter_map(move |&line| {
            mover_rules::next_sector(map, usize::from(line), usize::from(sector))
                .and_then(|n| self.sectors.get(n).copied())
        }))
    }
    fn own(&self, sector: u16) -> Result<SectorState, SimError> {
        self.sectors
            .get(usize::from(sector))
            .copied()
            .ok_or_else(|| crate::GeometryError::InvalidReference.into())
    }
    /// `P_FindLowestCeilingSurrounding` (starts at `MAXINT`; rule 3 guarantees a
    /// neighbour).
    fn lowest_ceiling(&self, sector: u16) -> Result<Fixed, SimError> {
        self.neighbors(sector)?
            .map(|s| s.ceiling)
            .min()
            .ok_or_else(|| crate::MapScaleError::MoverNeighbor.into())
    }
    /// `P_FindLowestCeilingSurrounding` as written (p_spec.c:383–403): folded from
    /// `MAXINT`, which a sector without a two-sided neighbour returns.
    fn lowest_ceiling_from_maxint(&self, sector: u16) -> Result<Fixed, SimError> {
        Ok(self
            .neighbors(sector)?
            .map(|s| s.ceiling)
            .fold(Fixed::from_bits(i32::MAX), Fixed::min))
    }
    /// `P_FindLowestFloorSurrounding` (starts at the sector's own floor).
    fn lowest_floor(&self, sector: u16) -> Result<Fixed, SimError> {
        let own = self.own(sector)?.floor;
        Ok(self
            .neighbors(sector)?
            .map(|s| s.floor)
            .fold(own, Fixed::min))
    }
    /// `P_FindHighestFloorSurrounding` (p_spec.c:297–316): the largest neighbour
    /// floor, folded from −500, so a sector whose neighbours all lie below −500 gets
    /// −500 (rule 3 guarantees a neighbour, not that one lies above −500).
    fn highest_floor(&self, sector: u16) -> Result<Fixed, SimError> {
        Ok(self
            .neighbors(sector)?
            .map(|s| s.floor)
            .fold(Fixed::from_map_units(HIGHEST_FLOOR_START), Fixed::max))
    }
    /// `P_FindNextHighestFloor` (p_spec.c:329–375): the smallest neighbour floor
    /// strictly above `current`, collected in line order into a 20-entry list; the
    /// walk stops right after the 20th entry (the overflow test follows every
    /// neighbour, so it breaks exactly when the 20th is written). `current` when
    /// the list is empty.
    fn next_highest_floor(&self, sector: u16, current: Fixed) -> Result<Fixed, SimError> {
        let mut collected = 0;
        let mut lowest: Option<Fixed> = None;
        for neighbor in self.neighbors(sector)? {
            if neighbor.floor > current {
                collected += 1;
                lowest = Some(lowest.map_or(neighbor.floor, |l| l.min(neighbor.floor)));
            }
            if collected >= MAX_ADJOINING_SECTORS {
                break;
            }
        }
        Ok(lowest.unwrap_or(current))
    }
}

/// The sector tag `A_BossDeath` hands `EV_DoFloor` on E1M8 (p_enemy.c:1731).
pub(crate) const BOSS_TAG: i16 = 666;

/// What an `EV_*` starter needs: the map, the current sector heights, the store and
/// the event stream; under the Episode 1 policy also the staged runtime sector
/// lights, specials and floor flats (written by 20, 22 and 35 and read by the
/// stairs, the donut and 20/22) and the using player (its keys and message).
pub(crate) struct Starter<'a, M: ?Sized> {
    pub world: &'a StaticWorld,
    pub sectors: Sectors<'a>,
    pub store: &'a mut M,
    pub events: &'a mut Vec<Event>,
    /// The staged runtime sector specials (`None` where no Episode 1 starter can
    /// run: monster crossings and uses, shots).
    pub specials: Option<&'a mut [SectorSpecial]>,
    /// The player whose use runs the starter (`None` for a monster or a crossing).
    pub player: Option<&'a mut PlayerRuntime>,
}
impl<M: MoverAccess + ?Sized> Starter<'_, M> {
    fn surrounding(&self) -> Surrounding<'_> {
        Surrounding {
            world: self.world,
            sectors: self.sectors,
        }
    }
    fn sector_sound(&mut self, sector: u16, name: &str) {
        self.events
            .push(Event::Mover(crate::MoverEvent::SectorSound {
                origin: crate::planes::SoundOrigin::Sector,
                index: sector,
                sound: sound(name),
            }));
    }
    fn tagged(&self, line: u16) -> Result<impl Iterator<Item = u16> + '_, SimError> {
        let world = self.world;
        let map = world.map();
        let tag = map
            .linedefs()
            .get(usize::from(line))
            .ok_or(crate::GeometryError::InvalidReference)?
            .tag;
        Ok(mover_rules::tagged(map, tag).map(|i| i as u16))
    }
    /// Refuses an Episode 1 starter outside the Episode 1 policy (defence in
    /// depth: rule 1 admits its specials only there).
    fn episode1(&self) -> Result<(), SimError> {
        if self.world.episode1.is_none() {
            return Err(crate::PursuitError::Specials.into());
        }
        Ok(())
    }
    fn specials(&mut self) -> Result<&mut [SectorSpecial], SimError> {
        self.specials
            .as_deref_mut()
            .ok_or_else(|| crate::MapSpawnError::Context.into())
    }
    /// `sector->floorpic` at this point of the tic.
    fn floorpic(&mut self, sector: usize) -> Result<[u8; 8], SimError> {
        self.specials()?
            .get(sector)
            .map(|s| s.floorpic)
            .ok_or_else(|| crate::GeometryError::InvalidReference.into())
    }
    /// Whether `sector` holds a live mover (`sector->specialdata`).
    fn busy(&self, sector: u16) -> bool {
        special_data(self.store.mover_store(), sector).is_some()
    }
    /// Appends `mover` to the shared order (`P_AddThinker`).
    fn spawn(&mut self, mover: SectorThinker) -> Result<SectorThinkerId, SimError> {
        Ok(self.store.spawn_mover(mover)?)
    }
    fn finish(
        &mut self,
        function: crate::planes::MoverFunction,
        line: u16,
        argument: i32,
        result: i32,
        created: &[(SectorThinkerId, SectorThinker)],
    ) {
        for (id, mover) in created {
            self.events.push(Event::Mover(crate::MoverEvent::Started {
                mover: *id,
                kind: mover.kind_code(),
                sector: mover.mover_sector().unwrap_or(u16::MAX),
            }));
        }
        self.events.push(Event::Mover(crate::MoverEvent::Call {
            function,
            line,
            argument,
            result,
            created: created.len() as u8,
        }));
    }

    /// p_doors.c `EV_DoDoor` (264–347) for the admitted `normal` (63, 90), `open`
    /// (2, 86, 103, 46), `close30ThenOpen` (16, 76) and `blazeOpen` (109) doors:
    /// every tagged sector without a live mover gets a door. `close30ThenOpen` tops
    /// at the current ceiling and closes with `sfx_dorcls` always; the others rise
    /// at speed 2 (8 for `blazeOpen`) to `P_FindLowestCeilingSurrounding − 4`,
    /// with `sfx_doropn` (`sfx_bdopn`) only when that top differs from the
    /// ceiling. Returns whether a door was created.
    pub(crate) fn do_door(&mut self, line: u16, kind: DoorKind) -> Result<bool, SimError> {
        if matches!(kind, DoorKind::Close30ThenOpen | DoorKind::Open) {
            self.episode1()?;
        }
        let targets: Vec<u16> = self.tagged(line)?.collect();
        let mut created = Vec::new();
        for sector in targets {
            if self.busy(sector) {
                continue;
            }
            let ceiling = self.surrounding().own(sector)?.ceiling;
            let (direction, topheight) = if kind == DoorKind::Close30ThenOpen {
                (-1, ceiling)
            } else {
                let lowest = self.surrounding().lowest_ceiling(sector)?;
                (
                    1,
                    crate::planes::height_sum(self.world, lowest, Fixed::from_map_units(-4))?,
                )
            };
            let door = SectorThinker::Door(Door {
                sector,
                kind,
                direction,
                topheight,
                topcountdown: 0,
            });
            let id = self.spawn(door)?;
            match kind {
                DoorKind::Close30ThenOpen => self.sector_sound(sector, "sfx_dorcls"),
                _ if topheight == ceiling => {}
                DoorKind::BlazeOpen => self.sector_sound(sector, "sfx_bdopn"),
                _ => self.sector_sound(sector, "sfx_doropn"),
            }
            created.push((id, door));
        }
        let result = i32::from(!created.is_empty());
        self.finish(
            crate::planes::MoverFunction::DoDoor,
            line,
            kind as i32,
            result,
            &created,
        );
        Ok(result != 0)
    }

    /// p_doors.c `EV_VerticalDoor` (354–499) for the DR lines 1, 26–28 and (Episode
    /// 1 policy) the D1 lines 31–34. A locked line (26/32 blue, 27/34 yellow, 28/33
    /// red) does nothing for a monster and, for a player without either key of its
    /// colour, sets the `PD_*` message and plays `sfx_oof` without an origin. On the
    /// back sector's live mover a DR line reverses the door (a closing door goes back
    /// up for anyone; otherwise a player sends it down and a monster does nothing);
    /// a non-door there refuses the tic ([`crate::MapScaleError::MoverFamily`]) and
    /// a D1 line, where the source spawns a second thinker over the busy sector,
    /// refuses it with [`crate::MapScaleError::SecondDoor`]. Otherwise `sfx_doropn`
    /// plays unconditionally and a `normal` (DR) or `open` (D1) door rises to
    /// `P_FindLowestCeilingSurrounding − 4`. `player` is whether the user is a
    /// player (`thing->player`). Returns whether the line special was cleared (a D1
    /// line that created its door).
    pub(crate) fn vertical_door(&mut self, line: u16, player: bool) -> Result<bool, SimError> {
        let world = self.world;
        let map = world.map();
        let l = map
            .linedefs()
            .get(usize::from(line))
            .ok_or(crate::GeometryError::InvalidReference)?;
        let special = l.special;
        match special {
            LINE_DOOR_DR => {}
            26..=28 | 31..=34 => self.episode1()?,
            _ => return Err(crate::PursuitError::Specials.into()),
        }
        // Rule 1: every DR and D1 line has a back side.
        let back = l.sides[1].ok_or(crate::MapScaleError::DoorLine)?;
        let sector = map.sidedefs()[usize::from(back)].sector;
        let done = |s: &mut Self, created: &[(SectorThinkerId, SectorThinker)]| {
            s.finish(
                crate::planes::MoverFunction::VerticalDoor,
                line,
                i32::from(special),
                -1,
                created,
            );
        };
        // The lock switch (p_doors.c:369–410).
        let lock = match special {
            26 | 32 => Some(([0, 3], PickupMessage::BlueKeyNeeded)),
            27 | 34 => Some(([1, 4], PickupMessage::YellowKeyNeeded)),
            28 | 33 => Some(([2, 5], PickupMessage::RedKeyNeeded)),
            _ => None,
        };
        if let Some((keys, message)) = lock {
            if !player {
                done(self, &[]);
                return Ok(false);
            }
            let p = self
                .player
                .as_deref_mut()
                .ok_or(crate::PlayerBodyError::Profile)?;
            let cards = p.powers.ok_or(crate::PlayerBodyError::Profile)?.cards;
            if !cards[keys[0]] && !cards[keys[1]] {
                p.pickups
                    .as_mut()
                    .ok_or(crate::PlayerBodyError::Profile)?
                    .message = message;
                self.events.push(Event::ActorState(
                    crate::lifecycle::ActorStateEvent::Sound {
                        actor: None,
                        sound: sound("sfx_oof"),
                    },
                ));
                done(self, &[]);
                return Ok(false);
            }
        }
        if let Some(id) = special_data(self.store.mover_store(), sector) {
            if special >= 31 {
                // D1 on a busy sector: the source falls through and spawns a
                // second thinker over it (p_doors.c:416–437, 459–461).
                return Err(crate::MapScaleError::SecondDoor.into());
            }
            // Rule 2 (or its per-tic witness): the cast of specialdata to a door
            // reads a door.
            let Some(SectorThinker::Door(door)) = self.store.mover_mut(id) else {
                return Err(crate::MapScaleError::MoverFamily.into());
            };
            if door.direction == -1 {
                door.direction = 1;
            } else if player {
                door.direction = -1;
            }
            done(self, &[]);
            return Ok(false);
        }
        self.sector_sound(sector, "sfx_doropn");
        let lowest = self.surrounding().lowest_ceiling(sector)?;
        let topheight = crate::planes::height_sum(world, lowest, Fixed::from_map_units(-4))?;
        let open = special >= 31;
        let door = SectorThinker::Door(Door {
            sector,
            kind: if open {
                DoorKind::Open
            } else {
                DoorKind::Normal
            },
            direction: 1,
            topheight,
            topcountdown: 0,
        });
        let created = [(self.spawn(door)?, door)];
        done(self, &created);
        Ok(open)
    }

    /// p_plats.c `EV_DoPlat` (150–260) for `downWaitUpStay` (WR 88, SR 62: down at
    /// speed 4 to `P_FindLowestFloorSurrounding`, at most its floor, `high` = its
    /// floor, `sfx_pstart`) and `raiseToNearestAndChange` (S1 20, W1 22: up at
    /// speed 1/2 to `P_FindNextHighestFloor`, the sector takes the floor flat of
    /// the line's front sector and special 0, `sfx_stnmov`), for every tagged
    /// sector without a live mover; each takes an `activeplats` slot (rule 5 keeps
    /// one free).
    pub(crate) fn do_plat(&mut self, line: u16, kind: PlatType) -> Result<bool, SimError> {
        let front = if kind == PlatType::RaiseToNearestAndChange {
            self.episode1()?;
            let world = self.world;
            let map = world.map();
            let side = map
                .linedefs()
                .get(usize::from(line))
                .and_then(|l| l.sides[0])
                .ok_or(crate::GeometryError::InvalidReference)?;
            Some(usize::from(map.sidedefs()[usize::from(side)].sector))
        } else {
            None
        };
        let targets: Vec<u16> = self.tagged(line)?.collect();
        let mut created = Vec::new();
        for sector in targets {
            if self.busy(sector) {
                continue;
            }
            let floor = self.surrounding().own(sector)?.floor;
            let plat = match front {
                None => {
                    let low = self.surrounding().lowest_floor(sector)?.min(floor);
                    Plat {
                        sector,
                        kind,
                        status: PlatStatus::Down,
                        count: 0,
                        low,
                        high: floor,
                    }
                }
                Some(front) => {
                    // sec->floorpic = sides[line->sidenum[0]].sector->floorpic.
                    let pic = self.floorpic(front)?;
                    let high = self.surrounding().next_highest_floor(sector, floor)?;
                    let s = self
                        .specials()?
                        .get_mut(usize::from(sector))
                        .ok_or(crate::GeometryError::InvalidReference)?;
                    s.floorpic = pic;
                    s.special = 0;
                    Plat {
                        sector,
                        kind,
                        status: PlatStatus::Up,
                        count: 0,
                        low: Fixed::ZERO,
                        high,
                    }
                }
            };
            let plat = SectorThinker::Plat(plat);
            let id = self.spawn(plat)?;
            self.sector_sound(
                sector,
                match kind {
                    PlatType::DownWaitUpStay => "sfx_pstart",
                    PlatType::RaiseToNearestAndChange => "sfx_stnmov",
                },
            );
            // P_AddActivePlat: I_Error without a free slot, unreachable under rule 5.
            if active_plats(self.store.mover_store()) > mover_rules::MAX_PLATS {
                return Err(crate::MapScaleError::MoverCount.into());
            }
            created.push((id, plat));
        }
        let result = i32::from(!created.is_empty());
        self.finish(
            crate::planes::MoverFunction::DoPlat,
            line,
            kind as i32,
            result,
            &created,
        );
        Ok(result != 0)
    }

    /// p_floor.c `EV_DoFloor` (264–444) for `lowerFloorToLowest` (38, 23, 82: speed
    /// 1 down to `P_FindLowestFloorSurrounding`), `turboLower` (36, 70, 98: speed 4
    /// down to `P_FindHighestFloorSurrounding`, plus 8 unless it equals the floor),
    /// `raiseFloor` (5, 91: speed 1 up to `P_FindLowestCeilingSurrounding`, from
    /// `MAXINT`, at most the own ceiling) and `raiseFloorToNearest` (18: speed 1 up
    /// to `P_FindNextHighestFloor`). No sound at the start.
    pub(crate) fn do_floor(&mut self, line: u16, kind: FloorKind) -> Result<bool, SimError> {
        match kind {
            FloorKind::LowerFloorToLowest | FloorKind::TurboLower => {}
            FloorKind::RaiseFloor | FloorKind::RaiseFloorToNearest => self.episode1()?,
            FloorKind::LowerFloor | FloorKind::DonutRaise => {
                return Err(crate::PursuitError::Specials.into());
            }
        }
        let targets: Vec<u16> = self.tagged(line)?.collect();
        self.do_floor_in(targets, line, kind)
    }
    /// `A_BossDeath`'s `junk.tag = 666; EV_DoFloor (&junk, lowerFloorToLowest)`
    /// (p_enemy.c:1729–1733) under wire 24 (M4b step 5): `junk` is an
    /// uninitialised local `line_t` whose only field set is the tag, and
    /// `EV_DoFloor` reads nothing else of it for this floor type
    /// (`P_FindSectorFromLineTag`, p_spec.c:434–445; p_floor.c:299–305). So the
    /// sectors are those tagged 666, in index order, and no line field is read;
    /// the call record names line 65535 (no line). Sectors already holding a
    /// mover are skipped (p_floor.c:277–278); each new floor is appended to the
    /// shared order and so first moves in this tic. Refused outside wire 24.
    pub(crate) fn boss_floor(&mut self) -> Result<bool, SimError> {
        if self.world.map_number.is_none() {
            return Err(crate::PursuitError::Specials.into());
        }
        let targets: Vec<u16> = mover_rules::tagged(self.world.map(), BOSS_TAG)
            .map(|i| i as u16)
            .collect();
        self.do_floor_in(targets, u16::MAX, FloorKind::LowerFloorToLowest)
    }
    fn do_floor_in(
        &mut self,
        targets: Vec<u16>,
        line: u16,
        kind: FloorKind,
    ) -> Result<bool, SimError> {
        let mut created = Vec::new();
        for sector in targets {
            if self.busy(sector) {
                continue;
            }
            let own = self.surrounding().own(sector)?;
            let floordestheight = match kind {
                FloorKind::LowerFloorToLowest => self.surrounding().lowest_floor(sector)?,
                FloorKind::TurboLower => {
                    let dest = self.surrounding().highest_floor(sector)?;
                    if dest != own.floor {
                        crate::planes::height_sum(self.world, dest, Fixed::from_map_units(8))?
                    } else {
                        dest
                    }
                }
                FloorKind::RaiseFloor => self
                    .surrounding()
                    .lowest_ceiling_from_maxint(sector)?
                    .min(own.ceiling),
                _ => self.surrounding().next_highest_floor(sector, own.floor)?,
            };
            let floor = SectorThinker::Floor(FloorMove::new(sector, kind, floordestheight));
            created.push((self.spawn(floor)?, floor));
        }
        let result = i32::from(!created.is_empty());
        self.finish(
            crate::planes::MoverFunction::DoFloor,
            line,
            kind as i32,
            result,
            &created,
        );
        Ok(result != 0)
    }

    /// p_floor.c `EV_BuildStairs(build8)` (453–554), S1 7 and W1 8: for every
    /// tagged sector without a live mover (the tag search resumes after the index
    /// of the last step built, since the walk overwrites `secnum`), a floor rising
    /// at 1/4 unit to its floor + 8; then, from the newest step, the first
    /// `ML_TWOSIDED` line of its line list whose front sector is that step and
    /// whose back sector has the first step's floor flat adds 8 to the height
    /// (even when that sector is busy, which is skipped) and, if free, becomes the
    /// next step rising to the accumulated height; the scan restarts from it.
    /// Returns whether a tagged sector got a floor.
    pub(crate) fn build_stairs(&mut self, line: u16) -> Result<bool, SimError> {
        self.episode1()?;
        let world = self.world;
        let map = world.map();
        let tag = map
            .linedefs()
            .get(usize::from(line))
            .ok_or(crate::GeometryError::InvalidReference)?
            .tag;
        let lines = world
            .sector_lines
            .as_ref()
            .ok_or(crate::MapSpawnError::Context)?;
        let sector_of =
            |side: Option<u16>| side.map(|s| usize::from(map.sidedefs()[usize::from(s)].sector));
        let count = map.sectors().len();
        let mut created = Vec::new();
        let mut secnum = 0;
        let mut rtn = false;
        // P_FindSectorFromLineTag(line, secnum) from secnum + 1.
        while let Some(found) = (secnum..count).find(|&i| map.sectors()[i].tag == tag) {
            secnum = found + 1;
            let mut sec = found;
            if self.busy(sec as u16) {
                continue;
            }
            rtn = true;
            let speed = Fixed::from_bits(STAIR_SPEED);
            let step = |sector: usize, height: i32| {
                SectorThinker::Floor(FloorMove {
                    sector: sector as u16,
                    kind: FloorKind::LowerFloor,
                    floordestheight: Fixed::from_bits(height),
                    direction: 1,
                    speed,
                    texture: [0; 8],
                    newspecial: 0,
                    stair: true,
                })
            };
            let floor = self.surrounding().own(sec as u16)?.floor;
            let mut height = floor
                .bits()
                .checked_add(STAIR_SIZE)
                .ok_or(crate::MapScaleError::Arithmetic)?;
            let first = step(sec, height);
            created.push((self.spawn(first)?, first));
            let texture = self.floorpic(sec)?;
            loop {
                let mut next = None;
                for &l in lines.get(sec).ok_or(crate::MapSpawnError::Context)? {
                    let linedef = &map.linedefs()[usize::from(l)];
                    if linedef.flags & crate::player_map_scale::ML_TWOSIDED == 0 {
                        continue;
                    }
                    if sector_of(linedef.sides[0]) != Some(sec) {
                        continue;
                    }
                    // The geometry rule guarantees a back side on a two-sided line.
                    let tsec = sector_of(linedef.sides[1]).ok_or(crate::MapScaleError::Geometry)?;
                    if self.floorpic(tsec)? != texture {
                        continue;
                    }
                    height = height
                        .checked_add(STAIR_SIZE)
                        .ok_or(crate::MapScaleError::Arithmetic)?;
                    if self.busy(tsec as u16) {
                        continue;
                    }
                    next = Some(tsec);
                    break;
                }
                let Some(tsec) = next else { break };
                sec = tsec;
                secnum = tsec + 1;
                let floor = step(sec, height);
                created.push((self.spawn(floor)?, floor));
            }
        }
        self.finish(
            crate::planes::MoverFunction::BuildStairs,
            line,
            0,
            i32::from(rtn),
            &created,
        );
        Ok(rtn)
    }

    /// p_spec.c `EV_DoDonut` (1163–1221), S1 9: for every tagged sector `s1`
    /// without a live mover (the return value becomes 1 before anything spawns),
    /// `s2` is `getNextSector(s1->lines[0], s1)` and `s3` the back sector of the
    /// first line of `s2` whose back sector is not `s1` (one-sided lines are not
    /// skipped: the source's test is `(!flags) & ML_TWOSIDED`, always 0; decision
    /// H8's load rule guarantees both exist); `s2` gets a `donutRaise` floor
    /// rising at 1/2 unit to `s3`'s floor that will give it `s3`'s floor flat and
    /// special 0, then `s1` a `lowerFloor` floor lowering at 1/2 unit to `s3`'s
    /// floor. A busy `s2`, where the source spawns a second thinker over it,
    /// refuses the tic ([`crate::MapScaleError::DonutBusy`], decision H8).
    pub(crate) fn do_donut(&mut self, line: u16) -> Result<bool, SimError> {
        self.episode1()?;
        let targets: Vec<u16> = self.tagged(line)?.collect();
        let world = self.world;
        let map = world.map();
        let lines = world
            .sector_lines
            .as_ref()
            .ok_or(crate::MapSpawnError::Context)?;
        let back = |line: u16| {
            map.linedefs()[usize::from(line)].sides[1]
                .map(|s| usize::from(map.sidedefs()[usize::from(s)].sector))
        };
        let mut created = Vec::new();
        let mut rtn = false;
        for s1 in targets {
            if self.busy(s1) {
                continue;
            }
            rtn = true;
            let s1 = usize::from(s1);
            let first = *lines
                .get(s1)
                .and_then(|l| l.first())
                .ok_or(crate::MapScaleError::DonutGeometry)?;
            let s2 = mover_rules::next_sector(map, usize::from(first), s1)
                .ok_or(crate::MapScaleError::DonutGeometry)?;
            let Some(&ring) = lines[s2].iter().find(|&&l| back(l) != Some(s1)) else {
                continue;
            };
            let s3 = back(ring).ok_or(crate::MapScaleError::DonutGeometry)?;
            if self.busy(s2 as u16) {
                return Err(crate::MapScaleError::DonutBusy.into());
            }
            let dest = self.surrounding().own(s3 as u16)?.floor;
            let texture = self.floorpic(s3)?;
            let speed = Fixed::from_bits(DONUT_SPEED);
            let raise = SectorThinker::Floor(FloorMove {
                sector: s2 as u16,
                kind: FloorKind::DonutRaise,
                floordestheight: dest,
                direction: 1,
                speed,
                texture,
                newspecial: 0,
                stair: false,
            });
            created.push((self.spawn(raise)?, raise));
            let hole = SectorThinker::Floor(FloorMove {
                sector: s1 as u16,
                kind: FloorKind::LowerFloor,
                floordestheight: dest,
                direction: -1,
                speed,
                texture: [0; 8],
                newspecial: 0,
                stair: false,
            });
            created.push((self.spawn(hole)?, hole));
        }
        self.finish(
            crate::planes::MoverFunction::DoDonut,
            line,
            0,
            i32::from(rtn),
            &created,
        );
        Ok(rtn)
    }

    /// p_lights.c `EV_LightTurnOn(line, bright)` (271–306) with a nonzero `bright`
    /// (W1 35: 35): every sector with the line's tag, in index order, takes light
    /// `bright` (no neighbour search). No thinker, no draw, no event.
    pub(crate) fn light_turn_on(&mut self, line: u16, bright: i16) -> Result<(), SimError> {
        self.episode1()?;
        let targets: Vec<u16> = self.tagged(line)?.collect();
        let specials = self.specials()?;
        for sector in targets {
            specials
                .get_mut(usize::from(sector))
                .ok_or(crate::GeometryError::InvalidReference)?
                .light = bright;
        }
        Ok(())
    }
}

/// One mover's turn in `P_RunThinkers`: `T_VerticalDoor`, `T_PlatRaise` or
/// `T_MoveFloor`. `leveltime` is the source `leveltime` of this tic.
pub(crate) fn think(
    host: &mut PlaneHost<'_, '_>,
    id: SectorThinkerId,
    leveltime: u64,
) -> Result<(), SimError> {
    let mover = *host
        .access
        .sector_get(id)
        .ok_or(StoreError::UnsupportedThinker)?;
    let (next, removed) = match mover {
        SectorThinker::Door(door) => vertical_door(host, door)?,
        SectorThinker::Plat(plat) => plat_raise(host, plat, leveltime)?,
        SectorThinker::Floor(floor) => move_floor(host, floor, leveltime)?,
        _ => return Err(StoreError::UnsupportedThinker.into()),
    };
    *host
        .access
        .sector_get_mut(id)
        .ok_or(StoreError::UnsupportedThinker)? = next;
    if removed {
        // P_RemoveThinker: marked, unlinked when the next pass reaches it.
        host.access.mark_remove_sector(id)?;
        host.events
            .push(Event::Mover(crate::MoverEvent::Removed { mover: id }));
    }
    if let Some(sound) = removed_sound(mover, removed) {
        sector_sound(host, mover.mover_sector().unwrap_or(u16::MAX), sound);
    }
    Ok(())
}
/// `T_MoveFloor` plays `sfx_pstop` after `P_RemoveThinker`.
fn removed_sound(mover: SectorThinker, removed: bool) -> Option<&'static str> {
    (removed && matches!(mover, SectorThinker::Floor(_))).then_some("sfx_pstop")
}
fn sector_sound(host: &mut PlaneHost<'_, '_>, sector: u16, name: &str) {
    host.events
        .push(Event::Mover(crate::MoverEvent::SectorSound {
            origin: crate::planes::SoundOrigin::Sector,
            index: sector,
            sound: sound(name),
        }));
}

/// p_doors.c `T_VerticalDoor` (63–200) for `normal`, `close30ThenOpen`, `open` and
/// `blazeOpen`, ceiling plane, `crush` false. Returns the updated record and
/// whether it removed itself.
fn vertical_door(
    host: &mut PlaneHost<'_, '_>,
    mut door: Door,
) -> Result<(SectorThinker, bool), SimError> {
    let mut removed = false;
    match door.direction {
        0 => {
            // Waiting: `if (!--door->topcountdown)`; `normal` turns down,
            // `close30ThenOpen` back up.
            door.topcountdown = door.topcountdown.wrapping_sub(1);
            if door.topcountdown == 0 {
                match door.kind {
                    DoorKind::Normal => {
                        door.direction = -1;
                        sector_sound(host, door.sector, "sfx_dorcls");
                    }
                    DoorKind::Close30ThenOpen => {
                        door.direction = 1;
                        sector_sound(host, door.sector, "sfx_doropn");
                    }
                    DoorKind::Open | DoorKind::BlazeOpen => {}
                }
            }
        }
        -1 => {
            let floor = host.sector(door.sector)?.floor;
            match host.move_plane(door.sector, door.speed(), floor, Plane::Ceiling, -1)? {
                // A normal door is removed at the floor and a close30ThenOpen door
                // waits there; a reversed open or blazeOpen door has no case
                // (default) and stays at the floor as a live thinker.
                MoveResult::PastDest => match door.kind {
                    DoorKind::Normal => removed = true,
                    DoorKind::Close30ThenOpen => {
                        door.direction = 0;
                        door.topcountdown = CLOSE30_WAIT;
                    }
                    DoorKind::Open | DoorKind::BlazeOpen => {}
                },
                // Every admitted door type reopens on an obstruction.
                MoveResult::Crushed => {
                    door.direction = 1;
                    sector_sound(host, door.sector, "sfx_doropn");
                }
                MoveResult::Ok => {}
            }
        }
        1 => {
            let result =
                host.move_plane(door.sector, door.speed(), door.topheight, Plane::Ceiling, 1)?;
            if result == MoveResult::PastDest {
                match door.kind {
                    DoorKind::Normal => {
                        door.direction = 0;
                        door.topcountdown = VDOORWAIT;
                    }
                    DoorKind::Close30ThenOpen | DoorKind::Open | DoorKind::BlazeOpen => {
                        removed = true;
                    }
                }
            }
        }
        // The switch has no default: nothing for any other direction (never stored).
        _ => {}
    }
    Ok((SectorThinker::Door(door), removed))
}

/// p_plats.c `T_PlatRaise` (52–129), floor plane, `crush` false; reaching the top
/// calls `P_RemoveActivePlat`. A `raiseToNearestAndChange` plat plays `sfx_stnmov`
/// every eighth tic while rising, and a blocked rise of one refuses the tic
/// ([`crate::MapScaleError::PlatBlocked`], decision H7: the source would turn it
/// down to its never-written `low`).
fn plat_raise(
    host: &mut PlaneHost<'_, '_>,
    mut plat: Plat,
    leveltime: u64,
) -> Result<(SectorThinker, bool), SimError> {
    let speed = plat.speed();
    let mut removed = false;
    match plat.status {
        PlatStatus::Up => {
            let result = host.move_plane(plat.sector, speed, plat.high, Plane::Floor, 1)?;
            let change = plat.kind == PlatType::RaiseToNearestAndChange;
            if change && leveltime & 7 == 0 {
                sector_sound(host, plat.sector, "sfx_stnmov");
            }
            match result {
                MoveResult::Crushed if change => {
                    return Err(crate::MapScaleError::PlatBlocked.into());
                }
                MoveResult::Crushed => {
                    plat.count = plat.wait();
                    plat.status = PlatStatus::Down;
                    sector_sound(host, plat.sector, "sfx_pstart");
                }
                MoveResult::PastDest => {
                    plat.count = plat.wait();
                    plat.status = PlatStatus::Waiting;
                    sector_sound(host, plat.sector, "sfx_pstop");
                    removed = true;
                }
                MoveResult::Ok => {}
            }
        }
        PlatStatus::Down => {
            if host.move_plane(plat.sector, speed, plat.low, Plane::Floor, -1)?
                == MoveResult::PastDest
            {
                plat.count = plat.wait();
                plat.status = PlatStatus::Waiting;
                sector_sound(host, plat.sector, "sfx_pstop");
            }
        }
        PlatStatus::Waiting => {
            plat.count = plat.count.wrapping_sub(1);
            if plat.count == 0 {
                plat.status = if host.sector(plat.sector)?.floor == plat.low {
                    PlatStatus::Up
                } else {
                    PlatStatus::Down
                };
                sector_sound(host, plat.sector, "sfx_pstart");
            }
        }
    }
    Ok((SectorThinker::Plat(plat), removed))
}

/// p_floor.c `T_MoveFloor` (209–253): one plane step in the floor's direction,
/// `sfx_stnmov` when `leveltime & 7 == 0`, and at `pastdest` a rising
/// `donutRaise` floor gives its sector `newspecial` and `texture`, then the
/// removal is followed by `sfx_pstop`. A stair step whose plane step does not fit
/// on its first `P_ChangeSector` refuses the tic
/// ([`crate::MapScaleError::StairBlocked`], decision H7: `EV_BuildStairs` never
/// writes `crush`, so the source's crush branch would read an indeterminate value).
fn move_floor(
    host: &mut PlaneHost<'_, '_>,
    floor: FloorMove,
    leveltime: u64,
) -> Result<(SectorThinker, bool), SimError> {
    let result = host.move_plane_witnessed(
        floor.sector,
        floor.speed,
        floor.floordestheight,
        Plane::Floor,
        floor.direction,
        floor
            .stair
            .then_some(crate::MapScaleError::StairBlocked.into()),
    )?;
    if leveltime & 7 == 0 {
        sector_sound(host, floor.sector, "sfx_stnmov");
    }
    let done = result == MoveResult::PastDest;
    if done && floor.direction == 1 && floor.kind == FloorKind::DonutRaise {
        let s = host
            .specials
            .as_deref_mut()
            .ok_or(crate::MapSpawnError::Context)?
            .get_mut(usize::from(floor.sector))
            .ok_or(crate::GeometryError::InvalidReference)?;
        s.special = floor.newspecial;
        s.floorpic = floor.texture;
    }
    Ok((SectorThinker::Floor(floor), done))
}

#[cfg(test)]
impl crate::Simulation {
    /// Runs `EV_*` starters on the committed state between tics, as a use in the
    /// players phase would before the pass: the movers join the order's tail and
    /// step in the next tic. Their events are kept for that tic.
    pub(crate) fn start_movers<R>(
        &mut self,
        f: impl FnOnce(&mut Starter<'_, ThinkerStore<Actor>>) -> Result<R, SimError>,
    ) -> Result<R, SimError> {
        let mut events = Vec::new();
        let result = f(&mut Starter {
            world: &self.world,
            sectors: Sectors::committed(&self.sectors),
            store: &mut self.actors,
            events: &mut events,
            specials: self.specials.as_mut().map(|s| s.sectors.as_mut_slice()),
            player: self.original_player.as_mut(),
        })?;
        self.pending.extend(events);
        Ok(result)
    }
}

#[cfg(test)]
#[path = "movers_tests.rs"]
mod tests;
