//! Canonical sound records of the Episode 1 part 3 policy (wire 25, M5c decision T3).
//!
//! Under `OriginalExecutionProfile::PlayerMonstersEpisode1V3` the simulation
//! records, in the C call order within a tic, every `S_StartSound` a level tic makes
//! and every `S_StopSound` made by `P_RemoveMobj` (p_mobj.c:565-566). The records
//! are the canonical DSIM41 group `events.sounds`; they leave `events.last` and
//! `events.pending` exactly as wire 24 emits them (apart from the corrected
//! hard-death scream). No sound logic, no `M_Random` and no mixer run here: the
//! records are what `S_StartSoundAtVolume` (s_sound.c:254-391) is given, with the
//! positions it reads at the moment of the call.
//!
//! Older policies never produce a record.

use crate::ActorId;

/// What `S_StartSound`'s first argument points at (M5c note §1.2.2, the five kinds
/// of origin): the identity `S_StopSound` compares (s_sound.c:349, 471-482).
///
/// The listener's own body is an [`SoundOrigin::Actor`] like any other actor; the
/// sound logic tells it apart by comparing the handle with the listener, as the C
/// compares `origin != players[consoleplayer].mo` (s_sound.c:302).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SoundOrigin {
    /// A NULL origin: full volume, centre separation (s_sound.c:304, 321-324).
    None,
    /// A `mobj_t`, by its generational handle (pointer identity: `P_RemoveMobj`
    /// stops its channel before the memory can be reused, p_mobj.c:565-569).
    Actor(ActorId),
    /// `&sectors[index].soundorg` (r_defs.h:121; computed once at load,
    /// p_setup.c:536-557).
    Sector(u16),
    /// `(mobj_t *)&buttonlist[slot].soundorg`, the switch-release origin of
    /// `P_UpdateSpecials` (p_spec.c:1151): the address of the pointer field, not
    /// its value (M5c note §1.2.6.3, §4.4.5).
    Button(u8),
}

/// The positions `S_AdjustSoundParams` reads (s_sound.c:761-793), taken at the
/// moment of the call: raw 16.16 `fixed_t` bits and an `angle_t`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SoundPlacement {
    /// `origin->x`, `origin->y` (for a button origin, the declared 64-bit reading
    /// of p_spec.c:1151: the next slot's `btimer` and 0).
    pub origin: [i32; 2],
    /// `players[consoleplayer].mo->x`, `->y`.
    pub listener: [i32; 2],
    /// `players[consoleplayer].mo->angle`.
    pub listener_angle: u32,
}

/// One sound call of a tic, in the C call order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SoundRecord {
    /// `S_StartSound(origin, sfx)` (s_sound.c:425-431).
    Start {
        /// The `sfxenum_t` value (sounds.h:179-288), 1..=108 at every call site.
        sfx: u16,
        /// The origin's identity.
        origin: SoundOrigin,
        /// Present exactly for a positional start: an origin that is not NULL and
        /// not the listener's own body (s_sound.c:302).
        at: Option<SoundPlacement>,
    },
    /// `S_StopSound(mobj)` from `P_RemoveMobj` (p_mobj.c:565-566).
    Stop {
        /// The removed actor.
        actor: ActorId,
    },
}

/// A refusal raised by wire 25's sound support (M5c decisions T11 and T12). Every
/// refusal leaves the complete canonical state as it was.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoundError {
    /// The sum `bbox[BOXRIGHT]+bbox[BOXLEFT]` or `bbox[BOXTOP]+bbox[BOXBOTTOM]`
    /// of this sector's sound origin overflows `int` (p_setup.c:556-557, signed
    /// overflow, undefined in C): the map is refused at load.
    SectorOriginOverflow {
        /// The sector index.
        sector: u16,
    },
    /// The release of button slot 15 (`P_UpdateSpecials`, p_spec.c:1151) would
    /// read its position past the end of `buttonlist` (`MAXBUTTONS` 16,
    /// p_spec.h:244): the tic is refused.
    ButtonSlot15,
    /// A positional sound start of the tic carries no placement, or a placement
    /// has no start: a broken internal invariant, refused rather than published.
    Unplaced,
}

/// The listener as an emitter reads it at the call, under wire 25: the console
/// player's body (`players[consoleplayer].mo`, s_sound.c:304), and its x, y and
/// angle when the body is held outside the staged store at that moment (a mover
/// crossing a line inside its own `P_XYMovement`, whose record the store does not
/// yet hold, p_map.c:488-512).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Listener {
    pub body: ActorId,
    pub held: Option<([doom_types::Fixed; 2], u32)>,
}
impl Listener {
    /// The placement of a positional start whose origin stands at `origin`, the
    /// listener being read from `read` unless it is held.
    pub(crate) fn place(
        self,
        read: impl Fn(ActorId) -> Option<crate::Actor>,
        origin: [doom_types::Fixed; 2],
    ) -> Option<SoundPlacement> {
        let (xy, angle) = self
            .held
            .or_else(|| read(self.body).map(|a| (a.position(), a.angle())))?;
        Some(SoundPlacement {
            origin: origin.map(doom_types::Fixed::bits),
            listener: xy.map(doom_types::Fixed::bits),
            listener_angle: angle,
        })
    }
}

