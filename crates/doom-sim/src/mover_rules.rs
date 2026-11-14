//! Load rules of the movers policy (M3h8 note §5–§7): admitted line specials, the
//! mover families, neighbours, the reachable-height fixpoint, counts, the `spechit`
//! window, sky movers, switch textures (rule 9) and defined mover blockboxes (rule
//! 10), all computed once from the static map in exact integers.
//! Every failure is a field-less error and no state is built.
use crate::{MapScaleError, PursuitError, SimError, player_map_scale::ML_TWOSIDED};
use doom_map::Map;

/// p_switch.c `P_UseSpecialLine` / p_doors.c `EV_VerticalDoor`: DR door (use).
pub(crate) const LINE_DOOR_DR: i16 = 1;
/// SR door, `EV_DoDoor(normal)` on the tagged sectors (use, repeatable switch).
pub(crate) const LINE_DOOR_SR: i16 = 63;
/// W1 door, `EV_DoDoor(blazeOpen)` on the tagged sectors (cross).
pub(crate) const LINE_DOOR_BLAZE: i16 = 109;
/// WR lift, `EV_DoPlat(downWaitUpStay)` on the tagged sectors (cross).
pub(crate) const LINE_LIFT: i16 = 88;
/// W1 floor, `EV_DoFloor(turboLower)` on the tagged sectors (cross).
pub(crate) const LINE_FLOOR_TURBO: i16 = 36;
/// W1 floor, `EV_DoFloor(lowerFloorToLowest)` on the tagged sectors (cross).
pub(crate) const LINE_FLOOR_LOWEST: i16 = 38;
/// S1 exit switch, `G_ExitLevel` (use).
pub(crate) const LINE_EXIT: i16 = 11;
/// p_spec.h `MAXPLATS`: `activeplats` slots (`P_AddActivePlat` `I_Error`s past it).
pub(crate) const MAX_PLATS: usize = 30;
/// p_spec.h `MAXBUTTONS`: `buttonlist` slots (`P_StartButton` `I_Error`s past it).
pub(crate) const MAX_BUTTONS: usize = 16;
/// p_map.c `MAXSPECIALCROSS`: `spechit` entries one `P_CheckPosition` may store.
pub(crate) const MAX_SPECIAL_CROSS: usize = 8;
/// The reachable-height bound of rule 4 (map units), the M3h4 vertical arithmetic's.
pub(crate) const REACHABLE_UNITS: i32 = 512;

/// The mover family of a sector (rule 2): what its `specialdata` can hold.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Family {
    /// Back sector of a DR line or target of 63/109 (`vldoor_t`).
    Door,
    /// Target of 88 (`plat_t`).
    Plat,
    /// Target of 36/38 (`floormove_t`).
    Floor,
}

/// Which admitted kinds act on one sector (a sector may be a door of several kinds,
/// decision N5).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Kinds {
    /// `normal` door by use (DR 1, 26–28).
    dr: bool,
    /// `normal` door by tag (SR 63, WR 90).
    sr: bool,
    blaze: bool,
    /// `downWaitUpStay` (WR 88, SR 62).
    lift: bool,
    /// `turboLower` (W1 36, SR 70, WR 98).
    turbo: bool,
    /// `lowerFloorToLowest` (W1 38, S1 23, WR 82).
    lowest: bool,
    /// `open` door (W1 2, WR 86, S1 103, GR 46 by tag; D1 31–34 by use).
    open: bool,
    /// `close30ThenOpen` door (W1 16, WR 76).
    close30: bool,
    /// `raiseFloor` (W1 5, WR 91).
    raise: bool,
    /// `raiseFloorToNearest` (S1 18).
    nearest: bool,
    /// `raiseToNearestAndChange` plat (S1 20, W1 22).
    change: bool,
}
impl Kinds {
    fn door(self) -> bool {
        self.dr || self.sr || self.blaze || self.open || self.close30
    }
    fn families(self) -> impl Iterator<Item = Family> {
        [
            self.door().then_some(Family::Door),
            (self.lift || self.change).then_some(Family::Plat),
            (self.turbo || self.lowest || self.raise || self.nearest).then_some(Family::Floor),
        ]
        .into_iter()
        .flatten()
    }
}

