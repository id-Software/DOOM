//! The status bar of st_stuff.c: its graphics (`ST_loadGraphics`, st_stuff.c:1124-1199, and
//! `STlib_init`'s `STTMINUS`, st_lib.c:58-61), its per-tic ticker (`ST_Ticker`, st_stuff.c:988-996,
//! with `ST_updateWidgets` and the face state machine `ST_updateFaceWidget`), the palette choice
//! (`ST_doPaletteStuff`, st_stuff.c:1000-1052) and the drawer (`ST_Drawer`, st_stuff.c:1108-1122).
//!
//! Single player only: `deathmatch` and `netgame` are 0 and `consoleplayer` is 0, so the frags
//! widget and the face background are never drawn and the arms are always shown. No
//! `ST_Responder`: cheats are outside the app (decision S1). Facts: M5b investigation §1.4.
use crate::st_lib::{self, LARGEAMMO, ST_Y};
use crate::v_video::{Canvas, DrawError, Patch, Screen};
use crate::{ResourceError, Resources};
use doom_types::DoomRng;

/// `ST_NUMFACES` (st_stuff.c:107-108): 5 pain levels of 8 faces, the god and the dead face.
pub const NUMFACES: usize = 42;
/// `ST_GODFACE` (st_stuff.c:114).
pub const GODFACE: i32 = 40;
/// `ST_DEADFACE` (st_stuff.c:115).
pub const DEADFACE: i32 = 41;
const TURNOFFSET: i32 = 3; // ST_TURNOFFSET, st_stuff.c:110
const OUCHOFFSET: i32 = 5; // ST_OUCHOFFSET, st_stuff.c:111
const EVILGRINOFFSET: i32 = 6; // ST_EVILGRINOFFSET, st_stuff.c:112
const RAMPAGEOFFSET: i32 = 7; // ST_RAMPAGEOFFSET, st_stuff.c:113
const EVILGRINCOUNT: i32 = 70; // 2*TICRATE, st_stuff.c:120
const STRAIGHTFACECOUNT: i32 = 17; // TICRATE/2, st_stuff.c:121
const TURNCOUNT: i32 = 35; // 1*TICRATE, st_stuff.c:122
const RAMPAGEDELAY: i32 = 70; // 2*TICRATE, st_stuff.c:124
const MUCHPAIN: i32 = 20; // st_stuff.c:126
const ANG45: u32 = 0x2000_0000; // tables.h:68
const ANG180: u32 = 0x8000_0000; // tables.h:70
/// `weaponinfo[w].ammo` (d_items.c:51-123): the ammunition index of each `weapontype_t`, `None`
/// for `am_noammo` (fist and chainsaw).
const WEAPON_AMMO: [Option<u8>; 9] = [
    None,
    Some(0),
    Some(1),
    Some(0),
    Some(3),
    Some(2),
    Some(2),
    None,
    Some(1),
];
/// `w_ammo[i]` and `w_maxammo[i]` y (st_stuff.c:181-206): clip, shell, cell, missile.
const AMMO_Y: [i32; 4] = [173, 179, 191, 185];

/// The status bar's lumps, resolved in `ST_loadGraphics` order (`W_CacheLumpName` is
/// `W_GetNumForName`: the last match wins, a missing name is `I_Error`, w_wad.c:406).
#[derive(Clone, Debug)]
pub struct StatusGraphics {
    /// `tallnum` (`STTNUM0`-`9`).
    pub tallnum: [Patch; 10],
    /// `shortnum` (`STYSNUM0`-`9`).
    pub shortnum: [Patch; 10],
    /// `tallpercent` (`STTPRCNT`).
    pub tallpercent: Patch,
    /// `keys` (`STKEYS0`-`5`).
    pub keys: [Patch; 6],
    /// `armsbg` (`STARMS`).
    pub armsbg: Patch,
    /// `arms[i][0]` (`STGNUM2`-`7`); `arms[i][1]` is `shortnum[i+2]` (st_stuff.c:1166).
    pub arms_gray: [Patch; 6],
    /// `faceback` (`STFB0`): loaded, drawn only in a netgame (st_stuff.c:506-507).
    pub faceback: Patch,
    /// `sbar` (`STBAR`).
    pub sbar: Patch,
    /// `faces` in the order of st_stuff.c:1177-1197.
    pub faces: Vec<Patch>,
    /// `sttminus` (`STTMINUS`, st_lib.c:60).
    pub minus: Patch,
}

