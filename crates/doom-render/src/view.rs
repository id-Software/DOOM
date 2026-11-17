//! The view shadow (decision R6): the presentation half of `P_CalcHeight` (p_user.c:77-141),
//! `P_MovePlayer`'s `onground` (p_user.c:157), `P_DeathThink` (p_user.c:185-198), the player
//! half of `P_ZMovement` (p_mobj.c:251-258, 299-307), `P_Teleport`'s `viewz` write
//! (p_telept.c:106-108) and the `fixedcolormap` of `P_PlayerThink` (p_user.c:339-383).
//!
//! The simulation keeps none of `viewz`, `viewheight`, `deltaviewheight`, `onground` or
//! `fixedcolormap`. The shadow re-derives them exactly by observing every committed tic from
//! level start: it keeps the player state the next tic's `P_PlayerThink` reads (the mobj's
//! `z`, `floorz`, `ceilingz`, `reactiontime`, the player state, the powers and the sector
//! floors), and replays the tic from the stored `bob`, the tic's `leveltime` and the player's
//! vertical-move and teleport events.
use crate::r_main::finesine;
use doom_sim::{Event, MoverEvent, PlayerBodyState, PlayerSlot, PlayerWeaponState, Simulation};
use doom_types::Fixed;

/// `VIEWHEIGHT` (p_local.h:34).
pub const VIEWHEIGHT: i32 = 41 << 16;
/// `GRAVITY` (p_local.h:53).
pub const GRAVITY: i32 = 1 << 16;
/// `FINEANGLES` (tables.h:50).
const FINEANGLES: i32 = 8192;
/// `INVERSECOLORMAP` (p_user.c:41).
pub const INVERSECOLORMAP: i32 = 32;

/// Why the shadow could not observe a tic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShadowError {
    /// The simulation is not at the next tic after the last observed one (a tic was skipped,
    /// observed twice, or refused by the simulation and so not committed).
    Gap {
        /// The tic the shadow expected.
        expected: u64,
        /// The simulation's tic.
        got: u64,
    },
    /// The simulation has no original player, player mobj, weapon state, location or powers
    /// (a profile the renderer does not admit).
    NoPlayer,
    /// A teleport landing is missing or has no sector.
    Landing,
    /// `FINEANGLES/20*leveltime` (p_user.c:108, evaluated on the tics `P_CalcHeight` finds
    /// the player on the ground) overflows `int` (signed overflow, undefined in C) from
    /// `leveltime` 5,250,572 on (about 41.7 hours), or `leveltime` itself would pass `INT_MAX`
    /// (`leveltime++`, p_tick.c:157).
    LevelTimeOverflow {
        /// The simulation's tic.
        tic: u64,
    },
}

/// The view state of one tic and what the next tic's player think reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewShadow {
    /// `player->viewheight`.
    pub viewheight: i32,
    /// `player->deltaviewheight`.
    pub deltaviewheight: i32,
    /// `player->viewz`.
    pub viewz: i32,
    /// The `onground` global (p_user.c:51).
    pub onground: bool,
    /// `player->fixedcolormap`.
    pub fixedcolormap: i32,
    /// The player mobj's `z` after the last observed tic.
    pub z: i32,
    /// Its `floorz`.
    pub floorz: i32,
    /// Its `ceilingz`.
    pub ceilingz: i32,
    /// Its `reactiontime`.
    pub reactiontime: i32,
    /// Whether the player was `PST_DEAD`.
    pub dead: bool,
    /// `player->powers`.
    pub powers: [i32; 6],
    /// Every sector's floor height.
    pub sector_floors: Vec<i32>,
}

/// The player inputs the shadow reads after a tic.
struct PlayerNow {
    mo: doom_sim::ActorId,
    z: i32,
    floorz: i32,
    ceilingz: i32,
    reactiontime: i32,
    dead: bool,
    powers: [i32; 6],
    weapons: PlayerWeaponState,
}

fn player_now(sim: &Simulation) -> Result<PlayerNow, ShadowError> {
    let p = sim
        .original_player(PlayerSlot::One)
        .map_err(|_| ShadowError::NoPlayer)?;
    let mo = p.mo.ok_or(ShadowError::NoPlayer)?;
    let actor = sim.actor(mo).ok_or(ShadowError::NoPlayer)?;
    let life = actor.life().ok_or(ShadowError::NoPlayer)?;
    let powers = sim
        .original_player_powers(PlayerSlot::One)
        .map_err(|_| ShadowError::NoPlayer)?
        .powers;
    let weapons = sim
        .original_player_weapons(PlayerSlot::One)
        .ok()
        .flatten()
        .ok_or(ShadowError::NoPlayer)?;
    Ok(PlayerNow {
        mo,
        z: actor.z().bits(),
        floorz: actor.floor().bits(),
        ceilingz: actor.ceiling().bits(),
        reactiontime: life.reaction_time,
        dead: p.state == PlayerBodyState::Dead,
        powers,
        weapons,
    })
}