/// The episode-1 entries of `alphSwitchList` (p_switch.c:51–69), both columns, in
/// table order: the first 38 `switchlist` slots `P_InitSwitchList` builds in every
/// game mode (p_switch.c:107–148 keeps `episode <= 1, 2 or 3` in table order).
pub(crate) const EPISODE1_SWITCHES: [[&[u8]; 2]; 19] = [
    [b"SW1BRCOM", b"SW2BRCOM"],
    [b"SW1BRN1", b"SW2BRN1"],
    [b"SW1BRN2", b"SW2BRN2"],
    [b"SW1BRNGN", b"SW2BRNGN"],
    [b"SW1BROWN", b"SW2BROWN"],
    [b"SW1COMM", b"SW2COMM"],
    [b"SW1COMP", b"SW2COMP"],
    [b"SW1DIRT", b"SW2DIRT"],
    [b"SW1EXIT", b"SW2EXIT"],
    [b"SW1GRAY", b"SW2GRAY"],
    [b"SW1GRAY1", b"SW2GRAY1"],
    [b"SW1METAL", b"SW2METAL"],
    [b"SW1PIPE", b"SW2PIPE"],
    [b"SW1SLAD", b"SW2SLAD"],
    [b"SW1STARG", b"SW2STARG"],
    [b"SW1STON1", b"SW2STON1"],
    [b"SW1STON2", b"SW2STON2"],
    [b"SW1STONE", b"SW2STONE"],
    [b"SW1STRTN", b"SW2STRTN"],
];
/// The registered-episode (2 and 3) and commercial entries of `alphSwitchList`
/// (p_switch.c:71–95), first columns then second, in table order. Under the
/// Episode 1 policy a switch line whose front side matches one of these and no
/// episode-1 entry is refused (decision D3): which of them `P_InitSwitchList`
/// keeps depends on the game mode (p_switch.c:107–148).
pub(crate) const LATER_EPISODE_SWITCHES: [[&[u8]; 2]; 21] = [
    [b"SW1BLUE", b"SW2BLUE"],
    [b"SW1CMT", b"SW2CMT"],
    [b"SW1GARG", b"SW2GARG"],
    [b"SW1GSTON", b"SW2GSTON"],
    [b"SW1HOT", b"SW2HOT"],
    [b"SW1LION", b"SW2LION"],
    [b"SW1SATYR", b"SW2SATYR"],
    [b"SW1SKIN", b"SW2SKIN"],
    [b"SW1VINE", b"SW2VINE"],
    [b"SW1WOOD", b"SW2WOOD"],
    [b"SW1PANEL", b"SW2PANEL"],
    [b"SW1ROCK", b"SW2ROCK"],
    [b"SW1MET2", b"SW2MET2"],
    [b"SW1WDMET", b"SW2WDMET"],
    [b"SW1BRIK", b"SW2BRIK"],
    [b"SW1MOD1", b"SW2MOD1"],
    [b"SW1ZIM", b"SW2ZIM"],
    [b"SW1STON6", b"SW2STON6"],
    [b"SW1TEK", b"SW2TEK"],
    [b"SW1MARB", b"SW2MARB"],
    [b"SW1SKULL", b"SW2SKULL"],
];
/// The line specials whose `P_UseSpecialLine` or `P_ShootSpecialLine` arm calls
/// `P_ChangeSwitchTexture` (p_switch.c:276–653, p_spec.c:959–1000) among the
/// specials the Episode 1 policy admits or will admit (decision H1): the S1, SR
/// and G1-style switch lines. Rule 9 (D3) reads the front texture of each.
pub(crate) const EPISODE1_SWITCH_SPECIALS: [i16; 12] = [
    LINE_EXIT,
    LINE_DOOR_SR,
    7,
    9,
    18,
    20,
    23,
    46,
    51,
    62,
    70,
    103,
];
/// Whether a sidedef texture name resolves to the same texture as `candidate`:
/// `R_CheckTextureNumForName` (r_data.c:696–710) maps a name starting with `-` to
/// texture 0 and otherwise compares with `strncasecmp(.., 8)` (ASCII case ignored,
/// stopping at the first NUL or after eight characters).
fn same_texture(name: &doom_types::LumpName, candidate: &[u8]) -> bool {
    let name = name.as_bytes();
    if name[0] == b'-' {
        return false;
    }
    for (k, &a) in name.iter().enumerate() {
        let b = candidate.get(k).copied().unwrap_or(0);
        if !a.eq_ignore_ascii_case(&b) {
            return false;
        }
        if a == 0 {
            return true;
        }
    }
    true
}
/// The `switchlist` slot `P_ChangeSwitchTexture` (p_switch.c:224–262) matches for a
/// front side's `[upper, lower, middle]` textures, restricted to the episode-1
/// prefix: slots in index order, each tested against the top, then the middle,
/// then the bottom texture. `Some((slot, part))` with `part` 0 top, 1 middle,
/// 2 bottom (`bwhere_e`). The first match is unique and the same in every game
/// mode, since the episode-1 slots precede every other.
pub(crate) fn switch_match(textures: &[doom_types::LumpName; 3]) -> Option<(usize, u8)> {
    let [upper, lower, middle] = textures;
    EPISODE1_SWITCHES
        .iter()
        .flatten()
        .enumerate()
        .find_map(|(slot, name)| {
            [(upper, 0), (middle, 1), (lower, 2)]
                .into_iter()
                .find(|(texture, _)| same_texture(texture, name))
                .map(|(_, part)| (slot, part))
        })
}

/// Whether a front side's textures match a registered-episode or commercial
/// switch entry ([`LATER_EPISODE_SWITCHES`]), in either column.
fn later_episode_switch(textures: &[doom_types::LumpName; 3]) -> bool {
    LATER_EPISODE_SWITCHES
        .iter()
        .flatten()
        .any(|name| textures.iter().any(|t| same_texture(t, name)))
}
/// The switch slot `P_ChangeSwitchTexture` finds for `line`'s front side under the
/// Episode 1 policy (decision D3): `None` when its front textures match no
/// episode-1 entry, which the source handles as a texture search that finds
/// nothing (no texture change, no sound, no button; a non-reusable line's
/// special is still cleared, p_switch.c:211–263). The load rule has already
/// refused a front side that matches only a later-episode entry.
pub(crate) fn line_switch_match(map: &Map, line: usize) -> Option<(usize, u8)> {
    let side = map.linedefs().get(line)?.sides[0]?;
    switch_match(&map.sidedefs().get(usize::from(side))?.textures)
}

