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
        .map(|p| p.powers)
        .unwrap_or([0; 6]);
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
        let leveltime = sim.tic().wrapping_sub(1) as i32;
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
        s.calc_height(bob, leveltime, live);
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
    fn calc_height(&mut self, bob: i32, leveltime: i32, live: bool) {
        if !self.onground {
            // The clamp of p_user.c:99-102 is overwritten at once (p_user.c:104).
            self.viewz = self.z.wrapping_add(self.viewheight);
            return;
        }
        let angle = (8192 / 20 * leveltime) & 8191;
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
}