/// Appends the placement of a positional start right after its sound event (the
/// transient [`crate::Event::SoundRecord`]); nothing when the placement cannot be
/// read, which [`split`] then refuses as [`SoundError::Unplaced`].
pub(crate) fn push_placed(
    events: &mut Vec<crate::Event>,
    sfx: u16,
    origin: SoundOrigin,
    at: Option<SoundPlacement>,
) {
    if let Some(at) = at {
        events.push(crate::Event::SoundRecord(SoundRecord::Start {
            sfx,
            origin,
            at: Some(at),
        }));
    }
}

/// The sound records of an event list, in its order: every sound start event
/// (`ActorStateEvent::Sound`, `MoverEvent::SectorSound`) becomes a start, every
/// `RemovalMarked` the stop of `P_RemoveMobj` (p_mobj.c:565-566). A NULL origin or
/// the listener's own body takes no placement (s_sound.c:304); every other start
/// must be followed at once by its transient placement, with the same sound and
/// origin. Appends to `out`; on a refusal `out` may hold a prefix, which the
/// caller drops with the tic. With `strict` false a positional start without a
/// placement is listed with `at: None` instead of refused (only for comparing the
/// calls of a list with records kept apart).
pub(crate) fn split(
    events: &[crate::Event],
    listener: Option<ActorId>,
    out: &mut Vec<SoundRecord>,
    strict: bool,
) -> Result<(), SoundError> {
    use crate::{Event, MoverEvent, lifecycle::ActorStateEvent, planes::SoundOrigin as Kind};
    let mut i = 0;
    while let Some(event) = events.get(i) {
        i += 1;
        let (sfx, origin) = match *event {
            Event::ActorState(ActorStateEvent::Sound { actor, sound }) => {
                (sound, actor.map_or(SoundOrigin::None, SoundOrigin::Actor))
            }
            Event::Mover(MoverEvent::SectorSound {
                origin: Kind::Sector,
                index,
                sound,
            }) => (sound, SoundOrigin::Sector(index)),
            Event::Mover(MoverEvent::SectorSound {
                origin: Kind::Button,
                index,
                sound,
            }) => (
                sound,
                SoundOrigin::Button(u8::try_from(index).map_err(|_| SoundError::Unplaced)?),
            ),
            Event::RemovalMarked(actor) => {
                out.push(SoundRecord::Stop { actor });
                continue;
            }
            Event::SoundRecord(_) => return Err(SoundError::Unplaced),
            _ => continue,
        };
        let positional = match origin {
            SoundOrigin::None => false,
            SoundOrigin::Actor(a) => Some(a) != listener,
            SoundOrigin::Sector(_) | SoundOrigin::Button(_) => true,
        };
        if !positional {
            out.push(SoundRecord::Start {
                sfx,
                origin,
                at: None,
            });
            continue;
        }
        match events.get(i) {
            Some(&Event::SoundRecord(
                record @ SoundRecord::Start {
                    sfx: s,
                    origin: o,
                    at: Some(_),
                },
            )) if s == sfx && o == origin => {
                out.push(record);
                i += 1;
            }
            _ if !strict => out.push(SoundRecord::Start {
                sfx,
                origin,
                at: None,
            }),
            _ => return Err(SoundError::Unplaced),
        }
    }
    Ok(())
}

/// Canonical bytes of one record (DSIM41 `events.sounds.*.N`): tag 1 for a start
/// (u16 sfx; u8 origin kind 0 none, 1 actor, 2 sector, 3 button, then the
/// identity: a 12-byte handle, a u16 sector or a u8 slot; u8 placed, then when 1
/// origin x, y, listener x, y as i32 and the listener angle as u32), tag 2 for a
/// stop (the 12-byte handle). Integers are little endian.
pub(crate) fn record_bytes(record: &SoundRecord) -> Vec<u8> {
    fn handle(b: &mut Vec<u8>, id: ActorId) {
        b.extend(id.slot().to_le_bytes());
        b.extend(id.generation().to_le_bytes());
    }
    let mut b = Vec::new();
    match *record {
        SoundRecord::Start { sfx, origin, at } => {
            b.push(1);
            b.extend(sfx.to_le_bytes());
            match origin {
                SoundOrigin::None => b.push(0),
                SoundOrigin::Actor(id) => {
                    b.push(1);
                    handle(&mut b, id);
                }
                SoundOrigin::Sector(s) => {
                    b.push(2);
                    b.extend(s.to_le_bytes());
                }
                SoundOrigin::Button(slot) => {
                    b.push(3);
                    b.push(slot);
                }
            }
            match at {
                None => b.push(0),
                Some(p) => {
                    b.push(1);
                    for v in [p.origin[0], p.origin[1], p.listener[0], p.listener[1]] {
                        b.extend(v.to_le_bytes());
                    }
                    b.extend(p.listener_angle.to_le_bytes());
                }
            }
        }
        SoundRecord::Stop { actor } => {
            b.push(2);
            handle(&mut b, actor);
        }
    }
    b
}