/// What the movers admission derived from the loaded map; immutable, never
/// canonical state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MoverAdmission {
    /// Every sector's family, `None` for a sector no admitted line moves.
    pub family: Vec<Option<Family>>,
    /// The number of mover sectors `M` (sectors with a family).
    pub movers: usize,
    /// The smallest and largest reachable height `R_min`, `R_max` (map units).
    pub reach: [i32; 2],
    /// The largest one-tic sink of a fitting thing, `Δ` (§5, map units).
    pub delta: i32,
}

/// The bounds the movers admission derives from a map (M3h8 note §5), public so
/// that external checks (the fuzz target) can evaluate the vertical invariant V′
/// (note §4.1) on their own: `reach` is `[R_min, R_max]` and `delta` is `Δ`, the
/// largest one-tic sink of a fitting thing, both in map units; `movers` is the
/// number of mover sectors `M`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoverBounds {
    /// The smallest and largest reachable floor or ceiling height.
    pub reach: [i32; 2],
    /// The largest one-tic sink of a fitting thing.
    pub delta: i32,
    /// The number of sectors an admitted line can move.
    pub movers: usize,
}
/// Runs the movers policy's line and sector load rules on `map` (the Stage A sector
/// special and light rules, then M3h8 note §7: the function
/// `Simulation::original_player_movers_map` runs after its geometry rules) and
/// returns the derived bounds, or the field-less error of the first failing rule.
/// Admission only; no simulation is built, and the geometry rules are not run.
pub fn mover_bounds(map: &Map) -> Result<MoverBounds, SimError> {
    let admission = validate_movers(map)?;
    Ok(MoverBounds {
        reach: admission.reach,
        delta: admission.delta,
        movers: admission.movers,
    })
}
/// [`mover_bounds`] under the E1M1 policy's load rules: the same rules with a
/// two-sided scroller (line special 48, blocking or not) admitted, as
/// `Simulation::original_player_e1m1_map` runs them. Admission only.
pub fn e1m1_mover_bounds(map: &Map) -> Result<MoverBounds, SimError> {
    let admission = validate_e1m1(map)?;
    Ok(MoverBounds {
        reach: admission.reach,
        delta: admission.delta,
        movers: admission.movers,
    })
}

/// `getNextSector` (p_spec.c:251–261) of `line` seen from `sector`: `None` without
/// `ML_TWOSIDED`; the back sector when `sector` is the front (itself for a line with
/// the sector on both sides), else the front.
pub(crate) fn next_sector(map: &Map, line: usize, sector: usize) -> Option<usize> {
    let line = &map.linedefs()[line];
    if line.flags & ML_TWOSIDED == 0 {
        return None;
    }
    let side = |i: usize| line.sides[i].map(|s| usize::from(map.sidedefs()[usize::from(s)].sector));
    if side(0) == Some(sector) {
        side(1)
    } else {
        side(0)
    }
}
/// The sectors `P_FindSectorFromLineTag` visits for `tag`, in index order (tag 0
/// selects every untagged sector, faithfully).
pub(crate) fn tagged(map: &Map, tag: i16) -> impl Iterator<Item = usize> + '_ {
    map.sectors()
        .iter()
        .enumerate()
        .filter(move |(_, s)| s.tag == tag)
        .map(|(i, _)| i)
}

