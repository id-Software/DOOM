//! The presentation halves of p_spec.c and p_switch.c that the renderer reads: the animated
//! flat and texture translations (`P_InitPicAnims`, `P_UpdateSpecials`), the scrolling
//! texture offset of line special 48, the switch list (`P_InitSwitchList`) and the switch
//! textures (`P_ChangeSwitchTexture`, the button restore of `P_UpdateSpecials`), derived from
//! the simulation's canonical line specials and button slots. The simulation does not keep
//! switch textures; a re-press of a running button cannot be derived and raises the
//! `SwitchParityUnknown` witness (decision R7).
use crate::r_data::{GameMode, ResourceError, Resources, name_text};

/// An `anim_t` (p_spec.c:60-68).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Anim {
    /// `istexture`.
    pub istexture: bool,
    /// `picnum`.
    pub picnum: usize,
    /// `basepic`.
    pub basepic: usize,
    /// `numpics`.
    pub numpics: usize,
    /// `speed`.
    pub speed: u32,
}

/// `animdefs` (p_spec.c:101-133): istexture, endname, startname, speed.
const ANIMDEFS: &[(bool, &[u8], &[u8], u32)] = &[
    (false, b"NUKAGE3", b"NUKAGE1", 8),
    (false, b"FWATER4", b"FWATER1", 8),
    (false, b"SWATER4", b"SWATER1", 8),
    (false, b"LAVA4", b"LAVA1", 8),
    (false, b"BLOOD3", b"BLOOD1", 8),
    (false, b"RROCK08", b"RROCK05", 8),
    (false, b"SLIME04", b"SLIME01", 8),
    (false, b"SLIME08", b"SLIME05", 8),
    (false, b"SLIME12", b"SLIME09", 8),
    (true, b"BLODGR4", b"BLODGR1", 8),
    (true, b"SLADRIP3", b"SLADRIP1", 8),
    (true, b"BLODRIP4", b"BLODRIP1", 8),
    (true, b"FIREWALL", b"FIREWALA", 8),
    (true, b"GSTFONT3", b"GSTFONT1", 8),
    (true, b"FIRELAVA", b"FIRELAV3", 8),
    (true, b"FIREMAG3", b"FIREMAG1", 8),
    (true, b"FIREBLU2", b"FIREBLU1", 8),
    (true, b"ROCKRED3", b"ROCKRED1", 8),
    (true, b"BFALL4", b"BFALL1", 8),
    (true, b"SFALL4", b"SFALL1", 8),
    (true, b"WFALL4", b"WFALL1", 8),
    (true, b"DBRAIN4", b"DBRAIN1", 8),
];

/// `P_InitPicAnims` (p_spec.c:148-183). A cycle reaching outside the texture or flat table
/// (an out-of-bounds translation write in the genuine engine) is refused.
pub(crate) fn init_pic_anims(res: &Resources) -> Result<Vec<Anim>, ResourceError> {
    let mut out = Vec::new();
    for &(istexture, end, start, speed) in ANIMDEFS {
        let (picnum, basepic, size) = if istexture {
            if res.check_texture_num_for_name(start).is_none() {
                continue;
            }
            (
                res.texture_num_for_name(end)?,
                res.texture_num_for_name(start)?,
                res.textures.len(),
            )
        } else {
            if res.directory().check_num_for_name(start).is_none() {
                continue;
            }
            (
                res.flat_num_for_name(end)?,
                res.flat_num_for_name(start)?,
                res.numflats,
            )
        };
        let bad =
            || ResourceError::BadAnimCycle(format!("{} to {}", name_text(start), name_text(end)));
        let numpics = (picnum as i64 - basepic as i64 + 1).max(0) as usize;
        if numpics < 2 || basepic + numpics > size {
            return Err(bad());
        }
        out.push(Anim {
            istexture,
            picnum,
            basepic,
            numpics,
            speed,
        });
    }
    Ok(out)
}

/// The flat and texture translation tables after the simulation's tic `tic`
/// (`Simulation::tic()`): identity before the first tic, then `P_UpdateSpecials`
/// (p_spec.c:1100-1110) with `leveltime = tic - 1`.
pub fn translations(res: &Resources, tic: u64) -> (Vec<usize>, Vec<usize>) {
    let mut flats: Vec<usize> = (0..res.numflats).collect();
    let mut textures: Vec<usize> = (0..res.textures.len()).collect();
    if tic > 0 {
        let leveltime = tic - 1;
        for a in &res.anims {
            for i in a.basepic..a.basepic + a.numpics {
                let pic = a.basepic
                    + ((leveltime / u64::from(a.speed) + i as u64) % a.numpics as u64) as usize;
                if a.istexture {
                    textures[i] = pic;
                } else {
                    flats[i] = pic;
                }
            }
        }
    }
    (flats, textures)
}