/// `player->fixedcolormap` from the powers after `P_PlayerThink`'s countdowns
/// (p_user.c:362-383).
pub fn fixed_colormap(powers: [i32; 6]) -> i32 {
    let invul = powers[0];
    let infrared = powers[5];
    if invul != 0 {
        if invul > 4 * 32 || invul & 8 != 0 {
            INVERSECOLORMAP
        } else {
            0
        }
    } else if infrared != 0 {
        if infrared > 4 * 32 || infrared & 8 != 0 {
            1
        } else {
            0
        }
    } else {
        0
    }
}

impl ViewShadow {
    /// The state after the level load (tic 0): `P_SpawnPlayer` sets `viewheight =
    /// VIEWHEIGHT` (p_mobj.c:683) after `G_PlayerReborn` cleared the player (so `viewz` and
    /// `deltaviewheight` are 0; p_setup.c:604's `viewz = 1` precedes that clear),
    /// `fixedcolormap` is 0 and the `onground` global is 0 in a freshly started process.
    pub fn level_start(sim: &Simulation) -> Result<Self, ShadowError> {
        if sim.tic() != 0 {
            return Err(ShadowError::Gap {
                expected: 0,
                got: sim.tic(),
            });
        }
        let p = player_now(sim)?;
        Ok(Self {
            viewheight: VIEWHEIGHT,
            deltaviewheight: 0,
            viewz: 0,
            onground: false,
            fixedcolormap: 0,
            z: p.z,
            floorz: p.floorz,
            ceilingz: p.ceilingz,
            reactiontime: p.reactiontime,
            dead: p.dead,
            powers: p.powers,
            sector_floors: sim.sectors().iter().map(|s| s.floor.bits()).collect(),
        })
    }

    /// The shadow after the simulation's latest tic (`sim.tic()`), from this shadow (the
    /// previous tic). Pure: `self` is unchanged.
    pub fn next(&self, sim: &Simulation) -> Result<Self, ShadowError> {
        let now = player_now(sim)?;
        // `leveltime` during the tic's player think is the tic count before it (P_Ticker
        // increments it after the thinkers, p_tick.c:157); `FINEANGLES/20*leveltime` must fit `int`.
        let overflow = ShadowError::LevelTimeOverflow { tic: sim.tic() };
        let leveltime = sim
            .tic()
            .checked_sub(1)
            .and_then(|t| i32::try_from(t).ok())
            .ok_or(overflow.clone())?;
        let mut s = self.clone();
        let bob = now.weapons.bob.bits();
        // P_PlayerThink (p_user.c:249-272).
        let live = !self.dead;
        if !live {
            // P_DeathThink (p_user.c:185-198).
            if s.viewheight > 6 << 16 {
                s.viewheight -= 1 << 16;
            }
            if s.viewheight < 6 << 16 {
                s.viewheight = 6 << 16;
            }
            s.deltaviewheight = 0;
            s.onground = self.z <= self.floorz;
        } else if self.reactiontime == 0 {
            // P_MovePlayer (p_user.c:157); skipped while reactiontime counts down.
            s.onground = self.z <= self.floorz;
        }
        s.calc_height(bob, leveltime, live).ok_or(overflow)?;
        if live {
            s.fixedcolormap = fixed_colormap(countdown(self.powers));
        }
        // P_RunThinkers: the player's own mobj thinker, in event order.
        for e in sim.events() {
            match e {
                Event::Mover(MoverEvent::Teleported { actor, landing, .. }) if *actor == now.mo => {
                    // p_telept.c:106-108: z = floorz of the landing's sector (sector thinkers
                    // run after the player, so its floor is the pre-tic one).
                    let sector = sim
                        .actor(*landing)
                        .and_then(doom_sim::Actor::location)
                        .ok_or(ShadowError::Landing)?
                        .sector;
                    let floor = *self
                        .sector_floors
                        .get(usize::from(sector))
                        .ok_or(ShadowError::Landing)?;
                    s.viewz = floor.wrapping_add(s.viewheight);
                }
                Event::Vertical { actor, update } if *actor == now.mo => {
                    let [z, momz] = update.before.map(Fixed::bits);
                    let floor = update.constraints[0].bits();
                    if z < floor {
                        s.viewheight = s.viewheight.wrapping_sub(floor.wrapping_sub(z));
                        s.deltaviewheight = VIEWHEIGHT.wrapping_sub(s.viewheight) >> 3;
                    }
                    if update.floor_contact && momz < 0 && momz < -GRAVITY * 8 {
                        s.deltaviewheight = momz >> 3;
                    }
                }
                _ => {}
            }
        }
        s.z = now.z;
        s.floorz = now.floorz;
        s.ceilingz = now.ceilingz;
        s.reactiontime = now.reactiontime;
        s.dead = now.dead;
        s.powers = now.powers;
        s.sector_floors = sim.sectors().iter().map(|x| x.floor.bits()).collect();
        Ok(s)
    }