/// The movers policy's load rules (M3h8 note §7), after the Stage A sector rules:
///
/// 1. every line special is 0, a one-sided 48 (a two-sided 48 stays rejected,
///    decision N1; the E1M1 policy admits it through [`validate_e1m1`]) or one of
///    1, 63, 109, 88, 36, 38, 11 — exactly E1M1's tag, use and cross types, M3h8
///    note §4.0 — (`PursuitError::Specials`),
///    and every DR line has a back side ([`MapScaleError::DoorLine`]);
/// 2. no sector is in two mover families ([`MapScaleError::MoverFamily`]);
/// 3. every mover sector has a `getNextSector` neighbour
///    ([`MapScaleError::MoverNeighbor`]);
/// 4. the §5 reachable-height fixpoint, with the W1 rules run to a global fixpoint
///    like the repeatable ones (decision N7), stays within ±512
///    ([`MapScaleError::MoverHeights`]); no door's smallest possible top is below
///    its floor ([`MapScaleError::InvertingDoor`]); no door sector has a
///    self-referencing two-sided line ([`MapScaleError::SelfReferencingDoor`]);
/// 5. at most 30 plat sectors and 16 reusable switch lines
///    ([`MapScaleError::MoverCount`]), at most 64 scrollers (`PursuitError::Specials`,
///    Stage A);
/// 6. at most 8 distinct two-sided special lines in any 2 × 2 window of the map's
///    own BLOCKMAP cells ([`MapScaleError::SpecialWindow`]);
/// 7. no crushers (none is in rule 1's set);
/// 8. no mover sector has an `F_SKY1` ceiling ([`MapScaleError::SkyMover`],
///    decision N4);
/// 9. every switch line (11, 63) has an episode-1 switch texture on its front side
///    ([`MapScaleError::SwitchTexture`], decision N9a, [`switch_match`]).
/// 10. every mover sector has a defined `blockbox` (the `P_GroupLines` arithmetic of
///     `multisector::sector_block_boxes` stays inside `fixed_t`), so every
///     `P_ChangeSector` of a plane step visits defined cells
///     ([`MapScaleError::MoverBlockBox`]).
pub(crate) fn validate_movers(map: &Map) -> Result<MoverAdmission, SimError> {
    validate_movers_with(map, Rules::Movers)
}
/// The E1M1 policy's load rules (M3h9 note §5.1, decision E1): every rule of
/// [`validate_movers`] with rule 1 relaxed for line special 48 on a two-sided line,
/// blocking (`ML_BLOCKING`) or not. `P_SpawnSpecials` lists every 48 line in
/// `linespeciallist` whatever its sides (p_spec.c:1336–1346) and `P_UpdateSpecials`
/// only adds `FRACUNIT` to `sides[line->sidenum[0]].textureoffset` each tic
/// (1114–1124), a field no game code reads; the game-code effects of the line are
/// the M3h8 bodies: it enters `spechit` from `PIT_CheckLine` when it does not block
/// (p_map.c:240–244), a monster's failed move resets `movedir` and
/// `P_UseSpecialLine(actor, line, 0)` returns false (p_enemy.c:310–324,
/// p_switch.c:301–319), a player's use reaches the trailing `return true` with no
/// effect and `PTR_UseTraverse` stops at it (p_map.c:1113–1122), and
/// `P_CrossSpecialLine` and `P_ShootSpecialLine` have no case 48. A blocking
/// two-sided 48 blocks every non-missile like any blocking line and never enters its
/// `spechit` (`PIT_CheckLine` returns before the special test); a missile skips the
/// `ML_BLOCKING` test (p_map.c:214–221), so the line enters the missile's `spechit`
/// (240–244) and its crossing calls `P_CrossSpecialLine`, which returns at once for
/// a missile (p_spec.c:503–514; review scenario
/// `fireball_through_blocking_scroller`). Rule 5 (at most 64 scrollers,
/// `MAXLINEANIMS`) counts it and rule 6 (the 2 × 2 window of `spechit`,
/// `MAXSPECIALCROSS`) counts every two-sided special line, so both are unchanged.
pub(crate) fn validate_e1m1(map: &Map) -> Result<MoverAdmission, SimError> {
    validate_movers_with(map, Rules::E1M1)
}
/// The Episode 1 policy's load rules (M4a decision H3, plan §B.4): every rule of
/// [`validate_e1m1`] except that
///
/// - rule 2 (one family per sector) is not a load rule: `EV_DoDoor`,
///   `EV_DoPlat` and `EV_DoFloor` skip a busy sector as the source does, and a
///   DR line whose back sector holds a non-door thinker refuses its tic with
///   [`MapScaleError::MoverFamily`] (the per-tic witness of `movers.rs`);
/// - rule 4's reachable-height fixpoint is still computed for the vertical
///   invariant V′ (saturated at the `short` map-unit range so it terminates) but
///   its ±512 bound ([`MapScaleError::MoverHeights`]) and the inverting-door rule
///   ([`MapScaleError::InvertingDoor`]) refuse nothing: heights beyond ±512 are
///   covered by the checked trace arithmetic and the per-tic vertical witnesses;
/// - rule 6 (the 2 × 2 `spechit` window) is replaced by the ninth-`spechit`
///   witness [`MapScaleError::SpecialCross`], already a per-tic refusal;
/// - rule 8 (sky movers) is dropped: both sky bodies read runtime heights;
/// - rule 9 is generalised (decision D3): a switch line whose front side matches
///   no episode-1 switch is admitted (the source's texture search finds nothing:
///   see [`line_switch_match`]), and one that matches only a registered-episode or
///   commercial entry is refused ([`MapScaleError::SwitchTexture`]);
/// - rule 10 (a defined `blockbox`) holds for every sector, since stair and
///   donut sectors are not tagged;
/// - and every sector tagged by a donut line (special 9) passes
///   [`donut_geometry`] ([`MapScaleError::DonutGeometry`]).
///
/// The `ML_TWOSIDED`-implies-a-back-side rule `getNextSector` and
/// `EV_BuildStairs` rely on is already the diagonal geometry rule
/// (`MapScaleError::Geometry`). Rule 1's special set is the E1M1 policy's.
pub(crate) fn validate_episode1(map: &Map) -> Result<MoverAdmission, SimError> {
    validate_movers_with(map, Rules::Episode1)
}
/// Wire 24's load rules (M4b step 4): those of [`validate_episode1`] plus line
/// special 97 (WR teleport, p_spec.c:904–907), which moves no tagged family: its
/// `EV_Teleport` (p_telept.c:47–131) only searches the tagged sectors for a
/// landing. 39, 125 and 126 stay refused (decision I1).
pub(crate) fn validate_episode1_teleport(map: &Map) -> Result<MoverAdmission, SimError> {
    validate_movers_with(map, Rules::Episode1Teleport)
}
/// Which policy's load rules [`validate_movers_with`] runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rules {
    /// The movers policy (wire 19).
    Movers,
    /// The E1M1 policy and its supersets up to the level policy (wires 20–22).
    E1M1,
    /// The Episode 1 policy (wire 23).
    Episode1,
    /// Wire 24: the Episode 1 rules plus line special 97.
    Episode1Teleport,
}
/// The donut load rule of decision H8 for every sector `s1` tagged by a line with
/// special 9: `EV_DoDonut` (p_spec.c:1163–1221) reads `s1->lines[0]` through
/// `getNextSector` without a null test, so `s1`'s first line must be two-sided
/// with a back side and its next sector `s2` must differ from `s1`; it then scans
/// `s2->lines` for the first line `i` whose back sector is not `s1` (one-sided
/// lines included: the source's `!s2->lines[i]->flags & ML_TWOSIDED` is
/// `(!flags) & 4`, always 0, p_spec.c:1187–1189) and reads its back sector, which
/// must exist. Topology is static, so the rule is exact.
pub(crate) fn donut_geometry(map: &Map) -> Result<(), SimError> {
    let lists = crate::player_map_scale::sector_lines(map);
    let back = |line: usize| {
        map.linedefs()[line].sides[1].map(|s| usize::from(map.sidedefs()[usize::from(s)].sector))
    };
    for line in map.linedefs().iter().filter(|l| l.special == 9) {
        for s1 in tagged(map, line.tag) {
            let first = *lists[s1].first().ok_or(MapScaleError::DonutGeometry)?;
            let first = usize::from(first);
            if back(first).is_none() {
                return Err(MapScaleError::DonutGeometry.into());
            }
            let s2 = next_sector(map, first, s1).ok_or(MapScaleError::DonutGeometry)?;
            if s2 == s1 {
                return Err(MapScaleError::DonutGeometry.into());
            }
            if let Some(&i) = lists[s2]
                .iter()
                .find(|&&i| back(usize::from(i)) != Some(s1))
            {
                if back(usize::from(i)).is_none() {
                    return Err(MapScaleError::DonutGeometry.into());
                }
            }
        }
    }
    Ok(())
}
/// [`validate_movers`] under `rules`: a two-sided 48 is admitted from the E1M1
/// policy on, and the Episode 1 policy runs the rules of [`validate_episode1`].
fn validate_movers_with(map: &Map, rules: Rules) -> Result<MoverAdmission, SimError> {
    use crate::sector_thinkers::{MAX_LINE_ANIMS, SCROLLING_WALL};
    let two_sided_scrollers = rules != Rules::Movers;
    let episode1 = matches!(rules, Rules::Episode1 | Rules::Episode1Teleport);
    let teleport = rules == Rules::Episode1Teleport;
    crate::demon_expanded_map::validate_sector_specials_for(map, episode1)?;
    let sectors = map.sectors().len();
    let mut kinds = vec![Kinds::default(); sectors];
    let (mut scrollers, mut switches) = (0, 0);
    for line in map.linedefs() {
        let back = line.sides[1].map(|s| usize::from(map.sidedefs()[usize::from(s)].sector));
        // Rule 9: every switch line finds its front texture in `switchlist`;
        // under the Episode 1 policy, no front texture is a later-episode switch
        // without being an episode-1 one (decision D3).
        if episode1 && EPISODE1_SWITCH_SPECIALS.contains(&line.special) {
            let front = line.sides[0].ok_or(MapScaleError::SwitchTexture)?;
            let side = map
                .sidedefs()
                .get(usize::from(front))
                .ok_or(MapScaleError::SwitchTexture)?;
            if switch_match(&side.textures).is_none() && later_episode_switch(&side.textures) {
                return Err(MapScaleError::SwitchTexture.into());
            }
        } else if matches!(line.special, LINE_EXIT | LINE_DOOR_SR) {
            let front = line.sides[0].ok_or(MapScaleError::SwitchTexture)?;
            let side = map
                .sidedefs()
                .get(usize::from(front))
                .ok_or(MapScaleError::SwitchTexture)?;
            if switch_match(&side.textures).is_none() {
                return Err(MapScaleError::SwitchTexture.into());
            }
        }
        match line.special {
            0 | LINE_EXIT => {}
            // Episode 1 (decision H1): the light trigger, the secret exit, the
            // stairs and the donut move no tagged family (their sectors are
            // bounded below by `reachable` and checked by `donut_geometry`).
            35 | 51 | 7 | 8 | 9 if episode1 => {}
            // Wire 24: the WR teleport moves no sector (M4b step 4).
            crate::teleport::LINE_TELEPORT if teleport => {}
            SCROLLING_WALL if back.is_none() || two_sided_scrollers => scrollers += 1,
            LINE_DOOR_DR => kinds[back.ok_or(MapScaleError::DoorLine)?].dr = true,
            26..=28 if episode1 => kinds[back.ok_or(MapScaleError::DoorLine)?].dr = true,
            31..=34 if episode1 => kinds[back.ok_or(MapScaleError::DoorLine)?].open = true,
            special @ (2 | 5 | 16 | 18 | 20 | 22 | 23 | 46 | 62 | 70 | 76 | 82 | 86 | 90 | 91
            | 98 | 103)
                if episode1 =>
            {
                if matches!(special, 46 | 62 | 70) {
                    switches += 1;
                }
                for i in tagged(map, line.tag) {
                    let k = &mut kinds[i];
                    match special {
                        2 | 46 | 86 | 103 => k.open = true,
                        16 | 76 => k.close30 = true,
                        90 => k.sr = true,
                        5 | 91 => k.raise = true,
                        18 => k.nearest = true,
                        20 | 22 => k.change = true,
                        23 | 82 => k.lowest = true,
                        62 => k.lift = true,
                        _ => k.turbo = true,
                    }
                }
            }
            special @ (LINE_DOOR_SR | LINE_DOOR_BLAZE | LINE_LIFT | LINE_FLOOR_TURBO
            | LINE_FLOOR_LOWEST) => {
                if special == LINE_DOOR_SR {
                    switches += 1;
                }
                for i in tagged(map, line.tag) {
                    let k = &mut kinds[i];
                    match special {
                        LINE_DOOR_SR => k.sr = true,
                        LINE_DOOR_BLAZE => k.blaze = true,
                        LINE_LIFT => k.lift = true,
                        LINE_FLOOR_TURBO => k.turbo = true,
                        _ => k.lowest = true,
                    }
                }
            }
            _ => return Err(PursuitError::Specials.into()),
        }
    }
    if scrollers > MAX_LINE_ANIMS {
        return Err(PursuitError::Specials.into());
    }
    crate::demon_expanded_map::validate_lights(map)?;
    let mut family = Vec::with_capacity(sectors);
    for k in &kinds {
        let mut f = k.families();
        let first = f.next();
        // Rule 2; under the Episode 1 policy a per-tic witness (the first family
        // is recorded).
        if f.next().is_some() && !episode1 {
            return Err(MapScaleError::MoverFamily.into());
        }
        family.push(first);
    }
    let lists = crate::player_map_scale::sector_lines(map);
    let neighbors = |i: usize| {
        lists[i]
            .iter()
            .filter_map(move |&l| next_sector(map, usize::from(l), i))
    };
    for (i, f) in family.iter().enumerate() {
        let Some(f) = f else { continue };
        if !episode1 && crate::demon_expanded_map::is_sky_flat(&map.sectors()[i].flats[1]) {
            return Err(MapScaleError::SkyMover.into());
        }
        if neighbors(i).next().is_none() {
            return Err(MapScaleError::MoverNeighbor.into());
        }
        if *f == Family::Door && neighbors(i).any(|n| n == i) {
            return Err(MapScaleError::SelfReferencingDoor.into());
        }
    }
    // Rule 10: the box `P_ChangeSector` walks is defined for every mover sector.
    // Under the Episode 1 policy for every sector (stair and donut sectors are
    // not tagged).
    let boxes = crate::multisector::sector_block_boxes(map)?;
    if family
        .iter()
        .zip(&boxes)
        .any(|(f, b)| (episode1 || f.is_some()) && b.is_none())
    {
        return Err(MapScaleError::MoverBlockBox.into());
    }
    let plats = kinds.iter().filter(|k| k.lift || k.change).count();
    if plats > MAX_PLATS || switches > MAX_BUTTONS {
        return Err(MapScaleError::MoverCount.into());
    }
    let stairs = if episode1 {
        stair_bounds(map, &lists)
    } else {
        Vec::new()
    };
    let donuts = if episode1 {
        donut_triples(map, &lists)
    } else {
        Vec::new()
    };
    let (reach, delta) = reachable(map, &kinds, &neighbors, episode1, &stairs, &donuts)?;
    if episode1 {
        donut_geometry(map)?;
    } else if special_window_lines(map) > MAX_SPECIAL_CROSS && !window_rule_bypassed() {
        return Err(MapScaleError::SpecialWindow.into());
    }
    Ok(MoverAdmission {
        movers: family.iter().filter(|f| f.is_some()).count(),
        family,
        reach,
        delta,
    })
}