/// Resolves `name` and validates it as a patch.
pub(crate) fn patch_named(res: &Resources, name: &str) -> Result<Patch, ResourceError> {
    let lump = res.directory().get_num_for_name(name.as_bytes())?;
    Patch::load(res, lump).ok_or(ResourceError::PatchBounds { lump })
}

impl StatusGraphics {
    /// Loads every status bar lump from the global lump list.
    pub fn load(res: &Resources) -> Result<Self, ResourceError> {
        let p = |n: &str| patch_named(res, n);
        let ten = |f: &dyn Fn(usize) -> String| -> Result<[Patch; 10], ResourceError> {
            let v = (0..10).map(|i| p(&f(i))).collect::<Result<Vec<_>, _>>()?;
            Ok(v.try_into().expect("ten"))
        };
        let tallnum = ten(&|i| format!("STTNUM{i}"))?;
        let shortnum = ten(&|i| format!("STYSNUM{i}"))?;
        let tallpercent = p("STTPRCNT")?;
        let keys = (0..6)
            .map(|i| p(&format!("STKEYS{i}")))
            .collect::<Result<Vec<_>, _>>()?;
        let armsbg = p("STARMS")?;
        let arms = (0..6)
            .map(|i| p(&format!("STGNUM{}", i + 2)))
            .collect::<Result<Vec<_>, _>>()?;
        let faceback = p("STFB0")?;
        let sbar = p("STBAR")?;
        let mut faces = Vec::with_capacity(NUMFACES);
        for i in 0..5 {
            for j in 0..3 {
                faces.push(p(&format!("STFST{i}{j}"))?);
            }
            for n in ["STFTR{}0", "STFTL{}0", "STFOUCH{}", "STFEVL{}", "STFKILL{}"] {
                faces.push(p(&n.replace("{}", &i.to_string()))?);
            }
        }
        faces.push(p("STFGOD0")?);
        faces.push(p("STFDEAD0")?);
        Ok(Self {
            tallnum,
            shortnum,
            tallpercent,
            keys: keys.try_into().expect("six"),
            armsbg,
            arms_gray: arms.try_into().expect("six"),
            faceback,
            sbar,
            faces,
            minus: p("STTMINUS")?,
        })
    }
}

/// Where the face looks for its attacker (`plyr->attacker`, st_stuff.c:800-817).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Attacker {
    /// `attacker` is NULL.
    #[default]
    None,
    /// `attacker == plyr->mo` (own splash damage).
    Player,
    /// Another thing, at this position (16.16) after the tic.
    At([i32; 2]),
    /// Another thing whose storage has been freed: the genuine face would read freed memory
    /// (st_stuff.c:814-817), so the tic is refused (decision S17).
    Removed,
}

/// Everything of `*plyr` the status bar reads, after the tic (`ST_Ticker` runs after `P_Ticker`,
/// g_game.c:730-731) and at draw time (one display per tic, so the two are the same values).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StatusInputs {
    /// `health`.
    pub health: i32,
    /// `armorpoints`.
    pub armor: i32,
    /// `ammo[4]` (clip, shell, cell, missile).
    pub ammo: [i32; 4],
    /// `maxammo[4]`.
    pub maxammo: [i32; 4],
    /// `weaponowned[NUMWEAPONS]`.
    pub weaponowned: [bool; 9],
    /// `readyweapon` (`weapontype_t`, 0..9).
    pub readyweapon: u8,
    /// `cards[NUMCARDS]`.
    pub cards: [bool; 6],
    /// `damagecount`.
    pub damagecount: i32,
    /// `bonuscount`.
    pub bonuscount: i32,
    /// `powers[NUMPOWERS]`.
    pub powers: [i32; 6],
    /// `cheats & CF_GODMODE` (never set by the app: no cheats, decision S1).
    pub godmode: bool,
    /// `attackdown`.
    pub attackdown: bool,
    /// `attacker`.
    pub attacker: Attacker,
    /// `mo->x`, `mo->y` (16.16).
    pub position: [i32; 2],
    /// `mo->angle`.
    pub angle: u32,
}