/// `alphSwitchList` (p_switch.c:50-96): name1, name2, episode.
const SWITCHES: &[(&[u8], &[u8], u8)] = &[
    (b"SW1BRCOM", b"SW2BRCOM", 1),
    (b"SW1BRN1", b"SW2BRN1", 1),
    (b"SW1BRN2", b"SW2BRN2", 1),
    (b"SW1BRNGN", b"SW2BRNGN", 1),
    (b"SW1BROWN", b"SW2BROWN", 1),
    (b"SW1COMM", b"SW2COMM", 1),
    (b"SW1COMP", b"SW2COMP", 1),
    (b"SW1DIRT", b"SW2DIRT", 1),
    (b"SW1EXIT", b"SW2EXIT", 1),
    (b"SW1GRAY", b"SW2GRAY", 1),
    (b"SW1GRAY1", b"SW2GRAY1", 1),
    (b"SW1METAL", b"SW2METAL", 1),
    (b"SW1PIPE", b"SW2PIPE", 1),
    (b"SW1SLAD", b"SW2SLAD", 1),
    (b"SW1STARG", b"SW2STARG", 1),
    (b"SW1STON1", b"SW2STON1", 1),
    (b"SW1STON2", b"SW2STON2", 1),
    (b"SW1STONE", b"SW2STONE", 1),
    (b"SW1STRTN", b"SW2STRTN", 1),
    (b"SW1BLUE", b"SW2BLUE", 2),
    (b"SW1CMT", b"SW2CMT", 2),
    (b"SW1GARG", b"SW2GARG", 2),
    (b"SW1GSTON", b"SW2GSTON", 2),
    (b"SW1HOT", b"SW2HOT", 2),
    (b"SW1LION", b"SW2LION", 2),
    (b"SW1SATYR", b"SW2SATYR", 2),
    (b"SW1SKIN", b"SW2SKIN", 2),
    (b"SW1VINE", b"SW2VINE", 2),
    (b"SW1WOOD", b"SW2WOOD", 2),
    (b"SW1PANEL", b"SW2PANEL", 3),
    (b"SW1ROCK", b"SW2ROCK", 3),
    (b"SW1MET2", b"SW2MET2", 3),
    (b"SW1WDMET", b"SW2WDMET", 3),
    (b"SW1BRIK", b"SW2BRIK", 3),
    (b"SW1MOD1", b"SW2MOD1", 3),
    (b"SW1ZIM", b"SW2ZIM", 3),
    (b"SW1STON6", b"SW2STON6", 3),
    (b"SW1TEK", b"SW2TEK", 3),
    (b"SW1MARB", b"SW2MARB", 3),
    (b"SW1SKULL", b"SW2SKULL", 3),
];

/// `P_InitSwitchList` (p_switch.c:107-140): episode 2 for registered, 1 otherwise (commercial
/// is not admitted).
pub(crate) fn init_switch_list(
    res: &Resources,
    mode: GameMode,
) -> Result<Vec<usize>, ResourceError> {
    let episode = if mode == GameMode::Registered { 2 } else { 1 };
    let mut out = Vec::new();
    for &(a, b, e) in SWITCHES {
        if e <= episode {
            out.push(res.texture_num_for_name(a)?);
            out.push(res.texture_num_for_name(b)?);
        }
    }
    Ok(out)
}

/// Line specials whose action calls `P_ChangeSwitchTexture(line, 0)` (p_switch.c:350-510,
/// the S1 cases; p_spec.c:985 and 997, the G1 cases 24 and 47): the only way such a line's
/// runtime special becomes 0.
pub const NONREUSABLE_SWITCH_SPECIALS: &[i16] = &[
    7, 9, 11, 14, 15, 18, 20, 21, 23, 24, 29, 41, 47, 49, 50, 51, 55, 71, 101, 102, 103, 111, 112,
    113, 122, 127, 131, 133, 135, 137, 140,
];

/// `sfx_swtchn` (sounds.h:202, the 24th `sfxenum_t`).
pub const SFX_SWTCHN: u16 = 23;

/// The texture part a switch change replaced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwitchPart {
    /// `top`.
    Top,
    /// `middle`.
    Middle,
    /// `bottom`.
    Bottom,
}