// Set only by `with_window_rule_bypassed`.
#[cfg(test)]
thread_local! {
    static WINDOW_RULE_BYPASSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// Whether a test has bypassed rule 6 (never outside `cfg(test)`).
fn window_rule_bypassed() -> bool {
    #[cfg(test)]
    {
        WINDOW_RULE_BYPASSED.with(std::cell::Cell::get)
    }
    #[cfg(not(test))]
    {
        false
    }
}
/// Runs `f` with rule 6 bypassed on this thread: the only way to load the Rust-only
/// nine-line lane and reach the `SpecialCross` runtime witness (M3h8 note §6).
#[cfg(test)]
pub(crate) fn with_window_rule_bypassed<R>(f: impl FnOnce() -> R) -> R {
    WINDOW_RULE_BYPASSED.with(|b| b.set(true));
    let result = f();
    WINDOW_RULE_BYPASSED.with(|b| b.set(false));
    result
}

/// Widens `interval` to contain `[lo, hi]`; whether it changed.
fn widen(interval: &mut [i32; 2], lo: i32, hi: i32) -> bool {
    let before = *interval;
    interval[0] = interval[0].min(lo);
    interval[1] = interval[1].max(hi);
    *interval != before
}

/// The `short` map-unit range the Episode 1 policy saturates its reachable-height
/// fixpoint to (the fixpoint then terminates without the ±512 refusal).
const SATURATED_UNITS: i32 = i16::MAX as i32;
/// The §5 least fixpoint of the floor and ceiling intervals of every sector under
/// the admitted rules, then rule 4b and `Δ`. Returns `[R_min, R_max]` and `Δ`.
/// With `saturate` (the Episode 1 policy) the intervals are clamped to
/// ±[`SATURATED_UNITS`] instead of refused beyond ±512, and rule 4b refuses
/// nothing.
/// The sectors `EV_BuildStairs` can raise from each sector tagged by a stair
/// line (S1 7, W1 8), with an upper bound on their final floors: the component
/// reachable through two-sided lines, each floor at most the largest static
/// floor of the component plus 8 per sector (each step adds 8 to the height of
/// the one before, plus 8 per skipped busy sector). Conservative: a Rust-only
/// input to the vertical invariant V′ (plan §C.10).
fn stair_bounds(map: &Map, lists: &[Vec<u16>]) -> Vec<(Vec<usize>, i32)> {
    let mut out = Vec::new();
    for line in map.linedefs().iter().filter(|l| matches!(l.special, 7 | 8)) {
        for start in tagged(map, line.tag) {
            let mut seen = vec![false; map.sectors().len()];
            let mut component = vec![start];
            seen[start] = true;
            let mut k = 0;
            while let Some(&sector) = component.get(k) {
                k += 1;
                for &l in &lists[sector] {
                    if let Some(n) = next_sector(map, usize::from(l), sector) {
                        if !seen[n] {
                            seen[n] = true;
                            component.push(n);
                        }
                    }
                }
            }
            let top = component
                .iter()
                .map(|&i| i32::from(map.sectors()[i].heights[0]))
                .max()
                .unwrap_or(0);
            let steps = i32::try_from(component.len()).unwrap_or(i32::MAX);
            out.push((component, top.saturating_add(steps.saturating_mul(8))));
        }
    }
    out
}
/// The `(s1, s2, s3)` sectors of every donut (S1 9) tagged sector where
/// `EV_DoDonut` finds them (see [`donut_geometry`]); skipped where the geometry
/// rule will refuse the map.
fn donut_triples(map: &Map, lists: &[Vec<u16>]) -> Vec<[usize; 3]> {
    let back = |line: usize| {
        map.linedefs()[line].sides[1].map(|s| usize::from(map.sidedefs()[usize::from(s)].sector))
    };
    let mut out = Vec::new();
    for line in map.linedefs().iter().filter(|l| l.special == 9) {
        for s1 in tagged(map, line.tag) {
            let Some(&first) = lists[s1].first() else {
                continue;
            };
            let Some(s2) = next_sector(map, usize::from(first), s1) else {
                continue;
            };
            let ring = lists[s2]
                .iter()
                .find(|&&i| back(usize::from(i)) != Some(s1))
                .and_then(|&i| back(usize::from(i)));
            if let Some(s3) = ring {
                out.push([s1, s2, s3]);
            }
        }
    }
    out
}
fn reachable<'m, I: Iterator<Item = usize> + 'm>(
    map: &'m Map,
    kinds: &[Kinds],
    neighbors: &impl Fn(usize) -> I,
    saturate: bool,
    stairs: &[(Vec<usize>, i32)],
    donuts: &[[usize; 3]],
) -> Result<([i32; 2], i32), SimError> {
    let static_heights = |i: usize, plane: usize| i32::from(map.sectors()[i].heights[plane]);
    let n = map.sectors().len();
    let mut f: Vec<[i32; 2]> = (0..n).map(|i| [static_heights(i, 0); 2]).collect();
    let mut c: Vec<[i32; 2]> = (0..n).map(|i| [static_heights(i, 1); 2]).collect();
    let top = |c: &[[i32; 2]], i: usize| {
        neighbors(i).fold([i32::MAX, i32::MAX], |t, m| {
            [t[0].min(c[m][0] - 4), t[1].min(c[m][1] - 4)]
        })
    };
    // P_FindHighestFloorSurrounding over the floor intervals, from −500.
    let start = i32::from(crate::movers::HIGHEST_FLOOR_START);
    let highest_floor = |f: &[[i32; 2]], i: usize| {
        neighbors(i).fold([start, start], |h, m| {
            [h[0].max(f[m][0]), h[1].max(f[m][1])]
        })
    };
    let in_bounds = |v: &[[i32; 2]]| {
        v.iter()
            .all(|r| r[0] >= -REACHABLE_UNITS && r[1] <= REACHABLE_UNITS)
    };
    loop {
        let mut changed = false;
        // Under saturation a widening past the clamp is undone by the clamp, so
        // a round counts as a change only when the clamped intervals differ.
        let previous = saturate.then(|| (f.clone(), c.clone()));
        for (i, k) in kinds.iter().enumerate() {
            if k.dr || k.sr {
                // A normal door rises to its top and closes to its floor.
                let t = top(&c, i);
                let lo = f[i][0].min(t[0]);
                changed |= widen(&mut c[i], lo, t[1]);
            }
            if k.blaze || k.open {
                let t = top(&c, i);
                changed |= widen(&mut c[i], t[0], t[1]);
            }
            if k.close30 {
                // Closes to its floor and reopens to the ceiling it had.
                let lo = f[i][0].min(c[i][0]);
                let hi = c[i][1];
                changed |= widen(&mut c[i], lo, hi);
            }
            if k.raise {
                // P_FindLowestCeilingSurrounding, at most the own ceiling.
                let hi = c[i][1];
                let lo = f[i][0];
                changed |= widen(&mut f[i], lo, hi);
            }
            if k.nearest || k.change {
                // P_FindNextHighestFloor: a neighbour floor (or the own floor).
                let hi = neighbors(i).fold(f[i][1], |hi, m| hi.max(f[m][1]));
                let lo = f[i][0];
                changed |= widen(&mut f[i], lo, hi);
            }
            if k.lift || k.lowest {
                // P_FindLowestFloorSurrounding starts at the sector's own floor.
                let lo = neighbors(i).fold(f[i][0], |lo, m| lo.min(f[m][0]));
                let hi = f[i][1];
                changed |= widen(&mut f[i], lo, hi);
            }
            if k.turbo {
                // P_FindHighestFloorSurrounding folds from −500 (p_spec.c:297–316);
                // EV_DoFloor adds 8 unless the destination equals the floor
                // (p_floor.c:307–315): no movement only when the destination is one
                // value equal to a constant own floor.
                let hi = highest_floor(&f, i);
                let constant = hi[0] == hi[1] && f[i] == hi;
                if !constant {
                    changed |= widen(&mut f[i], hi[0] + 8, hi[1] + 8);
                }
            }
        }
        for (component, hi) in stairs {
            for &i in component {
                let lo = f[i][0];
                changed |= widen(&mut f[i], lo, *hi);
            }
        }
        for &[s1, s2, s3] in donuts {
            // The hole lowers and the ring rises to the s3 floor.
            let [lo, hi] = f[s3];
            let own = f[s1][1];
            changed |= widen(&mut f[s1], lo, own);
            let own = f[s2][0];
            changed |= widen(&mut f[s2], own, hi);
        }
        if let Some((pf, pc)) = previous {
            for r in f.iter_mut().chain(c.iter_mut()) {
                *r = r.map(|v| v.clamp(-SATURATED_UNITS, SATURATED_UNITS));
            }
            changed = f != pf || c != pc;
        } else if !in_bounds(&f) || !in_bounds(&c) {
            return Err(MapScaleError::MoverHeights.into());
        }
        if !changed {
            break;
        }
    }
    let mut delta = 0;
    for (i, k) in kinds.iter().enumerate() {
        if k.dr || k.sr {
            delta = delta.max(2);
        }
        if k.lift {
            delta = delta.max(4);
        }
        if k.open || k.close30 {
            delta = delta.max(2);
        }
        if k.raise || k.nearest || k.change {
            delta = delta.max(1);
        }
        if k.close30 {
            delta = delta.max(c[i][1].saturating_sub(f[i][0]));
        }
        if k.door() {
            let t = top(&c, i);
            if t[0] < f[i][1] && !saturate {
                return Err(MapScaleError::InvertingDoor.into());
            }
            delta = delta.max(c[i][1].saturating_sub(t[0]));
        }
        if k.turbo {
            delta = delta.max((highest_floor(&f, i)[1] + 8).saturating_sub(f[i][0]));
        }
    }
    if !stairs.is_empty() || !donuts.is_empty() {
        // Stair steps rise at 1/4 unit, donut floors at 1/2.
        delta = delta.max(1);
    }
    let lo = f.iter().chain(&c).map(|r| r[0]).min().unwrap_or(0);
    let hi = f.iter().chain(&c).map(|r| r[1]).max().unwrap_or(0);
    Ok(([lo, hi], delta))
}

/// Rule 6: the largest number of distinct lines with a back side and a nonzero
/// special listed in one 2 × 2 window of the map's own BLOCKMAP cells (a window at
/// the last row or column is clipped to the grid).
pub fn special_window_lines(map: &Map) -> usize {
    let blockmap = map.blockmap();
    let [width, height] = blockmap.dimensions().map(usize::from);
    let words = blockmap.words();
    let ranges = blockmap.cell_ranges();
    let lines = map.linedefs();
    let mut best = 0;
    let mut seen = vec![usize::MAX; lines.len()];
    for cy in 0..height.max(1) {
        for cx in 0..width.max(1) {
            let stamp = cy * width + cx;
            let mut distinct = 0;
            for wy in [cy, cy + 1] {
                for wx in [cx, cx + 1] {
                    if wx >= width || wy >= height {
                        continue;
                    }
                    for &word in &words[ranges[wy * width + wx].clone()] {
                        let line = usize::from(word);
                        if line < lines.len()
                            && seen[line] != stamp
                            && lines[line].sides[1].is_some()
                            && lines[line].special != 0
                        {
                            seen[line] = stamp;
                            distinct += 1;
                        }
                    }
                }
            }
            best = best.max(distinct);
        }
    }
    best
}