/// The status bar's own history (st_stuff.c statics and the widgets' old values; §1.4.2,
/// §1.4.13), saved in the display state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusBarState {
    /// `rndindex` of `M_Random` (m_random.c:63-66): 0 at a new game, advanced only by the
    /// ticker under the declared sound-free, wipe-free contract (decision S15).
    pub rndindex: u8,
    /// `st_oldhealth` (st_stuff.c:378, 994).
    pub oldhealth: i32,
    /// `oldweaponsowned` (st_stuff.c:381).
    pub oldweaponsowned: [bool; 9],
    /// `st_facecount` (st_stuff.c:384; not reset by `ST_Start`).
    pub facecount: i32,
    /// `st_faceindex` (st_stuff.c:387).
    pub faceindex: i32,
    /// `priority` (st_stuff.c:758; not reset by `ST_Start`).
    pub priority: i32,
    /// `lastattackdown` (st_stuff.c:757; not reset by `ST_Start`).
    pub lastattackdown: i32,
    /// `keyboxes` (st_stuff.c:390).
    pub keyboxes: [i32; 3],
    /// What `w_ready.num` points at (st_stuff.c:932-935): an ammunition index, or `None` for
    /// `largeammo` (1994).
    pub ready_ammo: Option<u8>,
    /// `st_palette` (st_stuff.c:998; -1 after `ST_Start`).
    pub palette: i32,
    /// `st_firsttime` (st_stuff.c:273).
    pub firsttime: bool,
    /// Whether `screens[4]` has been drawn: it is then STBAR over indeterminate memory, a pure
    /// function of the STBAR lump (st_stuff.c:504).
    pub bg_drawn: bool,
    /// `w_arms[i].oldinum`.
    pub arms_old: [i32; 6],
    /// `w_faces.oldinum`.
    pub face_old: i32,
    /// `w_keyboxes[i].oldinum`.
    pub keys_old: [i32; 3],
    /// `w_armsbg.oldval`.
    pub armsbg_old: bool,
}

/// `ST_calcPainOffset` (st_stuff.c:729-743). Its cache is keyed on the clamped health, which is
/// never negative (p_inter.c:870-872), so the cached value always equals the expression.
pub fn pain_offset(health: i32) -> i32 {
    let h = health.min(100);
    8 * (((100 - h) * 5) / 101)
}

/// `ST_doPaletteStuff`'s choice (st_stuff.c:1006-1043): the PLAYPAL index for the player state.
pub fn palette_for(i: &StatusInputs) -> i32 {
    let mut cnt = i.damagecount;
    let strength = i.powers[1];
    if strength != 0 {
        let bzc = 12 - (strength >> 6);
        if bzc > cnt {
            cnt = bzc;
        }
    }
    if cnt != 0 {
        ((cnt + 7) >> 3).min(7) + 1 // STARTREDPALS, NUMREDPALS
    } else if i.bonuscount != 0 {
        ((i.bonuscount + 7) >> 3).min(3) + 9 // STARTBONUSPALS, NUMBONUSPALS
    } else if i.powers[3] > 4 * 32 || i.powers[3] & 8 != 0 {
        13 // RADIATIONPAL
    } else {
        0
    }
}

impl StatusBarState {
    /// A fresh process (static initial values) after `ST_Start` (st_stuff.c:1444-1454) with the
    /// player's starting weapons: `ST_initData` (st_stuff.c:1250-1278) and `ST_createWidgets`'
    /// old values (st_lib.c:203, 255).
    pub fn level_start(weaponowned: [bool; 9]) -> Self {
        Self {
            rndindex: 0,
            oldhealth: -1,
            oldweaponsowned: weaponowned,
            facecount: 0,
            faceindex: 0,
            priority: 0,
            lastattackdown: -1,
            keyboxes: [-1; 3],
            ready_ammo: None,
            palette: -1,
            firsttime: true,
            bg_drawn: false,
            arms_old: [-1; 6],
            face_old: -1,
            keys_old: [-1; 3],
            armsbg_old: false,
        }
    }

    /// `ST_Ticker` (st_stuff.c:988-996): one `M_Random`, `ST_updateWidgets` and `st_oldhealth`.
    /// A removed attacker the face could read refuses the tic first (decision S17, as the genuine
    /// runner does before the genuine `ST_Ticker`). That refusal is wider than the C's read:
    /// both refuse whenever `damagecount` is nonzero and the attacker is gone, but the genuine
    /// face reads `attacker->x` and `->y` only in its attacker branch, that is with `priority`
    /// below 8 and `health - st_oldhealth` at most `ST_MUCHPAIN` (st_stuff.c:798-817). With a
    /// dead player (priority 9), an evil grin (8) or the reversed ouch test the genuine code
    /// reads nothing freed, and the tic is refused all the same: a conservative refusal, never
    /// a divergent frame.
    pub fn tick(&mut self, p: &StatusInputs) -> Result<(), Attacker> {
        if p.damagecount != 0 && p.attacker == Attacker::Removed {
            return Err(Attacker::Removed);
        }
        let mut rng = DoomRng::from_indices(0, self.rndindex);
        let random = i32::from(rng.m_random());
        self.rndindex = rng.indices()[1];
        // ST_updateWidgets (st_stuff.c:924-986).
        self.tick_widgets(p);
        self.update_face(p, random);
        self.oldhealth = p.health;
        Ok(())
    }