/// The `P_ChangeSwitchTexture` search (p_switch.c:224-262) over side textures
/// `[top, mid, bottom]`: for each `switchlist` index in order, top, then middle, then bottom;
/// the first match is replaced by `switchlist[i ^ 1]`.
pub fn switch_flip(switchlist: &[usize], tex: [usize; 3]) -> Option<(SwitchPart, usize)> {
    for (i, s) in switchlist.iter().enumerate() {
        let to = switchlist[i ^ 1];
        if *s == tex[0] {
            return Some((SwitchPart::Top, to));
        }
        if *s == tex[1] {
            return Some((SwitchPart::Middle, to));
        }
        if *s == tex[2] {
            return Some((SwitchPart::Bottom, to));
        }
    }
    None
}

/// One `buttonlist` slot as the canonical state gives it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ButtonSlot {
    /// `btimer` (0 for a free slot).
    pub timer: i32,
    /// The line of a running button.
    pub line: u16,
}

/// The switch history the renderer carries between tics (decision R7): the previous tic's
/// button slots and cleared non-reusable switch lines, and the lines whose texture parity is
/// unknown since an unexplained switch sound.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SwitchTracker {
    /// The 16 slots after the last observed tic.
    pub buttons: [ButtonSlot; 16],
    /// Non-reusable switch lines (with a switch texture) whose runtime special was 0 after
    /// the last observed tic, ascending.
    pub cleared: Vec<u16>,
    /// Lines whose texture parity is unknown, ascending; each leaves the set when its button
    /// expires (the restore to `btexture` re-synchronises, p_spec.c:1127-1153).
    pub unknown: Vec<u16>,
}

/// The canonical runtime facts the switch tracker reads after a tic.
#[derive(Clone, Debug, Default)]
pub struct SwitchInputs {
    /// `button.{k}` slots.
    pub buttons: [ButtonSlot; 16],
    /// Non-reusable switch lines (with a switch texture) whose runtime special is 0.
    pub cleared: Vec<u16>,
    /// `sfx_swtchn` requests in the tic's events.
    pub sounds: u32,
}

/// The switch tracker found more switch changes than switch sounds: its model of the
/// simulation's switch events is wrong (an internal inconsistency, never silently ignored).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SwitchInconsistency;

impl SwitchTracker {
    /// The next tracker after one tic. Every `P_ChangeSwitchTexture` that found its texture
    /// and every button expiry makes one `sfx_swtchn` request: newly cleared non-reusable
    /// lines, new buttons and expiries (slots with `btimer` 1 before the tic) explain the
    /// sounds; any further sound is a re-press of a running button, whose line is unknown,
    /// so every line holding a button in this tic becomes parity-unknown.
    pub fn next(&self, now: &SwitchInputs) -> Result<Self, SwitchInconsistency> {
        let new_clears = now
            .cleared
            .iter()
            .filter(|l| self.cleared.binary_search(l).is_err())
            .count() as u32;
        let new_buttons = now
            .buttons
            .iter()
            .zip(&self.buttons)
            .filter(|(n, p)| n.timer != 0 && p.timer == 0)
            .count() as u32;
        let expiries = self.buttons.iter().filter(|p| p.timer == 1).count() as u32;
        let explained = new_clears + new_buttons + expiries;
        if explained > now.sounds {
            return Err(SwitchInconsistency);
        }
        let mut unknown = self.unknown.clone();
        if now.sounds > explained {
            for s in self.buttons.iter().chain(now.buttons.iter()) {
                if s.timer != 0 {
                    unknown.push(s.line);
                }
            }
        }
        unknown.sort_unstable();
        unknown.dedup();
        unknown.retain(|l| now.buttons.iter().any(|b| b.timer != 0 && b.line == *l));
        Ok(Self {
            buttons: now.buttons,
            cleared: now.cleared.clone(),
            unknown,
        })
    }
    /// Whether some switch texture's parity is unknown (frames are then non-comparable,
    /// decision R7).
    pub fn parity_unknown(&self) -> bool {
        !self.unknown.is_empty()
    }
}