/// Moves the records of `events` to `out` ([`split`], strict) and drops the
/// transient placements from `events`. On a refusal neither list is changed
/// beyond `out`'s new prefix.
pub(crate) fn take(
    events: &mut Vec<crate::Event>,
    listener: Option<ActorId>,
    out: &mut Vec<SoundRecord>,
) -> Result<(), SoundError> {
    split(events, listener, out, true)?;
    events.retain(|e| !matches!(e, crate::Event::SoundRecord(_)));
    Ok(())
}

/// Whether `records` are exactly the calls of `events` (sound, origin and order;
/// placements are not compared): the check that the records kept apart from
/// `pending` describe it.
pub(crate) fn same_calls(
    events: &[crate::Event],
    listener: Option<ActorId>,
    records: &[SoundRecord],
) -> bool {
    let mut calls = Vec::new();
    if split(events, listener, &mut calls, false).is_err() || calls.len() != records.len() {
        return false;
    }
    let bare = |r: &SoundRecord| match *r {
        SoundRecord::Start { sfx, origin, .. } => SoundRecord::Start {
            sfx,
            origin,
            at: None,
        },
        stop @ SoundRecord::Stop { .. } => stop,
    };
    calls.iter().zip(records).all(|(a, b)| bare(a) == bare(b))
}

/// `M_ClearBox` (m_bbox.c:39-43): `[top, bottom, left, right]` (m_bbox.h's
/// `BOXTOP` 0, `BOXBOTTOM` 1, `BOXLEFT` 2, `BOXRIGHT` 3) = `MININT`, `MAXINT`,
/// `MAXINT`, `MININT` (doomtype.h:48, 54).
pub(crate) const CLEAR_BOX: [i32; 4] = [i32::MIN, i32::MAX, i32::MAX, i32::MIN];
/// `M_AddToBox` (m_bbox.c:46-58), with its `else if` on each axis.
pub(crate) fn add_to_box(b: &mut [i32; 4], [x, y]: [i32; 2]) {
    if x < b[2] {
        b[2] = x;
    } else if x > b[3] {
        b[3] = x;
    }
    if y < b[1] {
        b[1] = y;
    } else if y > b[0] {
        b[0] = y;
    }
}
/// `soundorg.x = (bbox[BOXRIGHT]+bbox[BOXLEFT])/2` and `soundorg.y =
/// (bbox[BOXTOP]+bbox[BOXBOTTOM])/2` (p_setup.c:556-557), the division
/// truncating toward zero; `None` where a sum overflows `int`.
pub(crate) fn box_origin(b: &[i32; 4]) -> Option<[i32; 2]> {
    Some([b[3].checked_add(b[2])? / 2, b[0].checked_add(b[1])? / 2])
}