    /// `ST_updateWidgets`' ready-ammunition pointer and key boxes (st_stuff.c:932-958).
    pub(crate) fn tick_widgets(&mut self, p: &StatusInputs) {
        self.ready_ammo = WEAPON_AMMO[usize::from(p.readyweapon)];
        for i in 0..3 {
            self.keyboxes[i] = if p.cards[i] { i as i32 } else { -1 };
            if p.cards[i + 3] {
                self.keyboxes[i] = i as i32 + 3;
            }
        }
    }

    /// `ST_updateFaceWidget` (st_stuff.c:752-922), with `random` the tic's `st_randomnumber`.
    fn update_face(&mut self, p: &StatusInputs, random: i32) {
        let pain = || pain_offset(p.health);
        if p.health == 0 {
            self.priority = 9;
            self.faceindex = DEADFACE;
            self.facecount = 1;
        }
        if self.priority < 9 && p.bonuscount != 0 {
            let mut grin = false;
            for (old, now) in self.oldweaponsowned.iter_mut().zip(p.weaponowned) {
                if *old != now {
                    grin = true;
                    *old = now;
                }
            }
            if grin {
                self.priority = 8;
                self.facecount = EVILGRINCOUNT;
                self.faceindex = pain() + EVILGRINOFFSET;
            }
        }
        let attacked = match p.attacker {
            Attacker::At(xy) if p.damagecount != 0 => Some(xy),
            _ => None,
        };
        if self.priority < 8 {
            if let Some([ax, ay]) = attacked {
                self.priority = 7;
                self.facecount = TURNCOUNT;
                if p.health - self.oldhealth > MUCHPAIN {
                    self.faceindex = pain() + OUCHOFFSET;
                } else {
                    // R_PointToAngle2 (r_main.c:378-388): the angle from mo to the attacker.
                    let bad = crate::r_main::point_to_angle(ax, ay, p.position[0], p.position[1]);
                    let (diff, right) = if bad > p.angle {
                        let d = bad.wrapping_sub(p.angle);
                        (d, d > ANG180)
                    } else {
                        let d = p.angle.wrapping_sub(bad);
                        (d, d <= ANG180)
                    };
                    self.faceindex = pain()
                        + if diff < ANG45 {
                            RAMPAGEOFFSET
                        } else if right {
                            TURNOFFSET
                        } else {
                            TURNOFFSET + 1
                        };
                }
            }
        }
        if self.priority < 7 && p.damagecount != 0 {
            self.facecount = TURNCOUNT;
            if p.health - self.oldhealth > MUCHPAIN {
                self.priority = 7;
                self.faceindex = pain() + OUCHOFFSET;
            } else {
                self.priority = 6;
                self.faceindex = pain() + RAMPAGEOFFSET;
            }
        }
        if self.priority < 6 {
            if !p.attackdown {
                self.lastattackdown = -1;
            } else if self.lastattackdown == -1 {
                self.lastattackdown = RAMPAGEDELAY;
            } else {
                self.lastattackdown -= 1;
                if self.lastattackdown == 0 {
                    self.priority = 5;
                    self.faceindex = pain() + RAMPAGEOFFSET;
                    self.facecount = 1;
                    self.lastattackdown = 1;
                }
            }
        }
        if self.priority < 5 && (p.godmode || p.powers[0] != 0) {
            self.priority = 4;
            self.faceindex = GODFACE;
            self.facecount = 1;
        }
        if self.facecount == 0 {
            self.faceindex = pain() + random % 3;
            self.facecount = STRAIGHTFACECOUNT;
            self.priority = 0;
        }
        self.facecount -= 1;
    }
}