/// The side textures `[top, mid, bottom]` of every sidedef after the tracked switch changes:
/// the map textures, with each cleared non-reusable switch line's front side and each running
/// button's line front side flipped by [`switch_flip`] (valid while the parity is known).
pub fn side_textures(
    res: &Resources,
    map_sides: &[[usize; 3]],
    lines: &[doom_map::LineDef],
    tracker: &SwitchTracker,
) -> Vec<[usize; 3]> {
    let mut sides = map_sides.to_vec();
    let flipped = tracker.cleared.iter().copied().chain(
        tracker
            .buttons
            .iter()
            .filter(|b| b.timer != 0)
            .map(|b| b.line),
    );
    for line in flipped {
        let Some(front) = lines.get(usize::from(line)).and_then(|l| l.sides[0]) else {
            continue;
        };
        let s = &mut sides[usize::from(front)];
        if let Some((part, to)) = switch_flip(&res.switchlist, *s) {
            match part {
                SwitchPart::Top => s[0] = to,
                SwitchPart::Middle => s[1] = to,
                SwitchPart::Bottom => s[2] = to,
            }
        }
    }
    sides
}

/// The `textureoffset` of every sidedef after the simulation's tic `tic`: the map offset
/// plus, for every line of the special-48 list (p_spec.c:1336-1345, at most 64), FRACUNIT per
/// tic on its front side (p_spec.c:1113-1124), wrapping.
pub fn texture_offsets(map: &doom_map::Map, tic: u64) -> Vec<i32> {
    let mut per_side = vec![0i32; map.sidedefs().len()];
    for line in map.linedefs().iter().filter(|l| l.special == 48) {
        if let Some(front) = line.sides[0] {
            per_side[usize::from(front)] += 1;
        }
    }
    map.sidedefs()
        .iter()
        .zip(per_side)
        .map(|(s, k)| {
            (i32::from(s.offsets[0]) << 16)
                .wrapping_add((tic as i32).wrapping_mul(k).wrapping_mul(1 << 16))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_search_order_is_index_then_part() {
        let list = [10, 11, 20, 21];
        // Index 0 (10) matches the bottom before index 2 (20) matches the top.
        assert_eq!(
            switch_flip(&list, [20, 0, 10]),
            Some((SwitchPart::Bottom, 11))
        );
        assert_eq!(switch_flip(&list, [21, 0, 0]), Some((SwitchPart::Top, 20)));
        assert_eq!(switch_flip(&list, [1, 2, 3]), None);
    }

    #[test]
    fn a_re_press_makes_the_parity_unknown_until_the_button_expires() {
        let t = SwitchTracker::default();
        let mut now = SwitchInputs::default();
        now.buttons[0] = ButtonSlot { timer: 35, line: 4 };
        now.sounds = 1;
        let t = t.next(&now).unwrap();
        assert!(!t.parity_unknown());
        now.buttons[0].timer = 34;
        now.sounds = 1; // a re-press: no new button
        let t = t.next(&now).unwrap();
        assert_eq!(t.unknown, vec![4]);
        now.buttons[0] = ButtonSlot::default();
        now.sounds = 1; // the expiry
        let mut t2 = t.clone();
        t2.buttons[0].timer = 1;
        let t3 = t2.next(&now).unwrap();
        assert!(!t3.parity_unknown());
        let mut lying = SwitchInputs::default();
        lying.buttons[1] = ButtonSlot { timer: 35, line: 9 };
        assert_eq!(
            SwitchTracker::default().next(&lying),
            Err(SwitchInconsistency)
        );
    }

    use crate::r_data::{LumpDirectory, Texture};

    /// A `Resources` whose flat range holds `flats` (lumps `F_START`, the flats, `F_END`) and
    /// whose textures are named `textures`; the anim and switch initialisers read only these.
    fn resources(flats: &[&str], textures: &[&str]) -> Resources {
        let mut dir = LumpDirectory::new();
        dir.add_single_lump("F_START", &[]).unwrap();
        for f in flats {
            dir.add_single_lump(f, &[0; 4096]).unwrap();
        }
        dir.add_single_lump("F_END", &[]).unwrap();
        Resources {
            dir,
            colormaps: vec![],
            palette: None,
            textures: textures
                .iter()
                .map(|n| {
                    let mut name = [0u8; 8];
                    name[..n.len()].copy_from_slice(n.as_bytes());
                    Texture {
                        name,
                        width: 0,
                        height: 0,
                        patches: vec![],
                        widthmask: 0,
                        textureheight: 0,
                        columns: vec![],
                        compositesize: 0,
                        composite: vec![],
                        defined: vec![],
                    }
                })
                .collect(),
            firstflat: 1,
            numflats: flats.len(),
            firstspritelump: 0,
            sprite_lumps: vec![],
            sprites: vec![],
            switchlist: vec![],
            anims: vec![],
            skytexture: 0,
            skyflatnum: None,
        }
    }

    #[test]
    fn an_animation_cycle_needs_two_pictures_and_may_end_the_flat_table() {
        // P_InitPicAnims (p_spec.c:148-183): `numpics = picnum - basepic + 1`; fewer than 2
        // is `I_Error("P_InitPicAnims: bad cycle from %s to %s")`. The first animdef is
        // NUKAGE1..NUKAGE3 at speed 8 (p_spec.c:101-103). Every other animdef's start name is
        // absent and is skipped (`W_CheckNumForName(startname) == -1`, p_spec.c:163-164).
        let anim = |flats: &[&str]| init_pic_anims(&resources(flats, &[]));
        // Three pictures filling the whole table: accepted, with the cycle's last picture
        // the table's last flat.
        assert_eq!(
            anim(&["NUKAGE1", "NUKAGE2", "NUKAGE3"]),
            Ok(vec![Anim {
                istexture: false,
                picnum: 2,
                basepic: 0,
                numpics: 3,
                speed: 8
            }])
        );
        // The same cycle after two unrelated flats (basepic 2, numpics 3, table of 5).
        assert_eq!(
            anim(&["A", "B", "NUKAGE1", "NUKAGE2", "NUKAGE3"]),
            Ok(vec![Anim {
                istexture: false,
                picnum: 4,
                basepic: 2,
                numpics: 3,
                speed: 8
            }])
        );
        // Exactly two pictures (NUKAGE3 directly after NUKAGE1) is a cycle.
        let two = anim(&["NUKAGE1", "NUKAGE3"]).unwrap();
        assert_eq!((two[0].basepic, two[0].picnum, two[0].numpics), (0, 1, 2));
        // The end before the start (numpics 0 and below) is the genuine bad cycle.
        let bad = ResourceError::BadAnimCycle("NUKAGE1 to NUKAGE3".into());
        assert_eq!(anim(&["NUKAGE3", "NUKAGE1"]), Err(bad.clone()));
        assert_eq!(anim(&["NUKAGE3", "X", "NUKAGE1"]), Err(bad));
        // No animated flat present: no animation.
        assert_eq!(anim(&["A", "B"]), Ok(vec![]));
    }

    #[test]
    fn the_switch_list_follows_the_game_mode() {
        // P_InitSwitchList (p_switch.c:107-140): episode 1 by default, 2 for registered; the
        // table holds 19 episode-1, 10 episode-2 and 11 episode-3 pairs (p_switch.c:50-96),
        // and the list holds both textures of every pair whose episode is not above the game's
        // (retail is not commercial, so it keeps episode 1 as in the C else-branch).
        let names: Vec<String> = SWITCHES
            .iter()
            .flat_map(|(a, b, _)| [a, b])
            .map(|n| String::from_utf8(n.to_vec()).unwrap())
            .collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let res = resources(&[], &refs);
        assert_eq!(SWITCHES.iter().filter(|s| s.2 == 1).count(), 19);
        assert_eq!(SWITCHES.iter().filter(|s| s.2 == 2).count(), 10);
        let list = |m| init_switch_list(&res, m).unwrap();
        assert_eq!(list(GameMode::Shareware).len(), 2 * 19);
        assert_eq!(list(GameMode::Retail).len(), 2 * 19);
        assert_eq!(list(GameMode::Registered).len(), 2 * (19 + 10));
        // The pairs keep the table's order: SW1BRCOM (texture 0) then SW2BRCOM (texture 1).
        assert_eq!(&list(GameMode::Registered)[..4], &[0, 1, 2, 3]);
    }

    #[test]
    fn a_vanished_button_line_leaves_the_unknown_set_even_while_other_buttons_run() {
        // The expiry of the button on line 4 (btimer 1) re-synchronises that line
        // (p_spec.c:1127-1153); a different line's new button must not keep it unknown.
        let mut t = SwitchTracker::default();
        t.buttons[0] = ButtonSlot { timer: 1, line: 4 };
        t.unknown = vec![4];
        let mut now = SwitchInputs::default();
        now.buttons[1] = ButtonSlot { timer: 35, line: 9 };
        now.sounds = 2; // the expiry and the new button
        let t = t.next(&now).unwrap();
        assert!(t.unknown.is_empty(), "{:?}", t.unknown);
        // A line still holding a running button stays unknown while its button runs.
        let mut t = SwitchTracker::default();
        t.buttons[0] = ButtonSlot { timer: 20, line: 4 };
        t.unknown = vec![4];
        let mut now = SwitchInputs::default();
        now.buttons[0] = ButtonSlot { timer: 19, line: 4 };
        now.sounds = 0;
        assert_eq!(t.next(&now).unwrap().unknown, vec![4]);
    }
}