/// Every sector's sound origin, exactly as `P_GroupLines` computes
/// `sector->soundorg` (p_setup.c:536-557): `M_ClearBox` (m_bbox.c:39-43: top and
/// right `MININT`, bottom and left `MAXINT`), then for every line in `lines`
/// order whose front or back sector is the sector, `M_AddToBox` with its `v1`
/// and then its `v2` (m_bbox.c:46-58, with its `else if`: a point that lowers
/// the left edge never raises the right one), then `(right+left)/2` and
/// `(top+bottom)/2` as signed `int` with the division truncating toward zero (the
/// gcc reading of C89's implementation-defined rounding). A sum that leaves
/// `int` (undefined in C) refuses the map ([`SoundError::SectorOriginOverflow`]).
/// One pass over the lines visits each sector's lines in the same order as the
/// source's per-sector pass; a line whose two sides face the same sector is
/// added once (the source's `||`, p_setup.c:545).
pub(crate) fn sector_sound_origins(
    map: &doom_map::Map,
) -> Result<Vec<[doom_types::Fixed; 2]>, crate::SimError> {
    let sectors = map.sectors().len();
    // [top, bottom, left, right], m_bbox.h's BOXTOP 0, BOXBOTTOM 1, BOXLEFT 2,
    // BOXRIGHT 3.
    let mut boxes = Vec::new();
    boxes
        .try_reserve_exact(sectors)
        .map_err(|_| crate::SimError::from(crate::StoreError::Capacity))?;
    boxes.resize(sectors, CLEAR_BOX);
    let sector_of = |side: Option<u16>| {
        side.and_then(|s| map.sidedefs().get(usize::from(s)))
            .map(|s| s.sector)
    };
    for line in map.linedefs() {
        let front = sector_of(line.sides[0]);
        let back = sector_of(line.sides[1]);
        let mut points = [[0_i32; 2]; 2];
        for (point, v) in points.iter_mut().zip(line.vertices) {
            let v = map
                .vertices()
                .get(usize::from(v))
                .ok_or(crate::SimError::from(
                    crate::GeometryError::InvalidReference,
                ))?;
            *point = [
                doom_types::Fixed::from_map_units(v.x).bits(),
                doom_types::Fixed::from_map_units(v.y).bits(),
            ];
        }
        let mut faced = [front, back];
        if back == front {
            faced[1] = None;
        }
        for sector in faced.into_iter().flatten() {
            let b = boxes
                .get_mut(usize::from(sector))
                .ok_or(crate::SimError::from(
                    crate::GeometryError::InvalidReference,
                ))?;
            for point in points {
                add_to_box(b, point);
            }
        }
    }
    let mut origins = Vec::new();
    origins
        .try_reserve_exact(sectors)
        .map_err(|_| crate::SimError::from(crate::StoreError::Capacity))?;
    for (sector, b) in boxes.iter().enumerate() {
        let overflow = || {
            crate::SimError::from(SoundError::SectorOriginOverflow {
                sector: sector as u16,
            })
        };
        let [x, y] = box_origin(b).ok_or_else(overflow)?;
        origins.push([
            doom_types::Fixed::from_bits(x),
            doom_types::Fixed::from_bits(y),
        ]);
    }
    Ok(origins)
}

/// The listener as the sound logic reads it between tics (`S_UpdateSounds
/// (players[consoleplayer].mo)`, d_main.c:392): the console player's body and its
/// x, y (raw 16.16 bits) and angle.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SoundListener {
    /// The body's handle (also the origin of the player's own sounds).
    pub actor: ActorId,
    /// `mo->x`, `mo->y`.
    pub xy: [i32; 2],
    /// `mo->angle`.
    pub angle: u32,
}

/// Read-only views the sound logic needs after a tic (M5c note §4.5). None of
/// them draws a random number or changes anything.
impl crate::Simulation {
    /// The listener now: the console player's body, `None` without a bound
    /// player body or once it can no longer be read.
    pub fn sound_listener(&self) -> Option<SoundListener> {
        let actor = self.original_player.as_ref()?.mo?;
        let body = self.actors.get(actor)?;
        Some(SoundListener {
            actor,
            xy: body.position().map(doom_types::Fixed::bits),
            angle: body.angle(),
        })
    }
    /// The current x, y (raw 16.16 bits) of an actor origin, `None` once the
    /// handle no longer resolves (a removed actor stays readable until it
    /// retires, as its storage does).
    pub fn sound_origin_xy(&self, actor: ActorId) -> Option<[i32; 2]> {
        self.actors
            .get(actor)
            .map(|a| a.position().map(doom_types::Fixed::bits))
    }
    /// Wire 25: the sound origin of `sector` (`sector->soundorg`, computed once
    /// at load, p_setup.c:536-557) as raw 16.16 bits; `None` under every other
    /// policy and for an index past the sectors.
    pub fn sector_sound_origin(&self, sector: u16) -> Option<[i32; 2]> {
        self.world
            .sound_origin(sector)
            .map(|o| o.map(doom_types::Fixed::bits))
    }
    /// The switch buttons (`buttonlist`, p_switch.c:101): for each of the 16
    /// slots, `None` when free, otherwise its `btimer` (1..=35).
    pub fn sound_buttons(&self) -> [Option<u8>; 16] {
        let mut slots = [None; 16];
        for (slot, button) in slots.iter_mut().zip(&self.buttons) {
            *slot = button.map(|b| b.timer);
        }
        slots
    }
    /// The position the release origin of button slot `slot` reads now
    /// (`(mobj_t *)&buttonlist[slot].soundorg`, p_spec.c:1151) under the declared
    /// 64-bit reading (M5c decision T12): x is slot `slot + 1`'s `btimer` (0 when
    /// free), y is 0. Slot 15 and above read past `buttonlist` and are refused.
    pub fn button_release_xy(&self, slot: u8) -> Result<[i32; 2], SoundError> {
        let next = usize::from(slot)
            .checked_add(1)
            .filter(|n| *n < 16)
            .ok_or(SoundError::ButtonSlot15)?;
        let timer = self
            .buttons
            .get(next)
            .copied()
            .flatten()
            .map_or(0, |b| b.timer);
        Ok([i32::from(timer), 0])
    }
}