/// What one `ST_Drawer` call reports.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusFrame {
    /// `st_palette` after the call.
    pub palette: u8,
    /// The palette passed to `I_SetPalette` in this call, if any (st_stuff.c:1045-1050).
    pub palette_set: Option<u8>,
    /// `st_faceindex`, the face the call draws (or would draw with the bar on).
    pub face: i32,
}

/// `ST_Drawer(fullscreen, refresh)` (st_stuff.c:1108-1122) into `c`, with `st_statusbaron =
/// !fullscreen` (the automap is never active).
pub(crate) fn draw(
    s: &mut StatusBarState,
    g: &StatusGraphics,
    p: &StatusInputs,
    c: &mut Canvas<'_>,
    fullscreen: bool,
    refresh: bool,
) -> Result<StatusFrame, DrawError> {
    let on = !fullscreen;
    s.firsttime = s.firsttime || refresh;
    let palette = palette_for(p);
    let mut palette_set = None;
    if palette != s.palette {
        s.palette = palette;
        palette_set = Some(palette as u8);
    }
    let refresh = s.firsttime;
    if refresh {
        s.firsttime = false;
        if on {
            // ST_refreshBackground (st_stuff.c:499-512): STBAR into BG, then BG to FG rows 168-199.
            c.patch(0, 0, Screen::Bg, &g.sbar)?;
            s.bg_drawn = true;
            c.copy_bg([0, 0], [320, 32], [0, ST_Y])?;
        }
    }
    if on {
        // ST_drawWidgets (st_stuff.c:1054-1087), in order; `*on` is st_statusbaron throughout.
        let ready = s.ready_ammo.map_or(LARGEAMMO, |a| p.ammo[usize::from(a)]);
        st_lib::draw_num(c, [44, 171], 3, ready, &g.tallnum, &g.minus)?;
        for i in 0..4 {
            st_lib::draw_num(c, [288, AMMO_Y[i]], 3, p.ammo[i], &g.shortnum, &g.minus)?;
            st_lib::draw_num(c, [314, AMMO_Y[i]], 3, p.maxammo[i], &g.shortnum, &g.minus)?;
        }
        let (tn, mn, pc) = (&g.tallnum, &g.minus, &g.tallpercent);
        st_lib::update_percent(c, refresh, [90, 171], p.health, tn, mn, pc)?;
        st_lib::update_percent(c, refresh, [221, 171], p.armor, tn, mn, pc)?;
        st_lib::update_binicon(c, refresh, [104, 168], &mut s.armsbg_old, true, &g.armsbg)?;
        for i in 0..6 {
            let xy = [111 + (i as i32 % 3) * 12, 172 + (i as i32 / 3) * 10];
            let pair = [g.arms_gray[i], g.shortnum[i + 2]];
            let v = i32::from(p.weaponowned[i + 1]);
            st_lib::update_multicon(c, refresh, xy, &mut s.arms_old[i], v, &pair)?;
        }
        st_lib::update_multicon(
            c,
            refresh,
            [143, 168],
            &mut s.face_old,
            s.faceindex,
            &g.faces,
        )?;
        for i in 0..3 {
            let xy = [239, 171 + 10 * i as i32];
            st_lib::update_multicon(c, refresh, xy, &mut s.keys_old[i], s.keyboxes[i], &g.keys)?;
        }
    }
    Ok(StatusFrame {
        palette: s.palette as u8,
        palette_set,
        face: s.faceindex,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> StatusInputs {
        StatusInputs {
            health: 100,
            weaponowned: [true, true, false, false, false, false, false, false, false],
            readyweapon: 1,
            ..StatusInputs::default()
        }
    }

    /// st_stuff.c:735-739 (the table of §1.4.4).
    #[test]
    fn pain_offsets() {
        for (h, o) in [
            (200, 0),
            (80, 0),
            (79, 8),
            (60, 8),
            (59, 16),
            (40, 16),
            (39, 24),
        ] {
            assert_eq!(pain_offset(h), o, "{h}");
        }
        for (h, o) in [(20, 24), (19, 32), (0, 32)] {
            assert_eq!(pain_offset(h), o, "{h}");
        }
    }

    /// st_stuff.c:1006-1043 and the table of §1.4.10, including the berserk term the rooms never
    /// reach (decision S1) and the radiation bit 3.
    #[test]
    fn palette_table() {
        let pal = |d: i32, s: i32, b: i32, f: i32| {
            let mut i = inputs();
            (i.damagecount, i.powers[1], i.bonuscount, i.powers[3]) = (d, s, b, f);
            palette_for(&i)
        };
        let red = [
            (1, 2),
            (8, 2),
            (9, 3),
            (16, 3),
            (17, 4),
            (41, 7),
            (48, 7),
            (49, 8),
            (100, 8),
        ];
        for (d, want) in red {
            assert_eq!(pal(d, 0, 0, 0), want, "damage {d}");
        }
        // Berserk: bzc = 12 - (strength >> 6) beats a smaller damagecount.
        for (s, want) in [(1, 3), (255, 3), (256, 2), (767, 2), (768, 0), (100_000, 0)] {
            assert_eq!(pal(0, s, 0, 0), want, "strength {s}");
        }
        assert_eq!(pal(5, 1, 0, 0), 3);
        assert_eq!(pal(20, 1, 0, 0), 4);
        assert_eq!(pal(0, 768, 5, 0), 10);
        for (b, want) in [(1, 10), (8, 10), (9, 11), (16, 11), (17, 12), (100, 12)] {
            assert_eq!(pal(0, 0, b, 0), want, "bonus {b}");
        }
        assert_eq!(pal(1, 0, 1, 200), 2);
        assert_eq!(pal(0, 0, 1, 200), 10);
        for (f, want) in [
            (129, 13),
            (128, 0),
            (127, 13),
            (120, 13),
            (119, 0),
            (8, 13),
            (7, 0),
        ] {
            assert_eq!(pal(0, 0, 0, f), want, "ironfeet {f}");
        }
        assert_eq!(pal(0, 0, 0, 0), 0);
    }

    fn tick(s: &mut StatusBarState, p: &StatusInputs) {
        s.tick(p).expect("tick");
    }

    /// The god face (st_stuff.c:897-910), which wire 24 cannot reach (decision S1): priority 4,
    /// redrawn every tic, beaten by the rampage face (priority 5) but not by the look-around.
    #[test]
    fn the_god_face_and_its_priority() {
        let mut s = StatusBarState::level_start(inputs().weaponowned);
        let mut p = inputs();
        p.godmode = true;
        tick(&mut s, &p);
        assert_eq!((s.faceindex, s.priority, s.facecount), (GODFACE, 4, 0));
        p.godmode = false;
        p.powers[0] = 30;
        tick(&mut s, &p);
        assert_eq!(s.faceindex, GODFACE);
        p.attackdown = true;
        tick(&mut s, &p);
        assert_eq!(s.lastattackdown, RAMPAGEDELAY);
        for _ in 0..RAMPAGEDELAY - 1 {
            tick(&mut s, &p);
            assert_eq!(s.faceindex, GODFACE);
        }
        tick(&mut s, &p);
        assert_eq!((s.faceindex, s.priority, s.lastattackdown), (7, 5, 1));
        tick(&mut s, &p);
        assert_eq!((s.faceindex, s.priority), (7, 5));
        // Released: the rampage face has timed out and god (`< 5`) does not take priority 5,
        // so the look-around runs once; the next tic is god again.
        p.attackdown = false;
        tick(&mut s, &p);
        assert_eq!((s.priority, s.lastattackdown, s.facecount), (0, -1, 16));
        tick(&mut s, &p);
        assert_eq!((s.faceindex, s.priority), (GODFACE, 4));
    }

    /// The reversed ouch test (st_stuff.c:807, 860): a health *gain* above 20 while hurt, with
    /// the turn count; index 37 (ouch at pain level 4) needs health 19 or less after such a gain,
    /// which a start of -1 or more cannot give (19 - (-1) = 20).
    #[test]
    fn the_ouch_face_and_its_unreachable_level_4() {
        let mut s = StatusBarState::level_start(inputs().weaponowned);
        let mut p = inputs();
        p.health = 50;
        tick(&mut s, &p);
        p.health = 71;
        p.damagecount = 3;
        tick(&mut s, &p);
        assert_eq!(
            (s.faceindex, s.priority, s.facecount),
            (8 + OUCHOFFSET, 7, 34)
        );
        // A gain of exactly 20 is not "much pain": the rampage face at priority 6.
        let mut s = StatusBarState::level_start(inputs().weaponowned);
        p.health = 50;
        p.damagecount = 0;
        tick(&mut s, &p);
        p.health = 70;
        p.damagecount = 3;
        tick(&mut s, &p);
        assert_eq!((s.faceindex, s.priority), (8 + RAMPAGEOFFSET, 6));
        let mut s = StatusBarState::level_start(inputs().weaponowned);
        p.health = 19;
        tick(&mut s, &p);
        assert_eq!(s.faceindex, 32 + RAMPAGEOFFSET);
    }

    /// The attacker branch: head-on below 45 degrees, turned right or left otherwise, with the
    /// unfolded `diffang` (st_stuff.c:819-850), and a removed attacker refused (S17).
    #[test]
    fn the_attacker_direction() {
        let at = |angle: u32, ax: i32, ay: i32| {
            let mut s = StatusBarState::level_start(inputs().weaponowned);
            let mut p = inputs();
            // First a tic without damage: at the level start `st_oldhealth` is -1, so any
            // damage would take the (reversed) ouch test.
            tick(&mut s, &p);
            (p.damagecount, p.angle, p.attacker) = (5, angle, Attacker::At([ax, ay]));
            tick(&mut s, &p);
            (s.faceindex, s.priority, s.facecount)
        };
        let f = 1 << 16;
        assert_eq!(at(0, 100 * f, 0), (7, 7, 34));
        assert_eq!(at(0, 0, 100 * f), (4, 7, 34));
        assert_eq!(at(0, 0, -100 * f), (3, 7, 34));
        assert_eq!(at(ANG180, 0, 100 * f), (3, 7, 34));
        assert_eq!(at(0x1fff_0000, 100 * f, 0), (7, 7, 34));
        assert_eq!(at(u32::MAX - 0x1000_0000, 100 * f, 0), (4, 7, 34));
        let mut s = StatusBarState::level_start(inputs().weaponowned);
        let before = s;
        let mut p = inputs();
        (p.damagecount, p.attacker) = (1, Attacker::Removed);
        assert_eq!(s.tick(&p), Err(Attacker::Removed));
        assert_eq!(s, before);
        p.damagecount = 0;
        tick(&mut s, &p);
        // Own damage: the stupidity branch, not the attacker branch.
        (p.damagecount, p.attacker) = (2, Attacker::Player);
        tick(&mut s, &p);
        assert_eq!((s.faceindex, s.priority), (RAMPAGEOFFSET, 6));
    }

    /// The evil grin compares all nine weapons only while `bonuscount` is set, and the dead face
    /// wins over everything (st_stuff.c:761-796); the key boxes prefer the skull
    /// (st_stuff.c:953-959); the ready ammunition follows `weaponinfo` (d_items.c).
    #[test]
    fn grin_dead_keys_and_ready_ammo() {
        let mut s = StatusBarState::level_start(inputs().weaponowned);
        let mut p = inputs();
        p.weaponowned[8] = true;
        tick(&mut s, &p);
        assert!(!s.oldweaponsowned[8]);
        p.bonuscount = 6;
        tick(&mut s, &p);
        assert_eq!((s.faceindex, s.priority, s.facecount), (6, 8, 69));
        p.health = 0;
        tick(&mut s, &p);
        assert_eq!((s.faceindex, s.priority, s.facecount), (DEADFACE, 9, 0));
        p.cards = [true, false, true, false, true, true];
        p.readyweapon = 7;
        tick(&mut s, &p);
        assert_eq!(s.keyboxes, [0, 4, 5]);
        assert_eq!(s.ready_ammo, None);
        for (w, a) in [
            (0, None),
            (2, Some(1)),
            (4, Some(3)),
            (5, Some(2)),
            (8, Some(1)),
        ] {
            p.readyweapon = w;
            tick(&mut s, &p);
            assert_eq!(s.ready_ammo, a, "weapon {w}");
        }
    }

    /// The look-around (st_stuff.c:912-920) every 17 tics, from `M_Random`'s table.
    #[test]
    fn the_look_around_uses_m_random() {
        let mut s = StatusBarState::level_start(inputs().weaponowned);
        let p = inputs();
        tick(&mut s, &p);
        let mut rng = DoomRng::default();
        let first = i32::from(rng.m_random());
        assert_eq!((s.faceindex, s.facecount, s.rndindex), (first % 3, 16, 1));
        for _ in 0..16 {
            tick(&mut s, &p);
        }
        assert_eq!(s.faceindex, first % 3);
        tick(&mut s, &p);
        let r18 = (0..17).map(|_| rng.m_random()).last().expect("draw");
        assert_eq!(s.faceindex, i32::from(r18) % 3);
        assert_eq!(s.rndindex, 18);
    }
}