    /// `P_CalcHeight` (p_user.c:77-141) with `player->bob` as the simulation stored it, the
    /// mobj's pre-tic `z` and `ceilingz`, and `CF_NOMOMENTUM` clear (no cheat input path).
    /// `None` when `FINEANGLES/20*leveltime` overflows `int`; the caller refuses the tic.
    fn calc_height(&mut self, bob: i32, leveltime: i32, live: bool) -> Option<()> {
        if !self.onground {
            // The clamp of p_user.c:99-102 is overwritten at once (p_user.c:104).
            self.viewz = self.z.wrapping_add(self.viewheight);
            return Some(());
        }
        let angle = (FINEANGLES / 20).checked_mul(leveltime)? & (FINEANGLES - 1);
        let b = Fixed::from_bits(bob / 2)
            .fixed_mul(Fixed::from_bits(finesine(angle).unwrap_or(0)))
            .bits();
        if live {
            self.viewheight = self.viewheight.wrapping_add(self.deltaviewheight);
            if self.viewheight > VIEWHEIGHT {
                self.viewheight = VIEWHEIGHT;
                self.deltaviewheight = 0;
            }
            if self.viewheight < VIEWHEIGHT / 2 {
                self.viewheight = VIEWHEIGHT / 2;
                if self.deltaviewheight <= 0 {
                    self.deltaviewheight = 1;
                }
            }
            if self.deltaviewheight != 0 {
                self.deltaviewheight = self.deltaviewheight.wrapping_add(1 << 14);
                if self.deltaviewheight == 0 {
                    self.deltaviewheight = 1;
                }
            }
        }
        self.viewz = self.z.wrapping_add(self.viewheight).wrapping_add(b);
        let limit = self.ceilingz.wrapping_sub(4 << 16);
        if self.viewz > limit {
            self.viewz = limit;
        }
        Some(())
    }
}

/// The countdowns of `P_PlayerThink` (p_user.c:339-353) that `fixedcolormap` reads
/// (invulnerability and infrared).
fn countdown(mut powers: [i32; 6]) -> [i32; 6] {
    for i in [0, 5] {
        if powers[i] != 0 {
            powers[i] -= 1;
        }
    }
    powers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colormap_rule() {
        assert_eq!(fixed_colormap([0; 6]), 0);
        assert_eq!(fixed_colormap([0, 0, 0, 0, 0, 129]), 1);
        assert_eq!(fixed_colormap([0, 0, 0, 0, 0, 128]), 0);
        assert_eq!(fixed_colormap([0, 0, 0, 0, 0, 7]), 0);
        assert_eq!(fixed_colormap([0, 0, 0, 0, 0, 8]), 1);
        assert_eq!(fixed_colormap([200, 0, 0, 0, 0, 200]), INVERSECOLORMAP);
        assert_eq!(fixed_colormap([7, 0, 0, 0, 0, 200]), 0);
        assert_eq!(countdown([1, 1, 0, 0, 0, 0]), [0, 1, 0, 0, 0, 0]);
    }

    fn shadow(onground: bool) -> ViewShadow {
        ViewShadow {
            viewheight: VIEWHEIGHT,
            deltaviewheight: 0,
            viewz: 0,
            onground,
            fixedcolormap: 0,
            z: 0,
            floorz: 0,
            ceilingz: 128 << 16,
            reactiontime: 0,
            dead: false,
            powers: [0; 6],
            sector_floors: vec![],
        }
    }

    #[test]
    fn the_bob_angle_overflow_is_refused_on_the_ground_only() {
        // `FINEANGLES/20*leveltime` (p_user.c:108): 409 * 5,250,571 = 2,147,483,539 fits
        // `int`, 409 * 5,250,572 does not. The product is evaluated only past the
        // `!onground` return (p_user.c:97-105).
        assert_eq!(shadow(true).calc_height(0, 5_250_571, true), Some(()));
        assert_eq!(shadow(true).calc_height(0, 5_250_572, true), None);
        assert_eq!(shadow(true).calc_height(0, i32::MAX, false), None);
        let mut air = shadow(false);
        assert_eq!(air.calc_height(0, i32::MAX, true), Some(()));
        assert_eq!(air.viewz, VIEWHEIGHT);
    }

    #[test]
    fn the_constants_are_the_headers_values() {
        // p_local.h:34 `VIEWHEIGHT (41*FRACUNIT)`, p_local.h:53 `GRAVITY FRACUNIT`,
        // p_user.c:41 `INVERSECOLORMAP 32`, tables.h:50 `FINEANGLES 8192`.
        assert_eq!(VIEWHEIGHT, 41 * 65536);
        assert_eq!(GRAVITY, 65536);
        assert_eq!(INVERSECOLORMAP, 32);
        assert_eq!(FINEANGLES, 8192);
    }

    #[test]
    fn colormap_thresholds_are_over_four_times_thirty_two_or_bit_three() {
        // p_user.c:362-383: powers > 4*32 (128) or bit 8 selects the special map.
        let inv = |n| fixed_colormap([n, 0, 0, 0, 0, 0]);
        let inf = |n| fixed_colormap([0, 0, 0, 0, 0, n]);
        for (n, want) in [
            (1, 0),
            (8, 1),
            (9, 1),
            (15, 1),
            (16, 0),
            (40, 1),
            (100, 0),
            (127, 1),
            (112, 0),
            (128, 0),
            (129, 1),
            (130, 1),
            (136, 1),
            (144, 1),
        ] {
            assert_eq!(inv(n), want * INVERSECOLORMAP, "invulnerability {n}");
            assert_eq!(inf(n), want, "infrared {n}");
        }
        // Invulnerability wins over infrared, even when it selects map 0
        // (`else if`, p_user.c:371).
        assert_eq!(fixed_colormap([100, 0, 0, 0, 0, 200]), 0);
    }

    #[test]
    fn viewheight_adjusts_on_the_ground_at_the_exact_bounds() {
        // P_CalcHeight (p_user.c:112-130) for a live player on the ground (bob 0).
        // viewheight reaching exactly VIEWHEIGHT is not clamped: deltaviewheight survives
        // and gains FRACUNIT/4 (p_user.c:112-117, 126-129).
        let mut s = shadow(true);
        s.viewheight = VIEWHEIGHT - (1 << 14);
        s.deltaviewheight = 1 << 14;
        assert_eq!(s.calc_height(0, 0, true), Some(()));
        assert_eq!(s.viewheight, VIEWHEIGHT);
        assert_eq!(s.deltaviewheight, 2 << 14);
        // One past it is clamped and the delta zeroed (and stays 0: `if (deltaviewheight)`).
        let mut s = shadow(true);
        s.viewheight = VIEWHEIGHT;
        s.deltaviewheight = 1;
        s.calc_height(0, 0, true).unwrap();
        assert_eq!((s.viewheight, s.deltaviewheight), (VIEWHEIGHT, 0));
        // viewheight reaching exactly VIEWHEIGHT/2 is not clamped, so a negative delta is
        // not replaced by 1 (p_user.c:119-125): -FRACUNIT + FRACUNIT/4.
        let mut s = shadow(true);
        s.viewheight = VIEWHEIGHT / 2 + (1 << 16);
        s.deltaviewheight = -(1 << 16);
        s.calc_height(0, 0, true).unwrap();
        assert_eq!(s.viewheight, VIEWHEIGHT / 2);
        assert_eq!(s.deltaviewheight, -(1 << 16) + (1 << 14));
        // One below is clamped, and a non-positive delta becomes 1, then 1 + FRACUNIT/4.
        let mut s = shadow(true);
        s.viewheight = VIEWHEIGHT / 2;
        s.deltaviewheight = -1;
        s.calc_height(0, 0, true).unwrap();
        assert_eq!(s.viewheight, VIEWHEIGHT / 2);
        assert_eq!(s.deltaviewheight, 1 + (1 << 14));
        // A delta that reaches 0 after the increment becomes 1 (p_user.c:127-128).
        let mut s = shadow(true);
        s.deltaviewheight = -(1 << 14);
        s.calc_height(0, 0, true).unwrap();
        assert_eq!(s.deltaviewheight, 1);
    }

    #[test]
    fn viewz_stays_four_units_below_the_ceiling() {
        // p_user.c:132-135: `if (viewz > ceilingz - 4*FRACUNIT) viewz = ceilingz - 4*FRACUNIT`.
        let mut s = shadow(true);
        s.ceilingz = 44 << 16; // limit 40 < z + viewheight = 41
        s.calc_height(0, 0, true).unwrap();
        assert_eq!(s.viewz, 40 << 16);
        let mut s = shadow(true);
        s.ceilingz = 45 << 16; // limit 41: exactly viewz, unchanged
        s.calc_height(0, 0, true).unwrap();
        assert_eq!(s.viewz, 41 << 16);
        let mut s = shadow(true);
        s.ceilingz = 46 << 16;
        s.calc_height(0, 0, true).unwrap();
        assert_eq!(s.viewz, 41 << 16);
    }
}
