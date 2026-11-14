//! Bounded, read-only DSIM20–26 observations with explicit version readers. Never a world loader.
use crate::{identity::*, *};
use std::{error::Error, fmt, sync::OnceLock};

/// Explicit resource budgets, applied before allocation or untrusted length use.
#[derive(Clone, Copy, Debug)]
pub struct DiagnosticLimits {
    /// Maximum complete input/output bytes.
    pub bytes: usize,
    /// Maximum named fields.
    pub fields: usize,
    /// Maximum UTF-8 path bytes.
    pub path: usize,
    /// Maximum individual opaque value bytes.
    pub value: usize,
}
impl Default for DiagnosticLimits {
    fn default() -> Self {
        Self {
            bytes: 64 * 1024 * 1024,
            fields: 262_144,
            path: 192,
            value: 64 * 1024 * 1024,
        }
    }
}
/// Explicit rejection; diagnostics never cause implicit profile selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticError {
    /// Malformed framing, widths, UTF-8, duplicate paths or missing required fields.
    Structure,
    /// Unsupported diagnostic format.
    Version(u32),
    /// Unknown/incompatible execution profile or immutable registry/context schema.
    Schema,
    /// Invalid definition/state identity or live handle representation.
    Identity,
    /// Caller resource budget, arithmetic or fallible allocation failed.
    Limit,
}
impl fmt::Display for DiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "diagnostic: {self:?}")
    }
}
impl Error for DiagnosticError {}
/// Version-specific observations; native slots never acquire canonical meaning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticIdentity {
    /// Historical v20 runtime storage, interpreted only in that namespace.
    Native {
        /// Native type slot.
        actor: ActorTypeId,
        /// Native state slot or unavailable.
        state: Option<StateId>,
    },
    /// V21 actual source/context definition and source state.
    Original {
        /// Checked original/context identity.
        definition: DefinitionIdentity,
        /// Original state or unavailable.
        state: Option<OriginalState>,
    },
}
#[derive(Clone, Copy, Debug)]
struct Field<'a> {
    path: &'a str,
    value: &'a [u8],
}
/// Borrowed validated framing and identity observations, preserving encoded order.
///
/// Physical/map values and non-identity event payloads remain opaque. This is
/// neither a full gameplay validator nor a savegame restoration API.
#[derive(Debug)]
pub struct Diagnostic<'a> {
    bytes: &'a [u8],
    version: u32,
    fields: Vec<Field<'a>>,
    sorted: Vec<usize>,
}
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DiagnosticError> {
        let end = self.at.checked_add(n).ok_or(DiagnosticError::Limit)?;
        let out = self
            .bytes
            .get(self.at..end)
            .ok_or(DiagnosticError::Structure)?;
        self.at = end;
        Ok(out)
    }
    fn u32(&mut self) -> Result<u32, DiagnosticError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| DiagnosticError::Structure)?,
        ))
    }
    fn usize64(&mut self) -> Result<usize, DiagnosticError> {
        usize::try_from(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| DiagnosticError::Structure)?,
        ))
        .map_err(|_| DiagnosticError::Limit)
    }
}
fn number<const N: usize>(value: &[u8]) -> Result<[u8; N], DiagnosticError> {
    value.try_into().map_err(|_| DiagnosticError::Structure)
}
fn reserve<T>(n: usize) -> Result<Vec<T>, DiagnosticError> {
    let mut out = Vec::new();
    out.try_reserve_exact(n)
        .map_err(|_| DiagnosticError::Limit)?;
    Ok(out)
}
impl<'a> Diagnostic<'a> {
    /// Read only v20/v21 with explicit identity/schema validation and bounded allocation.
    pub fn decode(bytes: &'a [u8], limits: DiagnosticLimits) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, None)
    }
    /// Explicit DSIM-v22 player-body reader. Historical decode still rejects22.
    /// Shares bounded framing; observations never restore executable worlds.
    pub fn decode_player(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(22))
    }
    /// Explicit DSIM23 reader for player-owned pistol observations.
    /// Historical readers continue rejecting this separate field layout.
    pub fn decode_pistol(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(23))
    }
    /// Explicit DSIM25 world-pickup reader; older readers retain their version gate.
    pub fn decode_pickups(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(25))
    }
    /// Explicit DSIM24 reader for the original four-weapon capability.
    /// Decoding validates observations, never restores an executable world.
    pub fn decode_weapons(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(24))
    }
    /// Explicit DSIM26 original armor reader; historical entry points reject26.
    pub fn decode_armor(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(26))
    }
    /// Nonpending fireball bound of `PlayerMonstersImpV1`, looked up per slot
    /// because frames need not list fields in canonical order.
    fn validate_live_projectiles(&self) -> Result<(), DiagnosticError> {
        self.validate_live_projectiles_at(crate::player_map_scale::live_projectile_bound(
            self.policy(),
        ))
    }
    pub(crate) fn validate_live_projectiles_at(&self, bound: usize) -> Result<(), DiagnosticError> {
        let fireball = OriginalActorType::MT_TROOPSHOT.ordinal().to_le_bytes();
        let mut live = 0;
        for slot in 0..self.slots()? {
            if self.required(&format!("actor.{slot}.occupied"))? != [1] {
                continue;
            }
            let pending = self.required(&format!("actor.{slot}.pending_removal"))?;
            if !matches!(pending, [0 | 1]) {
                return Err(DiagnosticError::Structure);
            }
            if self.required(&format!("actor.{slot}.type"))? == fireball && pending == [0] {
                live += 1;
            }
        }
        if live > bound {
            return Err(DiagnosticError::Limit);
        }
        Ok(())
    }
    /// Explicit DSIM31 static-heights reader; every earlier reader rejects 31 and
    /// this reader rejects every earlier version. DSIM31 keeps the DSIM30 field
    /// layout and event vocabulary; it widens the item witnesses to the item's own
    /// sector heights.
    pub fn decode_heights(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(31))
    }
    /// Explicit DSIM32 diagonal-wall reader; every earlier reader rejects 32 and
    /// this reader rejects every earlier version. DSIM32 keeps the DSIM31 field
    /// layout, event vocabulary and item/vertical witnesses under policy 16 and the
    /// diagonal context; diagonal walls add no persistent field.
    pub fn decode_diagonal(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(32))
    }
    /// Explicit DSIM33 real-map reader; every earlier reader rejects 33 and this
    /// reader rejects every earlier version. DSIM33 keeps the DSIM32 field layout
    /// and event vocabulary under policy 17 and the real-map context; the caps the
    /// DSIM32 reader checks as constants (256 THINGs, 256 sectors) are read from the
    /// stream's own tables, bounded by the WAD format. No persistent field is added.
    pub fn decode_real_map(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(33))
    }
    /// Explicit DSIM34 sky and static-specials reader; every earlier reader
    /// rejects 34 and this reader rejects every earlier version. DSIM34 is the
    /// DSIM33 layout plus the shared thinker order, the sector-thinker store, the
    /// runtime sector light and special, the player's secret count and event tag
    /// 25 (STATE_FORMAT "DSIM v34"), each validated here.
    pub fn decode_specials(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(34))
    }
    /// Explicit DSIM35 doors, lifts, floors, switches and exit reader; every earlier
    /// reader, `decode_specials` included, rejects 35 and this reader rejects every
    /// earlier version. DSIM35 is the DSIM34 layout plus `level.exit`, moving sector
    /// heights, the runtime line specials, door/plat/floor and removed-mover records
    /// in the sector-thinker store, the `buttonlist` slots, `use_down`, the reborn
    /// request (`original_player.0.state` 2) and event tag 26 (STATE_FORMAT
    /// "DSIM v35"), each validated here. Giblets (`S_GIBS`, height and radius 0,
    /// `MF_SOLID` clear) are admitted for monsters and the player.
    pub fn decode_movers(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(35))
    }
    /// Explicit DSIM36 E1M1 reader (two-sided scrolling walls and the seven static
    /// decoration types, M3h9 note §5.1); every earlier reader, `decode_movers`
    /// included, rejects 36 and this reader rejects every earlier version. DSIM36
    /// is the DSIM35 layout, event vocabulary and witnesses under policy 20 and the
    /// E1M1 context (decision E8): a two-sided 48 adds no field (its offset is
    /// `leveltime × FRACUNIT`, p_spec.c:1114–1124), and a decoration is an ordinary
    /// actor record of its own type, spawn state, flags, radius and height
    /// (STATE_FORMAT "DSIM v36").
    pub fn decode_e1m1(bytes: &'a [u8], limits: DiagnosticLimits) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(36))
    }
    /// Explicit DSIM37 barrels reader (M3h9b note §5.1, decision F9); every earlier
    /// reader, `decode_e1m1` included, rejects 37 and this reader rejects every
    /// earlier version. DSIM37 is the DSIM36 layout, event vocabulary and witnesses
    /// under policy 21 and the barrels context, with the bounds of decision F2:
    /// `store.max_slots` 384, up to 64 items (`level.total_items`,
    /// `pickups.item_count` and the pickup events) and a `bonuscount` up to 6 × 64.
    pub fn decode_barrels(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(37))
    }
    /// Explicit DSIM38 level reader (M3h9c, decision G4); every earlier reader,
    /// `decode_barrels` included, rejects 38 and this reader rejects every earlier
    /// version. DSIM38 is the DSIM37 layout, event vocabulary, bounds and witnesses
    /// under policy 22 and the level context, with three differences: the player
    /// body's angle is any value (`ANG45 * (angle/45)` of its record at spawn,
    /// p_mobj.c:671, then the turns of `P_DeathThink`, p_user.c:200–222), its
    /// `damage_count` and `bonus_count` keep their DSIM37 domains but decay
    /// (p_user.c:355–359), and the body sits at its THINGS record's lump position
    /// (`P_LoadThings`, p_setup.c:313–346): the lump holds exactly one type-1
    /// record, the body carries no spawnpoint (`P_SpawnPlayer` sets none;
    /// `P_SpawnMapThing` returns before p_mobj.c:785), every actor before it in
    /// `thinker.order` carries a spawnpoint matching, in order, a record before
    /// the type-1 record, and its slot is at least the number of those actors and
    /// at most the number of those records.
    pub fn decode_level(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(38))
    }
    /// Explicit DSIM39 Episode 1 reader (M4a, decisions H2–H4); every earlier
    /// reader, `decode_level` included, rejects 39 and this reader rejects every
    /// earlier version. DSIM39 is the DSIM38 layout, event vocabulary and
    /// witnesses under policy 23 and the Episode 1 context, with these
    /// differences: `store.max_slots` is 1,024; the roster, pickup, drop,
    /// decoration and barrel bounds are the frame's own, counted from
    /// `map.things` (`Episode1Bounds::from_thing_records`); and these fields are
    /// added: `level.secret_exit` after `level.exit`; `sector.N.floorpic` (the
    /// normalized floor flat name) after `sector.N.special`; a plat's `type` after
    /// `high` and a floor's `direction` and `speed` after `floordestheight`;
    /// `original_player.0.weapons.cells`, `.rockets` and `.max_ammo.0` to `.3`
    /// after `.shells`; and `original_player.0.powers.0` to `.5`, `.cards.0` to
    /// `.5` and `.backpack` after `original_player.0.use_down`. Pickup message
    /// codes 14–29 are admitted.
    pub fn decode_episode1(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(39))
    }
    /// Explicit DSIM40 Episode 1 part 2 reader (M4b, decision I2); every earlier
    /// reader, `decode_episode1` included, rejects 40 and this reader rejects every
    /// earlier version. DSIM40 is the DSIM39 layout, event vocabulary, bounds and
    /// witnesses under policy 24 and the `PLAYER-MONSTERS-EPISODE1-2-v1` context,
    /// plus `level.map` (u8, 1–9) right after `level.secret_exit` (STATE_FORMAT
    /// "DSIM v40"). `actors.options` must read skill 2 or 3 (DSIM36–39 admit 2
    /// only), and the per-map bounds and the placed totals are counted with that
    /// skill's THING option bit (2 or 4, p_mobj.c:743–751; M4b step 6); the new
    /// types, states and event sub-tags of the format are admitted with their
    /// behaviours in M4b steps 4–6.
    pub fn decode_episode1_v2(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(40))
    }
    /// Explicit DSIM30 real-map-scale reader; every earlier reader rejects 30 and
    /// this reader rejects every earlier version.
    pub fn decode_map_scale(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(30))
    }
    /// Explicit DSIM29 Imp/fireball reader; every earlier reader rejects 29 and
    /// this reader rejects every earlier version.
    pub fn decode_imps(bytes: &'a [u8], limits: DiagnosticLimits) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(29))
    }
    /// Explicit DSIM28 former-human/mixed-roster reader; every earlier reader
    /// rejects28 and this reader rejects every earlier version.
    pub fn decode_monsters(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(28))
    }
    /// Explicit DSIM27 bonus reader; historical version readers reject27.
    pub fn decode_bonuses(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(27))
    }
    fn decode_selected(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
        selected: Option<u32>,
    ) -> Result<Self, DiagnosticError> {
        if bytes.len() > limits.bytes {
            return Err(DiagnosticError::Limit);
        }
        let mut cursor = Cursor { bytes, at: 0 };
        if cursor.take(4)? != b"DSIM" {
            return Err(DiagnosticError::Structure);
        }
        let version = cursor.u32()?;
        if selected.map_or(!matches!(version, 20 | 21), |expected| version != expected) {
            return Err(DiagnosticError::Version(version));
        }
        let count = cursor.usize64()?;
        // Every field needs at least its 12 framing bytes, before reserving.
        if count > limits.fields || count > bytes.len().saturating_sub(cursor.at) / 12 {
            return Err(DiagnosticError::Limit);
        }
        let mut fields = reserve(count)?;
        for _ in 0..count {
            let len = usize::try_from(cursor.u32()?).map_err(|_| DiagnosticError::Limit)?;
            if len == 0 || len > limits.path {
                return Err(DiagnosticError::Limit);
            }
            let path =
                std::str::from_utf8(cursor.take(len)?).map_err(|_| DiagnosticError::Structure)?;
            let len = cursor.usize64()?;
            if len > limits.value {
                return Err(DiagnosticError::Limit);
            }
            fields.push(Field {
                path,
                value: cursor.take(len)?,
            });
        }
        if cursor.at != bytes.len() {
            return Err(DiagnosticError::Structure);
        }
        let mut sorted = reserve(count)?;
        sorted.extend(0..count);
        sorted.sort_unstable_by_key(|i| fields[*i].path);
        if sorted
            .windows(2)
            .any(|p| fields[p[0]].path == fields[p[1]].path)
        {
            return Err(DiagnosticError::Structure);
        }
        let result = Self {
            bytes,
            version,
            fields,
            sorted,
        };
        result.validate()?;
        Ok(result)
    }
    /// Historical-reader contract: a newer original-ID representation must reject.
    pub fn decode_v20(bytes: &'a [u8], limits: DiagnosticLimits) -> Result<Self, DiagnosticError> {
        let result = Self::decode(bytes, limits)?;
        if result.version != 20 {
            return Err(DiagnosticError::Version(result.version));
        }
        Ok(result)
    }
    /// Explicit format version, separate from the execution profile field.
    pub const fn version(&self) -> u32 {
        self.version
    }
    /// Borrow a field without reordering the original capture/event sequence.
    pub fn field(&self, path: &str) -> Option<&'a [u8]> {
        self.sorted
            .binary_search_by_key(&path, |i| self.fields[*i].path)
            .ok()
            .map(|i| self.fields[self.sorted[i]].value)
    }
    fn required(&self, path: &str) -> Result<&'a [u8], DiagnosticError> {
        self.field(path).ok_or(DiagnosticError::Structure)
    }
    /// DSIM39: the per-map bounds counted from the frame's `map.things`; `None`
    /// for every other version (and for a malformed lump, which the THINGS
    /// validators refuse).
    fn e1(&self) -> Option<Episode1Bounds> {
        if !matches!(self.version, 39 | 40) {
            return None;
        }
        if self.version == 40 {
            // Wire 24 counts the Baron (doomednum 3003) in `K` (M4b step 5), and
            // counts with the option bit of the frame's skill (step 6).
            return Episode1Bounds::from_thing_records_v2(
                self.field("map.things")?,
                self.v40_skill(),
            );
        }
        Episode1Bounds::from_thing_records(self.field("map.things")?)
    }
    /// DSIM40: the skill the frame's bounds are counted at, from
    /// `actors.options[0]`: hard for 3, otherwise medium. A frame whose skill is
    /// neither 2 nor 3 (or that lacks the field) is refused by
    /// `validate_decorations`; counting it at medium keeps the step-5 behaviour
    /// for such frames until that refusal.
    fn v40_skill(&self) -> crate::Skill {
        match self.field("actors.options").and_then(|o| o.first()) {
            Some(&b) if b == crate::Skill::Hard as u8 => crate::Skill::Hard,
            _ => crate::Skill::Medium,
        }
    }
    fn slot_bound(&self) -> u32 {
        slot_bound_for(self.version)
    }
    fn slots(&self) -> Result<usize, DiagnosticError> {
        let count = u64::from_le_bytes(number(self.required("store.slot_count")?)?);
        if count > u64::from(self.slot_bound()) {
            return Err(DiagnosticError::Limit);
        }
        usize::try_from(count).map_err(|_| DiagnosticError::Limit)
    }
    fn identity(&self, ty: u16, state: u16) -> Result<DiagnosticIdentity, DiagnosticError> {
        let actor = ActorTypeId(ty);
        let state = (state != u16::MAX).then_some(StateId(state));
        if matches!(self.version, 21..=40) {
            let policy = self.policy();
            original_registry::require_role_for(policy, actor)
                .map_err(|_| DiagnosticError::Identity)?;
            if let Some(s) = state {
                if !self.gibs_admitted(actor, s) {
                    original_registry::state_role_for(policy, actor, s)
                        .map_err(|_| DiagnosticError::Identity)?;
                }
            }
            Ok(DiagnosticIdentity::Original {
                definition: original_registry::identity(actor)
                    .map_err(|_| DiagnosticError::Identity)?,
                state: state
                    .map(|s| OriginalState::try_from(s.0).map_err(|_| DiagnosticError::Identity))
                    .transpose()?,
            })
        } else {
            let valid = match ty {
                136 => state.is_none_or(|s| NativeDemonState::checked(s).is_some()),
                135 => state.is_none_or(|s| (149..=173).contains(&s.0)),
                37 | 38 => state.is_none_or(|s| (90..=96).contains(&s.0)),
                _ => false,
            };
            if !valid {
                return Err(DiagnosticError::Identity);
            }
            Ok(DiagnosticIdentity::Native { actor, state })
        }
    }
    /// Resolve only a currently occupied slot with the exact generation.
    /// Stale references and retired/reused slots never redirect to another actor.
    pub fn actor(&self, handle: ActorId) -> Result<Option<DiagnosticIdentity>, DiagnosticError> {
        let slot = usize::try_from(handle.slot()).map_err(|_| DiagnosticError::Identity)?;
        if slot >= self.slots()? {
            return Ok(None);
        }
        let prefix = format!("actor.{slot}");
        let generation =
            u64::from_le_bytes(number(self.required(&format!("{prefix}.generation"))?)?);
        if generation != handle.generation() || self.required(&format!("{prefix}.occupied"))? == [0]
        {
            return Ok(None);
        }
        let ty = u16::from_le_bytes(number(self.required(&format!("{prefix}.type"))?)?);
        let state = u16::from_le_bytes(number(self.required(&format!("{prefix}.state"))?)?);
        self.identity(ty, state).map(Some)
    }
    /// DSIM35 to DSIM38 only: `S_GIBS` (`PIT_ChangeSector`'s giblets) for an
    /// admitted monster type or `MT_PLAYER`, and under DSIM37/38 a dying barrel
    /// (p_map.c:1269–1279); its dimensions are checked by `validate_movers`.
    fn gibs_admitted(&self, actor: ActorTypeId, state: StateId) -> bool {
        matches!(self.version, 35..=40)
            && state.0 == OriginalState::S_GIBS.ordinal()
            && (actor.0 == OriginalActorType::MT_PLAYER.ordinal()
                || original_registry::is_policy_monster(self.policy(), actor)
                || crate::barrels::is_barrel(self.policy(), actor))
    }
    fn policy(&self) -> OriginalExecutionProfile {
        if self.version == 40 {
            return OriginalExecutionProfile::PlayerMonstersEpisode1V2;
        }
        if self.version == 39 {
            return OriginalExecutionProfile::PlayerMonstersEpisode1V1;
        }
        if self.version == 38 {
            return OriginalExecutionProfile::PlayerMonstersLevelV1;
        }
        if self.version == 37 {
            return OriginalExecutionProfile::PlayerMonstersBarrelsV1;
        }
        if self.version == 36 {
            return OriginalExecutionProfile::PlayerMonstersE1M1V1;
        }
        if self.version == 35 {
            return OriginalExecutionProfile::PlayerMonstersMoversV1;
        }
        if self.version == 34 {
            return OriginalExecutionProfile::PlayerMonstersSpecialsV1;
        }
        if self.version == 33 {
            return OriginalExecutionProfile::PlayerMonstersRealMapV1;
        }
        if self.version == 32 {
            return OriginalExecutionProfile::PlayerMonstersDiagonalV1;
        }
        if self.version == 31 {
            return OriginalExecutionProfile::PlayerMonstersHeightsV1;
        }
        if self.version == 30 {
            return OriginalExecutionProfile::PlayerMonstersMapScaleV1;
        }
        if self.version == 29 {
            return OriginalExecutionProfile::PlayerMonstersImpV1;
        }
        if self.version == 28 {
            return OriginalExecutionProfile::PlayerMonstersV1;
        }
        if self.version == 27 {
            return OriginalExecutionProfile::PlayerDemonBonusesV1;
        }
        if self.version == 26 {
            return OriginalExecutionProfile::PlayerDemonArmorV1;
        }
        if self.version == 25 {
            return OriginalExecutionProfile::PlayerDemonPickupsV1;
        }
        if self.version == 24 {
            return OriginalExecutionProfile::PlayerDemonWeaponsV1;
        }
        if self.version == 23 {
            return OriginalExecutionProfile::PlayerDemonPistolV1;
        }
        if matches!(self.version, 22..=40)
            && self.field("profile")
                == Some(
                    OriginalExecutionProfile::PlayerDemonMeleeV1
                        .wire_id()
                        .to_le_bytes()
                        .as_slice(),
                )
        {
            OriginalExecutionProfile::PlayerDemonMeleeV1
        } else if matches!(self.version, 22..=40)
            && self.field("profile")
                == Some(
                    OriginalExecutionProfile::PlayerDemonAcquisitionV1
                        .wire_id()
                        .to_le_bytes()
                        .as_slice(),
                )
        {
            OriginalExecutionProfile::PlayerDemonAcquisitionV1
        } else if matches!(self.version, 22..=40)
            && self.field("profile")
                == Some(
                    OriginalExecutionProfile::PlayerCommandMovementV1
                        .wire_id()
                        .to_le_bytes()
                        .as_slice(),
                )
        {
            OriginalExecutionProfile::PlayerCommandMovementV1
        } else if matches!(self.version, 22..=40)
            && self.field("profile")
                == Some(
                    OriginalExecutionProfile::PlayerBodyKnockbackV1
                        .wire_id()
                        .to_le_bytes()
                        .as_slice(),
                )
        {
            OriginalExecutionProfile::PlayerBodyKnockbackV1
        } else if matches!(self.version, 22..=40) {
            OriginalExecutionProfile::PlayerBodyLifecycleV1
        } else {
            OriginalExecutionProfile::DemonNormalV1
        }
    }
    fn validate(&self) -> Result<(), DiagnosticError> {
        let profile = u32::from_le_bytes(number(self.required("profile")?)?);
        if u32::from_le_bytes(number(self.required("tic_rate")?)?) != 35 {
            return Err(DiagnosticError::Schema);
        }
        number::<8>(self.required("tic")?)?;
        if matches!(self.version, 21..=40) {
            if profile != self.policy().wire_id()
                || self.required("registry.schema")? != ORIGINAL_REGISTRY_SCHEMA
                || self.required("context.schema")? != self.policy().context_schema()
                || self
                    .fields
                    .iter()
                    .any(|f| f.path.starts_with("registry.") && f.path != "registry.schema")
            {
                return Err(DiagnosticError::Schema);
            }
        } else {
            if profile != 20
                || self.field("registry.schema").is_some()
                || self.field("context.schema").is_some()
            {
                return Err(DiagnosticError::Schema);
            }
            // Old full metadata is meaningful only in its old namespace.
            static LEGACY: OnceLock<Vec<StateField>> = OnceLock::new();
            let expected = LEGACY.get_or_init(|| {
                let mut fields = Vec::new();
                snapshot::registry_fields(&mut fields, &ActorRegistry::demon_map_context_fixture());
                fields
            });
            for f in expected {
                if self.required(f.path())? != f.value() {
                    return Err(DiagnosticError::Schema);
                }
            }
        }
        if u32::from_le_bytes(number(self.required("store.max_slots")?)?) > self.slot_bound() {
            return Err(DiagnosticError::Limit);
        }
        for slot in 0..self.slots()? {
            let prefix = format!("actor.{slot}");
            let generation =
                u64::from_le_bytes(number(self.required(&format!("{prefix}.generation"))?)?);
            if generation == 0 {
                return Err(DiagnosticError::Identity);
            }
            match self.required(&format!("{prefix}.occupied"))? {
                [0] => {}
                [1] => {
                    self.actor(ActorId::from_parts(
                        u32::try_from(slot).map_err(|_| DiagnosticError::Limit)?,
                        generation,
                    ))?;
                }
                _ => return Err(DiagnosticError::Structure),
            }
        }
        // Handles identify instances, never definitions. Unavailable target/roster
        // references may be stale, but their bounded slot and width remain checked.
        for f in &self.fields {
            let optional = f.path == "store.head"
                || f.path == "store.tail"
                || f.path.ends_with(".target")
                || f.path.ends_with(".soundtarget")
                || f.path.ends_with(".previous")
                || f.path.ends_with(".next")
                || matches!(self.version, 22..=40)
                    && matches!(
                        f.path,
                        "original_player.0.mo" | "original_player.0.attacker"
                    )
                || f.path.starts_with("player.") && f.path.ends_with(".actor")
                || f.path.starts_with("perception.player.") && f.path.ends_with(".actor");
            if optional {
                let value = number::<13>(f.value)?;
                match value[0] {
                    0 if value[1..].iter().all(|b| *b == 0) => {}
                    1 => validate_handle(&value[1..], self.slot_bound())?,
                    _ => return Err(DiagnosticError::Identity),
                }
            }
            if f.path == "thinker.order"
                || f.path == "spatial.link_order"
                || f.path.starts_with("sector.") && f.path.ends_with(".actors")
            {
                if f.value.len() % 12 != 0 || f.value.len() / 12 > self.slot_bound() as usize {
                    return Err(DiagnosticError::Identity);
                }
                for handle in f.value.chunks_exact(12) {
                    validate_handle(handle, self.slot_bound())?;
                }
            }
            if let Some(rest) = f.path.strip_prefix("actor.") {
                let (slot, _) = rest.split_once('.').ok_or(DiagnosticError::Structure)?;
                let slot: usize = slot.parse().map_err(|_| DiagnosticError::Structure)?;
                if slot >= self.slots()? {
                    return Err(DiagnosticError::Identity);
                }
            }
        }
        for prefix in ["events.last", "events.pending"] {
            let count = u64::from_le_bytes(number(self.required(&format!("{prefix}.count"))?)?);
            let count = usize::try_from(count).map_err(|_| DiagnosticError::Limit)?;
            if count > self.fields.len() {
                return Err(DiagnosticError::Limit);
            }
            for i in 0..count {
                let event = self.required(&format!("{prefix}.{i}"))?;
                let tag = *event.first().ok_or(DiagnosticError::Structure)?;
                if tag
                    > if matches!(self.version, 35..=40) {
                        26
                    } else if self.version == 34 {
                        25
                    } else if matches!(self.version, 26..=40) {
                        24
                    } else if self.version == 25 {
                        23
                    } else if matches!(self.version, 22..=40) {
                        22
                    } else {
                        21
                    }
                {
                    return Err(DiagnosticError::Structure);
                }
                if matches!(self.version, 27..=40) && tag == 21 {
                    if event.len() != 18 || !matches!(event[1], 0..=2) {
                        return Err(DiagnosticError::Structure);
                    }
                    validate_handle(&event[2..14], self.slot_bound())?;
                    let total = i32::from_le_bytes(number(&event[14..18])?);
                    if total < 0
                        || (event[1] == 2
                            && !(1..=pickup_bound_for(self.version, self.e1())).contains(&total))
                    {
                        return Err(DiagnosticError::Identity);
                    }
                }
                if tag == 23 {
                    validate_pickup_event(event, self.version, self.slot_bound(), self.e1())?;
                }
                if tag == 26 {
                    self.validate_mover_event(event)?;
                }
                if tag == 25 {
                    let sectors = u32::from_le_bytes(number(self.required("sectors.count")?)?);
                    validate_special_event(event, self.slot_bound(), sectors)?;
                    self.validate_sky_event(event)?;
                }
                if tag == 24 {
                    validate_armor_event(event, self.version, self.slot_bound())?;
                    let mo = self.required("original_player.0.mo")?;
                    if mo.len() != 13 || mo[0] != 1 || mo[1..] != event[1..13] {
                        return Err(DiagnosticError::Identity);
                    }
                }
                if tag == 22 {
                    validate_player_event(
                        event,
                        matches!(self.version, 24..=40),
                        matches!(self.version, 27..=40),
                        matches!(self.version, 39 | 40),
                        self.slot_bound(),
                    )?;
                }
                if tag == 14 && matches!(self.version, 23..=40) {
                    let sectors = if matches!(self.version, 31..=40) {
                        Some(u32::from_le_bytes(number(self.required("sectors.count")?)?))
                    } else {
                        None
                    };
                    validate_weapon_event(
                        event,
                        matches!(self.version, 24..=40),
                        matches!(self.version, 39 | 40),
                        self.slot_bound(),
                        sectors,
                    )?;
                }
                if tag == 15 && matches!(self.version, 29..=40) {
                    validate_projectile_event(
                        event,
                        self.slot_bound(),
                        matches!(self.version, 39 | 40),
                        self.version == 40,
                    )?;
                }
                // Validate identity-bearing payloads without interpreting nested
                // untrusted collision/sight counts or allocating from them.
                let reference = match event.get(..2) {
                    Some([12, 0]) => Some((14, true)),
                    Some([12, 1] | [16, 8] | [13, 6] | [13, 7]) => Some((14, false)),
                    Some([18, 3]) => Some((15, false)),
                    _ => None,
                };
                if let Some((offset, is_type)) = reference {
                    let value = u16::from_le_bytes(number(
                        event
                            .get(offset..offset + 2)
                            .ok_or(DiagnosticError::Structure)?,
                    )?);
                    if is_type {
                        self.identity(value, u16::MAX)?;
                    } else if matches!(self.version, 21..=40) {
                        let state = OriginalState::try_from(value)
                            .map_err(|_| DiagnosticError::Identity)?;
                        // Entering S_NULL is a source removal event, not a live
                        // executable state. Real pistol impact thinkers expose
                        // this sentinel in23; historical readers stay unchanged.
                        if matches!(self.version, 23..=40)
                            && state == OriginalState::S_NULL
                            && event.get(..2) == Some(&[12, 1])
                        {
                            continue;
                        }
                        // DSIM35: a corpse crunched to giblets enters S_GIBS.
                        if matches!(self.version, 35..=40) && state == OriginalState::S_GIBS {
                            continue;
                        }
                        // Any admitted normal role; never original alias interpretation.
                        if ![
                            OriginalActorType::MT_SERGEANT,
                            OriginalActorType::MT_POSSESSED,
                            OriginalActorType::MT_SHOTGUY,
                            OriginalActorType::MT_CHAINGUY,
                            OriginalActorType::MT_TROOP,
                            OriginalActorType::MT_TROOPSHOT,
                            OriginalActorType::MT_PUFF,
                            OriginalActorType::MT_BLOOD,
                            OriginalActorType::MT_PLAYER,
                            OriginalActorType::MT_CLIP,
                            OriginalActorType::MT_MISC17,
                            OriginalActorType::MT_MISC22,
                            OriginalActorType::MT_MISC23,
                            OriginalActorType::MT_MISC10,
                            OriginalActorType::MT_MISC11,
                            OriginalActorType::MT_SHOTGUN,
                            OriginalActorType::MT_CHAINGUN,
                            OriginalActorType::MT_MISC0,
                            OriginalActorType::MT_MISC1,
                            OriginalActorType::MT_MISC2,
                            OriginalActorType::MT_MISC3,
                            // The DSIM36 decorations (admitted by their policy alone).
                            OriginalActorType::MT_MISC31,
                            OriginalActorType::MT_MISC48,
                            OriginalActorType::MT_MISC50,
                            OriginalActorType::MT_MISC71,
                            OriginalActorType::MT_MISC62,
                            OriginalActorType::MT_MISC68,
                            OriginalActorType::MT_MISC69,
                            // The DSIM37 barrel (admitted by its policy alone).
                            OriginalActorType::MT_BARREL,
                            // The DSIM39 pickups and decorations (likewise).
                            OriginalActorType::MT_MISC4,
                            OriginalActorType::MT_MISC5,
                            OriginalActorType::MT_MISC6,
                            OriginalActorType::MT_MISC12,
                            OriginalActorType::MT_INS,
                            OriginalActorType::MT_MISC14,
                            OriginalActorType::MT_MISC15,
                            OriginalActorType::MT_MISC16,
                            OriginalActorType::MT_MISC24,
                            OriginalActorType::MT_MISC18,
                            OriginalActorType::MT_MISC19,
                            OriginalActorType::MT_MISC49,
                            OriginalActorType::MT_MISC43,
                            // The DSIM39 weapons, rocket and Spectre (M4a step 6).
                            OriginalActorType::MT_MISC26,
                            OriginalActorType::MT_MISC27,
                            OriginalActorType::MT_ROCKET,
                            OriginalActorType::MT_SHADOWS,
                            // The DSIM40 teleport fog (admitted by its policy alone).
                            OriginalActorType::MT_TFOG,
                            // The DSIM40 Baron of Hell and its fireball (likewise).
                            OriginalActorType::MT_BRUISER,
                            OriginalActorType::MT_BRUISERSHOT,
                        ]
                        .into_iter()
                        .any(|ty| {
                            let slot = if ty == OriginalActorType::MT_PLAYER && self.version == 21 {
                                original_registry::body_slot()
                            } else {
                                ActorTypeId(ty.ordinal())
                            };
                            original_registry::state_role_for(
                                self.policy(),
                                slot,
                                StateId(state.ordinal()),
                            )
                            .is_ok()
                        }) {
                            return Err(DiagnosticError::Identity);
                        }
                    }
                }
            }
        }
        if matches!(self.version, 22..=40) {
            self.validate_player()?;
        }
        if self.version == 23 {
            self.validate_pistol()?;
        }
        if matches!(self.version, 24..=40) {
            self.validate_weapons()?;
        }
        if matches!(self.version, 26..=40) {
            let kind = self.required("original_player.0.armor.type")?;
            if kind.len() != 1 {
                return Err(DiagnosticError::Structure);
            }
            diagnostic_armor(
                kind[0],
                self.required("original_player.0.armor.points")?,
                self.policy(),
            )?;
        }
        if matches!(self.version, 27..=40) {
            self.validate_bonus_statistics()?;
        }
        if self.version == 34 {
            self.validate_specials()?;
        }
        if matches!(self.version, 35..=40) {
            self.validate_movers()?;
        }
        if matches!(self.version, 36..=40) {
            self.validate_decorations()?;
        }
        if matches!(self.version, 37..=40) {
            self.validate_barrels()?;
        }
        if matches!(self.version, 38..=40) {
            self.validate_level_body()?;
        }
        if matches!(self.version, 39 | 40) {
            self.validate_episode1()?;
        }
        if matches!(self.version, 29..=40) {
            self.validate_live_projectiles()?;
        }
        if matches!(self.version, 25..=40) {
            self.validate_pickup_items()?;
            if self.required("original_player.0.pickups.present")? != [1] {
                return Err(DiagnosticError::Structure);
            }
            let bonus = i32::from_le_bytes(number(
                self.required("original_player.0.pickups.bonus_count")?,
            )?);
            let items = i32::from_le_bytes(number(
                self.required("original_player.0.pickups.item_count")?,
            )?);
            let message = self.required("original_player.0.pickups.message")?;
            if !(0..=bonus_bound_for(self.version, self.e1())).contains(&bonus)
                || if matches!(self.version, 27..=40) {
                    !(0..=pickup_bound_for(self.version, self.e1())).contains(&items)
                } else {
                    items != 0
                }
                || !(matches!(message, [0..=6] | [8..=9])
                    || (matches!(self.version, 26..=40) && matches!(message, [10..=11]))
                    || (matches!(self.version, 27..=40) && matches!(message, [12..=13]))
                    || (matches!(self.version, 39 | 40) && matches!(message, [10..=30])))
            {
                return Err(DiagnosticError::Structure);
            }
        }
        Ok(())
    }
    /// DSIM34 fields: the shared order (every actor of `thinker.order` in the same
    /// relative order plus every occupied sector thinker, each once), the
    /// sector-thinker store (kinds, sectors, fields by kind), the runtime sector
    /// light and special widths and the secret count within `level.total_secrets`.
    ///
    /// Cross-checks against the encoded `map.sectors` lump: each sector thinker is
    /// the one thinker of a distinct light sector (map special 1, 2, 3, 4, 8, 12, 13
    /// or 17) of its kind; after the load operation `slot_count` is the number of
    /// light sectors (zero before it, only at tic 0); a light sector's runtime
    /// special is then 0 (4 for special 4), and every other runtime special is the
    /// map's, or 0 for a found secret (9).
    fn validate_specials(&self) -> Result<(), DiagnosticError> {
        let sectors = u32::from_le_bytes(number(self.required("sectors.count")?)?);
        let slots = u64::from_le_bytes(number(self.required("sector_thinkers.slot_count")?)?);
        if slots > u64::from(sectors) {
            return Err(DiagnosticError::Limit);
        }
        let slots = usize::try_from(slots).map_err(|_| DiagnosticError::Limit)?;
        let map = self.required("map.sectors")?;
        if map.len() % 26 != 0 || u32::try_from(map.len() / 26) != Ok(sectors) {
            return Err(DiagnosticError::Structure);
        }
        let map_special =
            |sector: usize| i16::from_le_bytes([map[26 * sector + 22], map[26 * sector + 23]]);
        let mut claimed = reserve::<bool>(map.len() / 26)?;
        claimed.resize(map.len() / 26, false);
        let i32field = |name: &str| -> Result<i32, DiagnosticError> {
            Ok(i32::from_le_bytes(number(self.required(name)?)?))
        };
        let mut occupied = Vec::new();
        for slot in 0..slots {
            let prefix = format!("sector_thinker.{slot}");
            let generation =
                u64::from_le_bytes(number(self.required(&format!("{prefix}.generation"))?)?);
            if generation == 0 {
                return Err(DiagnosticError::Identity);
            }
            match self.required(&format!("{prefix}.occupied"))? {
                [0] => continue,
                [1] => {}
                _ => return Err(DiagnosticError::Structure),
            }
            occupied.push((slot, generation));
            let kind = self.required(&format!("{prefix}.kind"))?;
            let sector = u16::from_le_bytes(number(self.required(&format!("{prefix}.sector"))?)?);
            if u32::from(sector) >= sectors {
                return Err(DiagnosticError::Identity);
            }
            // P_SpawnSpecials spawns one thinker per light sector, of its kind.
            let claim = &mut claimed[usize::from(sector)];
            if *claim || light_kind(map_special(usize::from(sector))) != kind.first().copied() {
                return Err(DiagnosticError::Identity);
            }
            *claim = true;
            i32field(&format!("{prefix}.minlight"))?;
            i32field(&format!("{prefix}.maxlight"))?;
            let count = self.field(&format!("{prefix}.count"));
            match kind {
                [1] => {
                    i32field(&format!("{prefix}.count"))?;
                    if i32field(&format!("{prefix}.maxtime"))? != 64
                        || i32field(&format!("{prefix}.mintime"))? != 7
                    {
                        return Err(DiagnosticError::Identity);
                    }
                }
                [2] => {
                    i32field(&format!("{prefix}.count"))?;
                    if !matches!(i32field(&format!("{prefix}.darktime"))?, 15 | 35)
                        || i32field(&format!("{prefix}.brighttime"))? != 5
                    {
                        return Err(DiagnosticError::Identity);
                    }
                }
                [3] => {
                    if count.is_some()
                        || !matches!(i32field(&format!("{prefix}.direction"))?, -1 | 1)
                    {
                        return Err(DiagnosticError::Identity);
                    }
                }
                [4] => {
                    i32field(&format!("{prefix}.count"))?;
                }
                _ => return Err(DiagnosticError::Identity),
            }
        }
        let order = self.required("thinker.shared_order")?;
        if order.len() % 13 != 0 {
            return Err(DiagnosticError::Structure);
        }
        // In this policy every sector thinker is created by the one load operation
        // after the map actors and the body, never removed: every slot is
        // occupied at generation 1, and the shared order holds them as one block
        // in slot (creation) order after at least one actor.
        if occupied.len() != slots || occupied.iter().any(|(_, g)| *g != 1) {
            return Err(DiagnosticError::Identity);
        }
        let mut actors = Vec::new();
        let mut next = 0;
        let mut previous = None;
        for (index, entry) in order.chunks_exact(13).enumerate() {
            match entry[0] {
                0 => {
                    validate_handle(&entry[1..], self.slot_bound())?;
                    actors.extend_from_slice(&entry[1..]);
                }
                1 => {
                    let slot = u32::from_le_bytes(number(&entry[1..5])?);
                    let generation = u64::from_le_bytes(number(&entry[5..13])?);
                    let contiguous = if next == 0 {
                        index > 0
                    } else {
                        previous.is_some() && previous == index.checked_sub(1)
                    };
                    if usize::try_from(slot) != Ok(next) || generation != 1 || !contiguous {
                        return Err(DiagnosticError::Identity);
                    }
                    next += 1;
                    previous = Some(index);
                }
                _ => return Err(DiagnosticError::Identity),
            }
        }
        if next != slots || actors != self.required("thinker.order")? {
            return Err(DiagnosticError::Identity);
        }
        let lights = (0..map.len() / 26)
            .filter(|&i| light_kind(map_special(i)).is_some())
            .count();
        let loaded = slots != 0;
        if (loaded && slots != lights)
            || (!loaded && lights != 0 && u64::from_le_bytes(number(self.required("tic")?)?) != 0)
        {
            return Err(DiagnosticError::Identity);
        }
        for i in 0..map.len() / 26 {
            number::<2>(self.required(&format!("sector.{i}.light"))?)?;
            let special =
                i16::from_le_bytes(number(self.required(&format!("sector.{i}.special"))?)?);
            let original = map_special(i);
            let expected = if loaded && light_kind(original).is_some() {
                special == if original == 4 { 4 } else { 0 }
            } else {
                special == original
                    || (original == 9 && special == 0)
                    // DSIM39: a raiseToNearestAndChange plat and a finished
                    // donut ring set the special to 0.
                    //
                    // Dead in this function: `validate_specials` runs for version
                    // 34 only (the call in `validate`), so `matches!(self.version,
                    // 39 | 40)` is false here and the mutant `special == 0` →
                    // `special != 0` of Mutants run 43543402068 (this clause) is
                    // equivalent. The live copy is `validate_movers`' (versions
                    // 35–40), pinned by `tests/episode1v2_reader_domains.rs`.
                    || (matches!(self.version, 39 | 40) && special == 0)
            };
            if !expected {
                return Err(DiagnosticError::Identity);
            }
        }
        let total = i32field("level.total_secrets")?;
        if !(0..=total).contains(&i32field("original_player.0.secret_count")?) {
            return Err(DiagnosticError::Identity);
        }
        Ok(())
    }
    /// DSIM35 fields (STATE_FORMAT "DSIM v35"), cross-checked against the encoded
    /// map lumps.
    ///
    /// The sector-thinker store: light records (kinds 1–4, generation 1, one per
    /// distinct light sector of its kind, as in DSIM34), door/plat/floor records
    /// (kinds 5–7) with their field domains, removed movers (kind 8, sector only) and
    /// free slots; at most one live mover names a sector. The shared order lists
    /// every actor of `thinker.order` in the same relative order and every occupied
    /// sector-thinker slot exactly once at its generation. Sector light and special
    /// fields as in DSIM34. `line.{i}.special` exists exactly for the lines whose
    /// map special is nonzero and holds the map's value or, on a W1 (36, 38, 109) or
    /// S1 (11) line, 0. The 16 `buttonlist` slots: a zero `btimer` and nothing else,
    /// or a timer 1..=35 on a distinct SR 63 line whose `soundorg` is that line's
    /// front sector. `use_down` and `level.exit` are booleans; an `S_GIBS` actor has
    /// height and radius 0 and `MF_SOLID` clear.
    fn validate_movers(&self) -> Result<(), DiagnosticError> {
        let sectors = u32::from_le_bytes(number(self.required("sectors.count")?)?);
        let map = self.required("map.sectors")?;
        if map.len() % 26 != 0 || u32::try_from(map.len() / 26) != Ok(sectors) {
            return Err(DiagnosticError::Structure);
        }
        let count = map.len() / 26;
        let map_special =
            |sector: usize| i16::from_le_bytes([map[26 * sector + 22], map[26 * sector + 23]]);
        // Lights take at most one slot per sector and movers at most two (a removed
        // entry beside its replacement).
        let slots = u64::from_le_bytes(number(self.required("sector_thinkers.slot_count")?)?);
        if slots > 3 * u64::from(sectors) {
            return Err(DiagnosticError::Limit);
        }
        let slots = usize::try_from(slots).map_err(|_| DiagnosticError::Limit)?;
        let i32field = |name: &str| -> Result<i32, DiagnosticError> {
            Ok(i32::from_le_bytes(number(self.required(name)?)?))
        };
        let mut light_claimed = reserve::<bool>(count)?;
        light_claimed.resize(count, false);
        let mut mover_claimed = reserve::<bool>(count)?;
        mover_claimed.resize(count, false);
        let mut occupied = reserve::<Option<u64>>(slots)?;
        let mut lights = 0;
        for slot in 0..slots {
            let prefix = format!("sector_thinker.{slot}");
            let generation =
                u64::from_le_bytes(number(self.required(&format!("{prefix}.generation"))?)?);
            if generation == 0 {
                return Err(DiagnosticError::Identity);
            }
            match self.required(&format!("{prefix}.occupied"))? {
                [0] => {
                    occupied.push(None);
                    continue;
                }
                [1] => occupied.push(Some(generation)),
                _ => return Err(DiagnosticError::Structure),
            }
            let kind = number::<1>(self.required(&format!("{prefix}.kind"))?)?[0];
            let sector = u16::from_le_bytes(number(self.required(&format!("{prefix}.sector"))?)?);
            let sector = usize::from(sector);
            if sector >= count {
                return Err(DiagnosticError::Identity);
            }
            match kind {
                1..=4 => {
                    let claim = &mut light_claimed[sector];
                    if generation != 1 || *claim || light_kind(map_special(sector)) != Some(kind) {
                        return Err(DiagnosticError::Identity);
                    }
                    *claim = true;
                    lights += 1;
                    i32field(&format!("{prefix}.minlight"))?;
                    i32field(&format!("{prefix}.maxlight"))?;
                    let has_count = self.field(&format!("{prefix}.count")).is_some();
                    let ok = match kind {
                        1 => {
                            i32field(&format!("{prefix}.count"))?;
                            i32field(&format!("{prefix}.maxtime"))? == 64
                                && i32field(&format!("{prefix}.mintime"))? == 7
                        }
                        2 => {
                            i32field(&format!("{prefix}.count"))?;
                            matches!(i32field(&format!("{prefix}.darktime"))?, 15 | 35)
                                && i32field(&format!("{prefix}.brighttime"))? == 5
                        }
                        3 => {
                            !has_count
                                && matches!(i32field(&format!("{prefix}.direction"))?, -1 | 1)
                        }
                        _ => {
                            i32field(&format!("{prefix}.count"))?;
                            true
                        }
                    };
                    if !ok {
                        return Err(DiagnosticError::Identity);
                    }
                }
                5..=7 => {
                    let claim = &mut mover_claimed[sector];
                    if *claim {
                        return Err(DiagnosticError::Identity);
                    }
                    *claim = true;
                    let byte = |name: &str| -> Result<u8, DiagnosticError> {
                        Ok(number::<1>(self.required(&format!("{prefix}.{name}"))?)?[0])
                    };
                    let ok = match kind {
                        5 => {
                            (matches!(byte("type")?, 0 | 6)
                                || (matches!(self.version, 39 | 40)
                                    && matches!(byte("type")?, 1 | 3)))
                                && (-1..=1).contains(&i32field(&format!("{prefix}.direction"))?)
                                && i32field(&format!("{prefix}.topcountdown"))? >= 0
                                && i32field(&format!("{prefix}.topheight")).is_ok()
                        }
                        6 => {
                            byte("status")? <= 2
                                && i32field(&format!("{prefix}.count"))? >= 0
                                // A raiseToNearestAndChange plat (DSIM39 type 3)
                                // never writes `low` (0); `validate_episode1`.
                                && (i32field(&format!("{prefix}.low"))?
                                    <= i32field(&format!("{prefix}.high"))?
                                    || (matches!(self.version, 39 | 40)
                                        && self.field(&format!("{prefix}.type")) == Some(&[3][..])))
                        }
                        _ => {
                            (matches!(byte("type")?, 1 | 2)
                                || (matches!(self.version, 39 | 40)
                                    && matches!(byte("type")?, 0 | 3 | 4 | 11)))
                                && i32field(&format!("{prefix}.floordestheight")).is_ok()
                        }
                    };
                    if !ok {
                        return Err(DiagnosticError::Identity);
                    }
                }
                8 => {}
                _ => return Err(DiagnosticError::Identity),
            }
        }
        let order = self.required("thinker.shared_order")?;
        if order.len() % 13 != 0 {
            return Err(DiagnosticError::Structure);
        }
        let mut seen = reserve::<bool>(slots)?;
        seen.resize(slots, false);
        let mut actors = Vec::new();
        for entry in order.chunks_exact(13) {
            match entry[0] {
                0 => {
                    validate_handle(&entry[1..], self.slot_bound())?;
                    actors.extend_from_slice(&entry[1..]);
                }
                1 => {
                    let slot = usize::try_from(u32::from_le_bytes(number(&entry[1..5])?))
                        .map_err(|_| DiagnosticError::Identity)?;
                    let generation = u64::from_le_bytes(number(&entry[5..13])?);
                    if slot >= slots || seen[slot] || occupied[slot] != Some(generation) {
                        return Err(DiagnosticError::Identity);
                    }
                    seen[slot] = true;
                }
                _ => return Err(DiagnosticError::Identity),
            }
        }
        if occupied
            .iter()
            .zip(&seen)
            .any(|(slot, seen)| slot.is_some() != *seen)
            || actors != self.required("thinker.order")?
        {
            return Err(DiagnosticError::Identity);
        }
        let expected_lights = (0..count)
            .filter(|&i| light_kind(map_special(i)).is_some())
            .count();
        let loaded = slots != 0;
        if (loaded && lights != expected_lights)
            || (!loaded
                && expected_lights != 0
                && u64::from_le_bytes(number(self.required("tic")?)?) != 0)
        {
            return Err(DiagnosticError::Identity);
        }
        for i in 0..count {
            number::<2>(self.required(&format!("sector.{i}.light"))?)?;
            let special =
                i16::from_le_bytes(number(self.required(&format!("sector.{i}.special"))?)?);
            let original = map_special(i);
            let expected = if loaded && light_kind(original).is_some() {
                special == if original == 4 { 4 } else { 0 }
            } else {
                special == original
                    || (original == 9 && special == 0)
                    // DSIM39: a raiseToNearestAndChange plat and a finished
                    // donut ring set the special to 0.
                    || (matches!(self.version, 39 | 40) && special == 0)
            };
            if !expected {
                return Err(DiagnosticError::Identity);
            }
        }
        let total = i32field("level.total_secrets")?;
        if !(0..=total).contains(&i32field("original_player.0.secret_count")?) {
            return Err(DiagnosticError::Identity);
        }
        // Runtime line specials: exactly the lines with a nonzero map special.
        let linedefs = self.required("map.linedefs")?;
        let sidedefs = self.required("map.sidedefs")?;
        if linedefs.len() % 14 != 0 || sidedefs.len() % 30 != 0 {
            return Err(DiagnosticError::Structure);
        }
        let line_count = linedefs.len() / 14;
        let line_word = |line: usize, at: usize| {
            u16::from_le_bytes([linedefs[14 * line + at], linedefs[14 * line + at + 1]])
        };
        let mut encoded = 0;
        for line in 0..line_count {
            let original = line_word(line, 6) as i16;
            let field = self.field(&format!("line.{line}.special"));
            if original == 0 {
                if field.is_some() {
                    return Err(DiagnosticError::Structure);
                }
                continue;
            }
            encoded += 1;
            let special = i16::from_le_bytes(number(field.ok_or(DiagnosticError::Structure)?)?);
            // DSIM39 adds the Episode 1 W1, S1 and D1 lines (decision H1).
            let clearable = matches!(original, 11 | 36 | 38 | 109)
                || (matches!(self.version, 39 | 40)
                    && matches!(
                        original,
                        2 | 5 | 7 | 8 | 9 | 16 | 18 | 20 | 22 | 23 | 31..=35 | 51 | 103
                    ));
            if special != original && !(special == 0 && clearable) {
                return Err(DiagnosticError::Identity);
            }
        }
        if self
            .fields
            .iter()
            .filter(|f| f.path.starts_with("line."))
            .count()
            != encoded
        {
            return Err(DiagnosticError::Structure);
        }
        // The buttonlist (decision N9).
        let mut pressed = Vec::new();
        for k in 0..crate::mover_rules::MAX_BUTTONS {
            let timer = i32field(&format!("button.{k}.btimer"))?;
            let line = self.field(&format!("button.{k}.line"));
            let origin = self.field(&format!("button.{k}.soundorg"));
            if timer == 0 {
                if line.is_some() || origin.is_some() {
                    return Err(DiagnosticError::Structure);
                }
                continue;
            }
            let line = usize::from(u16::from_le_bytes(number(
                line.ok_or(DiagnosticError::Structure)?,
            )?));
            let origin = u16::from_le_bytes(number(origin.ok_or(DiagnosticError::Structure)?)?);
            if !(1..=35).contains(&timer)
                || line >= line_count
                || !(line_word(line, 6) == 63
                    || (matches!(self.version, 39 | 40)
                        && matches!(line_word(line, 6), 46 | 62 | 70)))
                || pressed.contains(&line)
            {
                return Err(DiagnosticError::Identity);
            }
            let side = usize::from(line_word(line, 10));
            if 30 * side + 30 > sidedefs.len()
                || u16::from_le_bytes([sidedefs[30 * side + 28], sidedefs[30 * side + 29]])
                    != origin
            {
                return Err(DiagnosticError::Identity);
            }
            pressed.push(line);
        }
        if self
            .fields
            .iter()
            .filter(|f| f.path.starts_with("button."))
            .count()
            != crate::mover_rules::MAX_BUTTONS + 2 * pressed.len()
        {
            return Err(DiagnosticError::Structure);
        }
        if !matches!(self.required("original_player.0.use_down")?, [0 | 1])
            || !matches!(self.required("level.exit")?, [0 | 1])
        {
            return Err(DiagnosticError::Structure);
        }
        // Giblets: PIT_ChangeSector's S_GIBS has height and radius 0, MF_SOLID clear.
        // Type-keyed: MT_MISC71 (DSIM36) spawns in S_GIBS with height 16, radius 20
        // and health 1000 (info.c:4255–4276) and is checked by `validate_decorations`.
        for slot in 0..self.slots()? {
            let prefix = format!("actor.{slot}");
            if self.required(&format!("{prefix}.occupied"))? != [1] {
                continue;
            }
            let ty = ActorTypeId(u16::from_le_bytes(number(
                self.required(&format!("{prefix}.type"))?,
            )?));
            if e1m1_rules::is_decoration(self.policy(), ty) {
                continue;
            }
            let state = u16::from_le_bytes(number(self.required(&format!("{prefix}.state"))?)?);
            if state == OriginalState::S_GIBS.ordinal()
                && (i32field(&format!("{prefix}.height"))? != 0
                    || i32field(&format!("{prefix}.radius"))? != 0
                    || i32field(&format!("{prefix}.health"))? > 0
                    || u32::from_le_bytes(number(self.required(&format!("{prefix}.flags"))?)?)
                        & crate::original_player::MF_SOLID
                        != 0)
            {
                return Err(DiagnosticError::Identity);
            }
        }
        Ok(())
    }
    /// DSIM36: every occupied actor of one of the seven decoration types
    /// (`E1M1_DECORATIONS`, M3h9 note §2.5) is the record `P_SpawnMapThing` made and
    /// nothing changed since (§2.5–§2.7): its spawn state with that state's sprite
    /// and frame, `tics` −1, health 1000, the type's radius and height, its flags
    /// with at most `MF_AMBUSH` added, no momentum, `z == floorz`, no target, the
    /// spawn reaction time and zero `threshold`, `movedir` and `movecount`, no
    /// player, not pending removal, and a spawnpoint whose DoomEd number is the
    /// type's and whose position is the actor's, with the angle
    /// `ANG45 * (angle/45)` of that record (p_mobj.c:794), `MF_AMBUSH` set exactly
    /// when its options carry `MTF_AMBUSH` (795–796), `lastlook` below
    /// `MAXPLAYERS` (the one `P_Random () % MAXPLAYERS` draw, 506) and no pending
    /// death request; at most `MAX_DECORATIONS` of them,
    /// and their spawnpoints, read in `thinker.order`, a subsequence of the THINGS
    /// lump (`P_SpawnMapThing` spawns in lump order and a decoration is never
    /// removed, so the decoration subsequence of the thinker list is the lump
    /// order for the whole level).
    /// The decoration type of `ty` this version admits: the seven of E1M1, and
    /// from DSIM39 the candle and the firestick.
    fn decoration(&self, ty: ActorTypeId) -> Option<crate::identity::OriginalActorType> {
        if matches!(self.version, 39 | 40) {
            e1m1_rules::listed_decoration(ty)
        } else {
            e1m1_rules::decoration(ty)
        }
    }
    fn validate_decorations(&self) -> Result<(), DiagnosticError> {
        // The E1M1 policy is medium skill, single player (M3h9 note §6, decision
        // E4; owner decision of the independent review): `actors.options` must read
        // skill 2 (`sk_medium`). Only this reader checks it; every reader before
        // DSIM36 keeps its historical contract and leaves the skill unchecked.
        // DSIM40 (wire 24, M4b decision I4) also admits skill 3 (`sk_hard`).
        let skill = self.required("actors.options")?.first().copied();
        let admitted = skill == Some(crate::Skill::Medium as u8)
            || (self.version == 40 && skill == Some(crate::Skill::Hard as u8));
        if !admitted {
            return Err(DiagnosticError::Identity);
        }
        let mut count = 0;
        for slot in 0..self.slots()? {
            let prefix = format!("actor.{slot}");
            if self.required(&format!("{prefix}.occupied"))? != [1] {
                continue;
            }
            let ty = ActorTypeId(u16::from_le_bytes(number(
                self.required(&format!("{prefix}.type"))?,
            )?));
            let Some(decoration) = self.decoration(ty) else {
                continue;
            };
            count += 1;
            if count > self.e1().map_or(MAX_DECORATIONS, |b| b.statics) {
                return Err(DiagnosticError::Limit);
            }
            let d = decoration.descriptor();
            // The firestick (DSIM39) may hold any state of its spawn loop.
            let state = OriginalState::try_from(u16::from_le_bytes(number(
                self.required(&format!("{prefix}.state"))?,
            )?))
            .map_err(|_| DiagnosticError::Identity)?;
            // Redundant with the generic per-actor check that ran first
            // (`identity` → `state_role_for` → `e1m1_rules::spawn_state`, the same
            // `decoration_states` predicate), so only a loop state reaches here and
            // the mutant `||` → `&&` (Mutants run 43543402068) is equivalent:
            // `tests/episode1v2_reader_domains.rs`
            // (`a_decoration_holds_a_state_of_its_own_loop_only`) sweeps the states.
            if !e1m1_rules::decoration_states(decoration).contains(&state)
                || (!matches!(self.version, 39 | 40) && state != d.states.spawn)
            {
                return Err(DiagnosticError::Identity);
            }
            let s = state.descriptor();
            let i32field = |name: &str| -> Result<i32, DiagnosticError> {
                Ok(i32::from_le_bytes(number(
                    self.required(&format!("{prefix}.{name}"))?,
                )?))
            };
            let u16field = |name: &str| -> Result<u16, DiagnosticError> {
                Ok(u16::from_le_bytes(number(
                    self.required(&format!("{prefix}.{name}"))?,
                )?))
            };
            let flags = u32::from_le_bytes(number(self.required(&format!("{prefix}.flags"))?)?);
            let spawn = self.required(&format!("{prefix}.spawnpoint"))?;
            if u16field("sprite")? != s.sprite.ordinal()
                || u32::from_le_bytes(number(self.required(&format!("{prefix}.frame"))?)?)
                    != s.frame
                || !e1m1_rules::decoration_tics(state, i32field("tics")?)
                || i32field("health")? != d.spawn_health
                || i32field("radius")? != d.radius
                || i32field("height")? != d.height
                || flags & !(d.flags | world_pickups::MF_AMBUSH) != 0
                || flags & d.flags != d.flags
                || i32field("momx")? != 0
                || i32field("momy")? != 0
                || i32field("momz")? != 0
                || i32field("z")? != i32field("floor")?
                || i32field("reaction_time")? != d.reaction_time
                || i32field("threshold")? != 0
                || self.required(&format!("{prefix}.movedir"))? != [0]
                || i32field("movecount")? != 0
                || self.required(&format!("{prefix}.player"))? != [255]
                || self.required(&format!("{prefix}.pending_removal"))? != [0]
                || self.required(&format!("{prefix}.target"))? != [0; 13]
                || spawn.len() != 11
                || spawn[0] != 1
                || i32::from(i16::from_le_bytes([spawn[7], spawn[8]])) != d.doomed_number
                || i32::from(i16::from_le_bytes([spawn[1], spawn[2]])) * 65536 != i32field("x")?
                || i32::from(i16::from_le_bytes([spawn[3], spawn[4]])) * 65536 != i32field("y")?
                || u32::from_le_bytes(number(self.required(&format!("{prefix}.angle"))?)?)
                    != e1m1_rules::spawn_angle(i16::from_le_bytes([spawn[5], spawn[6]]))
                || (flags & world_pickups::MF_AMBUSH != 0)
                    != e1m1_rules::spawn_ambush(u16::from_le_bytes([spawn[9], spawn[10]]))
                || !matches!(self.required(&format!("{prefix}.lastlook"))?, [0..=3])
                || self.required(&format!("{prefix}.death_requested"))? != [0]
            {
                return Err(DiagnosticError::Identity);
            }
        }
        // Lump order: each decoration of the thinker list matches the next unused
        // THINGS record with its spawnpoint bytes (x, y, angle, type, options).
        let things = self.required("map.things")?;
        let mut records = things.chunks_exact(10);
        for handle in self.required("thinker.order")?.chunks_exact(12) {
            let slot = u32::from_le_bytes(number(&handle[..4])?) as usize;
            let prefix = format!("actor.{slot}");
            let ty = ActorTypeId(u16::from_le_bytes(number(
                self.required(&format!("{prefix}.type"))?,
            )?));
            if self.decoration(ty).is_none() {
                continue;
            }
            let spawn = &self.required(&format!("{prefix}.spawnpoint"))?[1..];
            if !records.any(|record| record == spawn) {
                return Err(DiagnosticError::Identity);
            }
        }
        Ok(())
    }
    /// DSIM37: every occupied actor of type `MT_BARREL` (M3h9b note §2.1–§2.6,
    /// decision F9) holds what the source can give it. Pending removal only after
    /// its own `S_NULL` (state sentinel `u16::MAX`, `tics` 0, health at most 0,
    /// the dying flags and height 10.5). Otherwise: a state
    /// of `S_BAR1`, `S_BAR2`, `S_BEXP..S_BEXP5` (with that state's sprite and frame
    /// and a countdown in `1..=tics`) or `S_GIBS` after a crunch (`tics` −1);
    /// health at most 20, positive exactly in `S_BAR1`/`S_BAR2`; the info flags
    /// (`MF_SOLID|MF_SHOOTABLE|MF_NOBLOOD`) alive, `MF_SHOOTABLE` cleared and
    /// `MF_CORPSE|MF_DROPOFF` set once dead (p_inter.c:675–680), `MF_SOLID` cleared
    /// only by the crunch (p_map.c:1274), with `MF_AMBUSH` exactly when the THING
    /// record carries `MTF_AMBUSH`; radius 10 (0 gibbed) and height 42 alive, 10.5
    /// dead (`height >>= 2`, 681), 0 gibbed; reaction time 8, or 0 after a
    /// non-lethal wound (`P_DamageMobj`, p_inter.c:902); threshold 0, or 100 with a
    /// target (the retarget block, 904–915; nothing lowers a barrel's threshold);
    /// no `movedir`, `movecount`, player or death request; `lastlook` below
    /// `MAXPLAYERS`; a 2035 spawnpoint with the angle `ANG45 * (angle/45)` of its
    /// record. At most `MAX_BARRELS` of them, their spawnpoints in `thinker.order`
    /// a subsequence of the THINGS lump.
    fn validate_barrels(&self) -> Result<(), DiagnosticError> {
        use OriginalState as S;
        let d = OriginalActorType::MT_BARREL.descriptor();
        let dead_flags = (d.flags & !crate::original_player::MF_SHOOTABLE)
            | crate::original_player::MF_CORPSE
            | crate::original_player::MF_DROPOFF;
        let mut count = 0;
        for slot in 0..self.slots()? {
            let prefix = format!("actor.{slot}");
            if self.required(&format!("{prefix}.occupied"))? != [1] {
                continue;
            }
            let u16field = |name: &str| -> Result<u16, DiagnosticError> {
                Ok(u16::from_le_bytes(number(
                    self.required(&format!("{prefix}.{name}"))?,
                )?))
            };
            if !crate::barrels::is_barrel_type(ActorTypeId(u16field("type")?)) {
                continue;
            }
            count += 1;
            if count > self.e1().map_or(crate::MAX_BARRELS, |b| b.barrels) {
                return Err(DiagnosticError::Limit);
            }
            let raw = u16field("state")?;
            let i32field = |name: &str| -> Result<i32, DiagnosticError> {
                Ok(i32::from_le_bytes(number(
                    self.required(&format!("{prefix}.{name}"))?,
                )?))
            };
            match self.required(&format!("{prefix}.pending_removal"))? {
                [0] => {}
                // Removed by its own `S_NULL` this tic (p_mobj.c:62–66): still in the
                // thinker list until the next pass, with the dying flags and height.
                [1] if raw == u16::MAX
                    && i32field("tics")? == 0
                    && i32field("health")? <= 0
                    && u32::from_le_bytes(number(self.required(&format!("{prefix}.flags"))?)?)
                        & !world_pickups::MF_AMBUSH
                        == dead_flags
                    && i32field("height")? == d.height >> 2 =>
                {
                    continue;
                }
                _ => return Err(DiagnosticError::Identity),
            }
            let state = S::try_from(raw).map_err(|_| DiagnosticError::Identity)?;
            let gibbed = state == S::S_GIBS;
            if !gibbed && !crate::barrels::state_allowed(state) {
                return Err(DiagnosticError::Identity);
            }
            let alive = crate::barrels::live_state(state);
            let s = state.descriptor();
            let (flags, radius, height) = if gibbed {
                (dead_flags & !crate::original_player::MF_SOLID, 0, 0)
            } else if alive {
                (d.flags, d.radius, d.height)
            } else {
                (dead_flags, d.radius, d.height >> 2)
            };
            let tics = i32field("tics")?;
            let health = i32field("health")?;
            let actual = u32::from_le_bytes(number(self.required(&format!("{prefix}.flags"))?)?);
            let threshold = i32field("threshold")?;
            let target = self.required(&format!("{prefix}.target"))?;
            let spawn = self.required(&format!("{prefix}.spawnpoint"))?;
            if u16field("sprite")? != s.sprite.ordinal()
                || u32::from_le_bytes(number(self.required(&format!("{prefix}.frame"))?)?)
                    != s.frame
                || !(if gibbed {
                    tics == -1
                } else {
                    (1..=s.tics).contains(&tics)
                })
                || health > d.spawn_health
                || (health > 0) != alive
                || actual & !world_pickups::MF_AMBUSH != flags
                || i32field("radius")? != radius
                || i32field("height")? != height
                || !matches!(i32field("reaction_time")?, 0 | 8)
                || !(threshold == 0 || threshold == crate::original_player::BASETHRESHOLD)
                // p_inter.c:902 zeroes the reaction time before the retarget
                // block (904–911) sets the threshold: 100 implies 0.
                || (threshold == crate::original_player::BASETHRESHOLD
                    && i32field("reaction_time")? != 0)
                || target.first() != Some(&u8::from(threshold != 0))
                || (threshold == 0 && target != [0; 13])
                || self.required(&format!("{prefix}.movedir"))? != [0]
                || i32field("movecount")? != 0
                || self.required(&format!("{prefix}.player"))? != [255]
                || self.required(&format!("{prefix}.death_requested"))? != [0]
                || !matches!(self.required(&format!("{prefix}.lastlook"))?, [0..=3])
                || spawn.len() != 11
                || spawn[0] != 1
                || i32::from(i16::from_le_bytes([spawn[7], spawn[8]])) != d.doomed_number
                || u32::from_le_bytes(number(self.required(&format!("{prefix}.angle"))?)?)
                    != e1m1_rules::spawn_angle(i16::from_le_bytes([spawn[5], spawn[6]]))
                || (actual & world_pickups::MF_AMBUSH != 0)
                    != e1m1_rules::spawn_ambush(u16::from_le_bytes([spawn[9], spawn[10]]))
            {
                return Err(DiagnosticError::Identity);
            }
        }
        // Lump order: each barrel of the thinker list matches the next unused THINGS
        // record with its spawnpoint bytes (a barrel is spawned only from the lump).
        let things = self.required("map.things")?;
        let mut records = things.chunks_exact(10);
        for handle in self.required("thinker.order")?.chunks_exact(12) {
            let slot = u32::from_le_bytes(number(&handle[..4])?) as usize;
            let prefix = format!("actor.{slot}");
            let ty = ActorTypeId(u16::from_le_bytes(number(
                self.required(&format!("{prefix}.type"))?,
            )?));
            if !crate::barrels::is_barrel_type(ty) {
                continue;
            }
            let spawn = &self.required(&format!("{prefix}.spawnpoint"))?[1..];
            if !records.any(|record| record == spawn) {
                return Err(DiagnosticError::Identity);
            }
        }
        Ok(())
    }
    /// DSIM39 witnesses of the Episode 1 fields (plan §A.7): `store.max_slots` is
    /// exactly 1,024 (decision D1); `level.secret_exit`
    /// is 0 or 1 and implies `level.exit`; every `sector.N.floorpic` is a
    /// normalized flat name (upper-case ASCII bytes, then zero padding, at least
    /// one byte); every live plat carries a `type` of 1 or 3 (`low` 0 for 3),
    /// every live floor a `direction` of −1 or 1 and a positive `speed`, and
    /// exactly the `donutRaise` floors (type 11) carry `texture` and
    /// `newspecial`; the weapon maxima are the `G_PlayerReborn` values doubled
    /// once by `backpack` and the cell and rocket counts are within them; the
    /// power counters are in their source domains (no invulnerability or
    /// strength in Episode 1, invisibility and the suit at most 2,100 tics, the
    /// map 0 or 1, the visor at most 4,200); the skull keys are never held; and
    /// the player body carries `MF_SHADOW` exactly while invisible.
    fn validate_episode1(&self) -> Result<(), DiagnosticError> {
        let byte = |name: &str| -> Result<u8, DiagnosticError> {
            Ok(number::<1>(self.required(name)?)?[0])
        };
        let i32field = |name: &str| -> Result<i32, DiagnosticError> {
            Ok(i32::from_le_bytes(number(self.required(name)?)?))
        };
        if u64::from_le_bytes(number(self.required("store.slot_count")?)?)
            > u64::from(u32::from_le_bytes(number(
                self.required("store.max_slots")?,
            )?))
        {
            return Err(DiagnosticError::Limit);
        }
        // `store.max_slots` is exactly 1,024 under wire 23 (STATE_FORMAT "DSIM v39";
        // decision D1, M4A_REFERENCE_INVESTIGATION.md §14). The older readers keep
        // their historical upper-bound check only.
        if u32::from_le_bytes(number(self.required("store.max_slots")?)?)
            != crate::player_map_scale::EPISODE1_ACTOR_SLOTS
        {
            return Err(DiagnosticError::Limit);
        }
        let secret = byte("level.secret_exit")?;
        if secret > 1 || (secret == 1 && byte("level.exit")? != 1) {
            return Err(DiagnosticError::Structure);
        }
        // DSIM40: `level.map`, the Episode 1 map number (`gamemap`), is 1–9.
        if self.version == 40 && !crate::EPISODE1_MAP_NUMBERS.contains(&byte("level.map")?) {
            return Err(DiagnosticError::Structure);
        }
        // DSIM40: the telefrag's health domain. One `P_DamageMobj` of 10,000
        // (p_map.c:105) leaves a victim at no less than 1 − 10,000, and nothing
        // damages a corpse (p_inter.c:790–791), so every actor's health is at least
        // −9,999.
        if self.version == 40 {
            for slot in 0..self.slots()? {
                let prefix = format!("actor.{slot}");
                if self.required(&format!("{prefix}.occupied"))? == [1]
                    && i32field(&format!("{prefix}.health"))? < 1 - crate::teleport::TELEFRAG_DAMAGE
                {
                    return Err(DiagnosticError::Identity);
                }
            }
        }
        let sectors = u32::from_le_bytes(number(self.required("sectors.count")?)?);
        for i in 0..sectors {
            let name = number::<8>(self.required(&format!("sector.{i}.floorpic"))?)?;
            if !normalized_flat_name(&name) {
                return Err(DiagnosticError::Structure);
            }
        }
        let slots = u64::from_le_bytes(number(self.required("sector_thinkers.slot_count")?)?);
        for slot in 0..slots {
            let prefix = format!("sector_thinker.{slot}");
            if self.required(&format!("{prefix}.occupied"))? != [1] {
                continue;
            }
            match byte(&format!("{prefix}.kind"))? {
                6 => {
                    let ty = byte(&format!("{prefix}.type"))?;
                    if !matches!(ty, 1 | 3) || (ty == 3 && i32field(&format!("{prefix}.low"))? != 0)
                    {
                        return Err(DiagnosticError::Identity);
                    }
                }
                7 => {
                    let donut = byte(&format!("{prefix}.type"))? == 11;
                    let texture = self.field(&format!("{prefix}.texture"));
                    let newspecial = self.field(&format!("{prefix}.newspecial"));
                    if !matches!(i32field(&format!("{prefix}.direction"))?, -1 | 1)
                        || i32field(&format!("{prefix}.speed"))? <= 0
                        || texture.is_some() != donut
                        || newspecial.is_some() != donut
                    {
                        return Err(DiagnosticError::Identity);
                    }
                    if let (Some(texture), Some(newspecial)) = (texture, newspecial) {
                        if !normalized_flat_name(&number::<8>(texture)?) {
                            return Err(DiagnosticError::Structure);
                        }
                        number::<2>(newspecial)?;
                    }
                }
                _ => {}
            }
        }
        let prefix = "original_player.0.weapons";
        if self.required(&format!("{prefix}.present"))? == [1] {
            let backpack = byte("original_player.0.backpack")?;
            if backpack > 1 {
                return Err(DiagnosticError::Structure);
            }
            let word = |name: &str| -> Result<u16, DiagnosticError> {
                Ok(u16::from_le_bytes(number(
                    self.required(&format!("{prefix}.{name}"))?,
                )?))
            };
            let max = crate::weapons::AMMO_CAPACITIES.map(|m| m << backpack);
            for (i, m) in max.into_iter().enumerate() {
                if i32::from(word(&format!("max_ammo.{i}"))?) != m {
                    return Err(DiagnosticError::Identity);
                }
            }
            if i32::from(word("cells")?) > max[2] || i32::from(word("rockets")?) > max[3] {
                return Err(DiagnosticError::Identity);
            }
        } else if byte("original_player.0.backpack")? != 0 {
            return Err(DiagnosticError::Structure);
        }
        let limits = [0..=0, 0..=0, 0..=2100, 0..=2100, 0..=1, 0..=4200];
        let mut powers = [0; 6];
        for (i, range) in limits.into_iter().enumerate() {
            powers[i] = i32field(&format!("original_player.0.powers.{i}"))?;
            if !range.contains(&powers[i]) {
                return Err(DiagnosticError::Identity);
            }
        }
        for i in 0..6 {
            let card = byte(&format!("original_player.0.cards.{i}"))?;
            if card > 1 || (i >= 3 && card != 0) {
                return Err(DiagnosticError::Identity);
            }
        }
        let raw = self.required("original_player.0.mo")?;
        if raw.len() == 13 && raw[0] == 1 {
            let slot = u32::from_le_bytes(number(&raw[1..5])?);
            let prefix = format!("actor.{slot}");
            if self.required(&format!("{prefix}.occupied"))? == [1] {
                let flags = u32::from_le_bytes(number(self.required(&format!("{prefix}.flags"))?)?);
                if (flags & MF_SHADOW != 0) != (powers[2] > 0) {
                    return Err(DiagnosticError::Identity);
                }
            }
        }
        Ok(())
    }
    /// DSIM38: the player body at its type-1 THINGS record's lump position
    /// (`P_LoadThings`, p_setup.c:313–346; `P_SpawnMapThing`, p_mobj.c:729–736).
    /// The thinker list only appends and an actor spawned before the body is
    /// either still before it or removed, so: exactly one type-1 record in
    /// `map.things`; `original_player.0.mo` present, occupied and in
    /// `thinker.order`, with no spawnpoint (`P_SpawnPlayer` sets none, p_mobj.c:
    /// 642–686); every actor before it in `thinker.order` has a spawnpoint
    /// matching, in lump order, a record before the type-1 record; and the body's
    /// slot (allocated in spawn order on the fresh store) lies between the number
    /// of those actors and the number of those records.
    fn validate_level_body(&self) -> Result<(), DiagnosticError> {
        let things = self.required("map.things")?;
        if things.len() % 10 != 0 {
            return Err(DiagnosticError::Structure);
        }
        let mut starts = things
            .chunks_exact(10)
            .enumerate()
            .filter(|(_, r)| i16::from_le_bytes([r[6], r[7]]) == 1)
            .map(|(i, _)| i);
        let (Some(start), None) = (starts.next(), starts.next()) else {
            return Err(DiagnosticError::Identity);
        };
        let raw = self.required("original_player.0.mo")?;
        if raw.len() != 13 || raw[0] != 1 {
            return Err(DiagnosticError::Identity);
        }
        let body = &raw[1..];
        let slot = u32::from_le_bytes(number(&body[..4])?);
        let prefix = format!("actor.{slot}");
        if self.required(&format!("{prefix}.occupied"))? != [1]
            || self.required(&format!("{prefix}.generation"))? != &body[4..]
            || self.required(&format!("{prefix}.spawnpoint"))? != [0]
        {
            return Err(DiagnosticError::Identity);
        }
        let mut records = things[..start * 10].chunks_exact(10);
        let mut before = 0_u32;
        let mut found = false;
        for handle in self.required("thinker.order")?.chunks_exact(12) {
            if handle == body {
                found = true;
                break;
            }
            let other = u32::from_le_bytes(number(&handle[..4])?);
            let spawn = self.required(&format!("actor.{other}.spawnpoint"))?;
            if spawn.len() != 11 || spawn[0] != 1 || !records.any(|r| r == &spawn[1..]) {
                return Err(DiagnosticError::Identity);
            }
            before += 1;
        }
        if !found || slot < before || slot as usize > start {
            return Err(DiagnosticError::Identity);
        }
        Ok(())
    }
    /// DSIM35 tag 26: the payload width of each sub-tag 0..=14 and its handles,
    /// lines, sectors and small domains.
    fn validate_mover_event(&self, event: &[u8]) -> Result<(), DiagnosticError> {
        let sectors = u32::from_le_bytes(number(self.required("sectors.count")?)?);
        let lines = self.required("map.linedefs")?.len() / 14;
        let sub = *event.get(1).ok_or(DiagnosticError::Structure)?;
        let len = match sub {
            0 | 10 | 11 => 14,
            1 => 9,
            2 => 22,
            3 => 21,
            4 => 15,
            5 => 17,
            6 => 14,
            7 => 26,
            8 => 5,
            9 => 27,
            12 => 7,
            13 | 14 => 3,
            // DSIM40: `EV_Teleport` returned 1 (15) or 0 (16); `A_BossDeath` (17).
            15 if self.version == 40 => 40,
            16 if self.version == 40 => 17,
            17 if self.version == 40 => 15,
            _ => return Err(DiagnosticError::Structure),
        };
        if event.len() != len {
            return Err(DiagnosticError::Structure);
        }
        let word = |at: usize| u16::from_le_bytes([event[at], event[at + 1]]);
        let line = |at: usize| {
            if usize::from(word(at)) >= lines {
                Err(DiagnosticError::Identity)
            } else {
                Ok(())
            }
        };
        let sector = |at: usize| {
            if u32::from(word(at)) >= sectors {
                Err(DiagnosticError::Identity)
            } else {
                Ok(())
            }
        };
        let mover = |at: usize| {
            if u64::from_le_bytes(number(&event[at + 4..at + 12])?) == 0 {
                Err(DiagnosticError::Identity)
            } else {
                Ok(())
            }
        };
        let ok = match sub {
            0 => {
                validate_handle(&event[2..14], self.slot_bound())?;
                true
            }
            1 => {
                line(2)?;
                event[8] <= 1
            }
            2 => {
                validate_handle(&event[2..14], self.slot_bound())?;
                line(14)?;
                event[16] <= 1 && event[19] <= 1
            }
            3 => {
                validate_handle(&event[2..14], self.slot_bound())?;
                line(14)?;
                event[16] <= 1
            }
            4 => {
                validate_handle(&event[2..14], self.slot_bound())?;
                (1..=8).contains(&event[14])
            }
            5 => {
                mover(2)?;
                sector(15)?;
                (5..=7).contains(&event[14])
            }
            6 => {
                // DSIM40: `A_BossDeath`'s `EV_DoFloor` (4) names no line (65535)
                // and is `lowerFloorToLowest` (1).
                if !(self.version == 40
                    && word(3) == u16::MAX
                    && event[2] == 4
                    && i32::from_le_bytes([event[5], event[6], event[7], event[8]]) == 1)
                {
                    line(3)?;
                }
                // DSIM39: `EV_BuildStairs` (5) and `EV_DoDonut` (6) too.
                (1..=4).contains(&event[2])
                    || (matches!(self.version, 39 | 40) && (5..=6).contains(&event[2]))
            }
            7 => {
                sector(2)?;
                event[4] <= 1 && event[25] <= 2
            }
            8 => {
                sector(2)?;
                event[4] <= 1
            }
            9 => {
                validate_handle(&event[2..14], self.slot_bound())?;
                event[14] <= 4
            }
            10 | 11 => {
                mover(2)?;
                true
            }
            12 => match event[2] {
                1 => {
                    sector(3)?;
                    true
                }
                2 => usize::from(word(3)) < crate::mover_rules::MAX_BUTTONS,
                _ => false,
            },
            // DSIM39: the secret exit (S1 51) sets `secretexit`.
            13 => event[2] == 0 || (matches!(self.version, 39 | 40) && event[2] == 1),
            14 => event[2] == 0,
            // DSIM40: thing, line, landing, old x/y/z (any 16.16 value).
            15 => {
                validate_handle(&event[2..14], self.slot_bound())?;
                line(14)?;
                validate_handle(&event[16..28], self.slot_bound())?;
                event[2..14] != event[16..28]
            }
            // DSIM40: thing, line, reason 0 back side, 1 no landing, 2 refused.
            16 => {
                validate_handle(&event[2..14], self.slot_bound())?;
                line(14)?;
                event[16] <= 2
            }
            // DSIM40: the Baron, outcome 0 not map 8, 1 no live player, 2 another
            // Baron alive, 3 the tag-666 floor call.
            17 => {
                validate_handle(&event[2..14], self.slot_bound())?;
                event[14] <= 3
            }
            _ => false,
        };
        if !ok {
            return Err(DiagnosticError::Identity);
        }
        Ok(())
    }
    /// DSIM34 tag 25 against the snapshot: a sky event's line exists in
    /// `map.linedefs`, and a sky removal names an occupied fireball (it is marked
    /// for removal in the same tic and retired only by the next traversal).
    fn validate_sky_event(&self, event: &[u8]) -> Result<(), DiagnosticError> {
        let line = match event.get(1) {
            Some(1) if event.len() == 16 => [event[14], event[15]],
            Some(2) if event.len() == 9 => [event[3], event[4]],
            _ => return Ok(()),
        };
        let linedefs = self.required("map.linedefs")?;
        if linedefs.len() % 14 != 0 || usize::from(u16::from_le_bytes(line)) >= linedefs.len() / 14
        {
            return Err(DiagnosticError::Identity);
        }
        if event[1] == 1 {
            let slot = u32::from_le_bytes(number(&event[2..6])?);
            let prefix = format!("actor.{slot}");
            let fireball = OriginalActorType::MT_TROOPSHOT.ordinal().to_le_bytes();
            // DSIM39: a rocket removed against the sky likewise (M4a step 6).
            let rocket = OriginalActorType::MT_ROCKET.ordinal().to_le_bytes();
            // DSIM40: a Baron fireball removed against the sky likewise.
            let baron_ball = OriginalActorType::MT_BRUISERSHOT.ordinal().to_le_bytes();
            let ty = self.field(&format!("{prefix}.type"));
            if self.field(&format!("{prefix}.occupied")) != Some(&[1])
                || self.field(&format!("{prefix}.generation")) != Some(&event[6..14])
                || !(ty == Some(&fireball)
                    || (matches!(self.version, 39 | 40) && ty == Some(&rocket))
                    || (self.version == 40 && ty == Some(&baron_ball)))
            {
                return Err(DiagnosticError::Identity);
            }
        }
        Ok(())
    }
    fn validate_bonus_statistics(&self) -> Result<(), DiagnosticError> {
        let total = i32::from_le_bytes(number(self.required("level.total_items")?)?);
        let collected = i32::from_le_bytes(number(
            self.required("original_player.0.pickups.item_count")?,
        )?);
        if !(0..=pickup_bound_for(self.version, self.e1())).contains(&total)
            || !(0..=total).contains(&collected)
        {
            return Err(DiagnosticError::Identity);
        }
        let things = self.required("map.things")?;
        // DSIM33 carries the THINGS lump of a real-size map, bounded by the WAD
        // format's 32,768 records; earlier versions keep the 256-THING cap.
        let bound = if matches!(self.version, 33..=40) {
            32_768 * 10
        } else {
            MAX_DEMON_MAP_THINGS * 10
        };
        if things.len() % 10 != 0 || things.len() > bound {
            return Err(DiagnosticError::Limit);
        }
        let raw = || {
            things.chunks_exact(10).map(|t| {
                (
                    i16::from_le_bytes([t[6], t[7]]),
                    u16::from_le_bytes([t[8], t[9]]),
                )
            })
        };
        // DSIM40 counts with the option bit of the frame's skill (M4b step 6);
        // every older version with the medium bit.
        let bit = if self.version == 40 {
            crate::episode1::skill_bit(self.v40_skill())
        } else {
            2
        };
        let expected = world_pickups::counted_things_with_bit(
            raw(),
            if matches!(self.version, 39 | 40) {
                &world_pickups::COUNTED_ITEM_DOOMED_EPISODE1
            } else {
                &world_pickups::COUNTED_ITEM_DOOMED
            },
            bit,
        );
        if matches!(self.version, 28..=40) {
            // Every admitted COUNTKILL monster is a placed THING; credits never
            // exceed the map total. Zero again describes a pre-batch world.
            let kills = i32::from_le_bytes(number(self.required("level.total_kills")?)?);
            let credited =
                i32::from_le_bytes(number(self.required("level.single_player_kill_count")?)?);
            let placed = world_pickups::counted_things(
                raw(),
                if matches!(self.version, 29..=40) {
                    &world_pickups::COUNTED_KILL_DOOMED_IMPS
                } else {
                    &world_pickups::COUNTED_KILL_DOOMED
                },
            );
            // DSIM39: every Episode 1 monster is COUNTKILL; `K` bounds the total.
            let (placed, roster) = match self.e1() {
                Some(b) => (b.monsters, b.monsters),
                None => (placed, MAX_PLAYER_MONSTERS),
            };
            if !(0..=roster as i32).contains(&kills)
                || (kills > 0 && kills as usize != placed)
                || !(0..=kills).contains(&credited)
            {
                return Err(DiagnosticError::Identity);
            }
        }
        // Zero also describes a constructed world whose THING batch has not run.
        if total > 0 && total as usize != expected {
            return Err(DiagnosticError::Identity);
        }
        let mut live = 0;
        for slot in 0..self.slots()? {
            let prefix = format!("actor.{slot}");
            if self.required(&format!("{prefix}.occupied"))? != [1] {
                continue;
            }
            let ty = u16::from_le_bytes(number(self.required(&format!("{prefix}.type"))?)?);
            let counted = matches!(ty, 45 | 46)
                || (matches!(self.version, 39 | 40)
                    && world_pickups::kind(crate::ActorTypeId(ty))
                        .is_some_and(|k| k.definition_flags() & world_pickups::MF_COUNTITEM != 0));
            if counted && self.required(&format!("{prefix}.pending_removal"))? == [0] {
                live += 1;
            }
        }
        if live > (total - collected) as usize {
            return Err(DiagnosticError::Identity);
        }
        Ok(())
    }
    /// The `[floor, ceiling]` of this slot's `actor.{slot}.sector`, whose
    /// `sector.{i}.actors` list must hold the slot's handle (12 bytes: slot then
    /// generation) when `listed`, for the DSIM31 item witness. A linked item whose
    /// sector field and sector list disagree is not a consistent snapshot; an item
    /// pending removal is already unlinked and only its sector field is read.
    fn item_sector_span(&self, slot: usize, listed: bool) -> Result<[i32; 2], DiagnosticError> {
        let generation = number::<8>(self.required(&format!("actor.{slot}.generation"))?)?;
        let sector = u16::from_le_bytes(number(self.required(&format!("actor.{slot}.sector"))?)?);
        let slot = u32::try_from(slot).map_err(|_| DiagnosticError::Limit)?;
        let mut handle = [0_u8; 12];
        handle[..4].copy_from_slice(&slot.to_le_bytes());
        handle[4..].copy_from_slice(&generation);
        let count = u32::from_le_bytes(number(self.required("sectors.count")?)?);
        if u32::from(sector) >= count
            || (listed
                && !self
                    .required(&format!("sector.{sector}.actors"))?
                    .chunks(12)
                    .any(|h| h == handle))
        {
            return Err(DiagnosticError::Identity);
        }
        let height = |name: &str| -> Result<i32, DiagnosticError> {
            Ok(i32::from_le_bytes(number(
                self.required(&format!("sector.{sector}.{name}"))?,
            )?))
        };
        Ok([height("floor")?, height("ceiling")?])
    }
    fn validate_pickup_items(&self) -> Result<(), DiagnosticError> {
        let mut count = 0;
        for slot in 0..self.slots()? {
            let prefix = format!("actor.{slot}");
            if self.required(&format!("{prefix}.occupied"))? != [1] {
                continue;
            }
            let ty = ActorTypeId(u16::from_le_bytes(number(
                self.required(&format!("{prefix}.type"))?,
            )?));
            let Some(k) = world_pickups::kind_for(self.policy(), ty) else {
                continue;
            };
            count += 1;
            let state = OriginalState::try_from(u16::from_le_bytes(number(
                self.required(&format!("{prefix}.state"))?,
            )?))
            .map_err(|_| DiagnosticError::Identity)?;
            if !world_pickups::state_allowed(ty, state) {
                return Err(DiagnosticError::Identity);
            }
            let descriptor = state.descriptor();
            let i32field = |name: &str| -> Result<i32, DiagnosticError> {
                Ok(i32::from_le_bytes(number(
                    self.required(&format!("{prefix}.{name}"))?,
                )?))
            };
            let flags = u32::from_le_bytes(number(self.required(&format!("{prefix}.flags"))?)?);
            let spawn = self.required(&format!("{prefix}.spawnpoint"))?;
            let dropped = flags & world_pickups::MF_DROPPED != 0;
            // DSIM31 items sit on their own sector's floor; earlier versions on the
            // flat world's floor 0 under ceiling 128.
            let span = if matches!(self.version, 31..=40) {
                // A collected or removed item is unlinked from its sector list at
                // once and retired by the next pass; its sector field still holds.
                let listed = self.required(&format!("{prefix}.pending_removal"))? == [0];
                self.item_sector_span(slot, listed)?
            } else {
                [0, 128 * 65536]
            };
            if u16::from_le_bytes(number(self.required(&format!("{prefix}.state"))?)?)
                != state.ordinal()
                || u16::from_le_bytes(number(self.required(&format!("{prefix}.sprite"))?)?)
                    != descriptor.sprite.ordinal()
                || u32::from_le_bytes(number(self.required(&format!("{prefix}.frame"))?)?)
                    != descriptor.frame
                || (if k.is_animated() {
                    !(1..=descriptor.tics).contains(&i32field("tics")?)
                } else {
                    i32field("tics")? != -1
                })
                || i32field("health")? != 1000
                || i32field("radius")? != 20 * 65536
                || i32field("height")? != 16 * 65536
                || i32field("momx")? != 0
                || i32field("momy")? != 0
                || i32field("momz")? != 0
                || i32field("z")? != i32field("floor")?
                // DSIM35: the relaxed witness (§4.2 of the M3h8 note): `z == floorz` on
                // the item's own fields, not the centre sector's heights.
                || (!matches!(self.version, 35..=40)
                    && [i32field("floor")?, i32field("ceiling")?] != span)
                || self.required(&format!("{prefix}.target"))? != [0; 13]
                || flags
                    & !(k.definition_flags() | world_pickups::MF_AMBUSH | world_pickups::MF_DROPPED)
                    != 0
                || flags & k.definition_flags() != k.definition_flags()
                || (dropped
                    && (spawn != [0]
                        || flags & world_pickups::MF_AMBUSH != 0
                        || !matches!(
                            k,
                            PickupKind::Clip | PickupKind::Shotgun | PickupKind::Chaingun
                        )))
                || (!dropped
                    && (spawn.len() != 11
                        || spawn[0] != 1
                        || i32::from(i16::from_le_bytes(number(&spawn[7..9])?))
                            != k.original_type().descriptor().doomed_number))
            {
                return Err(DiagnosticError::Identity);
            }
        }
        if count > world_pickups::world_item_bound(self.policy(), self.e1()) {
            return Err(DiagnosticError::Limit);
        }
        Ok(())
    }
    fn validate_weapons(&self) -> Result<(), DiagnosticError> {
        let prefix = "original_player.0.weapons";
        if self
            .fields
            .iter()
            .any(|f| f.path.starts_with("original_player.0.pistol"))
        {
            return Err(DiagnosticError::Schema);
        }
        match self.required(&format!("{prefix}.present"))? {
            [0] => {
                if self
                    .fields
                    .iter()
                    .any(|f| f.path.starts_with(prefix) && !f.path.ends_with(".present"))
                {
                    return Err(DiagnosticError::Schema);
                }
                return Ok(());
            }
            [1] => {}
            _ => return Err(DiagnosticError::Structure),
        }
        let byte = |key: &str| -> Result<u8, DiagnosticError> {
            Ok(number::<1>(self.required(&format!("{prefix}.{key}"))?)?[0])
        };
        let word = |key: &str| -> Result<u16, DiagnosticError> {
            Ok(u16::from_le_bytes(number(
                self.required(&format!("{prefix}.{key}"))?,
            )?))
        };
        let int = |key: &str| -> Result<i32, DiagnosticError> {
            Ok(i32::from_le_bytes(number(
                self.required(&format!("{prefix}.{key}"))?,
            )?))
        };
        let ready = crate::Weapon::from_number_for(byte("ready")?, matches!(self.version, 39 | 40))
            .map_err(|_| DiagnosticError::Identity)?;
        let pending = match byte("pending")? {
            crate::player_weapons::NO_CHANGE => None,
            w => Some(
                crate::Weapon::from_number_for(w, matches!(self.version, 39 | 40))
                    .map_err(|_| DiagnosticError::Identity)?,
            ),
        };
        let latch = match byte("attack_down")? {
            0 => false,
            1 => true,
            _ => return Err(DiagnosticError::Schema),
        };
        let mut psprites = [crate::Psprite::default(); 2];
        for (i, ps) in psprites.iter_mut().enumerate() {
            let state = word(&format!("psprite.{i}.state"))?;
            ps.state = if state == u16::MAX {
                None
            } else {
                Some(crate::StateId(state))
            };
            ps.tics = int(&format!("psprite.{i}.tics"))?;
            ps.position = [
                crate::Fixed::from_bits(int(&format!("psprite.{i}.x"))?),
                crate::Fixed::from_bits(int(&format!("psprite.{i}.y"))?),
            ];
        }
        let state = crate::PlayerWeaponState {
            ready,
            pending,
            owned: word("owned")?,
            bullets: word("bullets")?,
            shells: word("shells")?,
            cells: if matches!(self.version, 39 | 40) {
                word("cells")?
            } else {
                0
            },
            rockets: if matches!(self.version, 39 | 40) {
                word("rockets")?
            } else {
                0
            },
            backpack: matches!(self.version, 39 | 40)
                && self.required("original_player.0.backpack")? == [1],
            attack_down: latch,
            refire: int("refire")?,
            bob: crate::Fixed::from_bits(int("bob")?),
            extra_light: int("extra_light")?,
            psprites,
        };
        state
            .validate_for(matches!(self.version, 39 | 40))
            .map_err(|_| DiagnosticError::Schema)?;
        if self.required("original_player.0.state")? != [0]
            && state.psprites[0].state != Some(state.ready.states()[1])
        {
            return Err(DiagnosticError::Schema);
        }
        Ok(())
    }
    fn validate_pistol(&self) -> Result<(), DiagnosticError> {
        let prefix = "original_player.0.pistol";
        match self.required(&format!("{prefix}.present"))? {
            [0] => {
                if self
                    .fields
                    .iter()
                    .any(|f| f.path.starts_with(prefix) && !f.path.ends_with(".present"))
                {
                    return Err(DiagnosticError::Schema);
                }
                return Ok(());
            }
            [1] => {}
            _ => return Err(DiagnosticError::Structure),
        }
        let read = |key: &str| -> Result<i32, DiagnosticError> {
            Ok(i32::from_le_bytes(number(
                self.required(&format!("{prefix}.{key}"))?,
            )?))
        };
        let bullets = u16::from_le_bytes(number(self.required(&format!("{prefix}.bullets"))?)?);
        let light = read("extra_light")?;
        if bullets > 200
            || !matches!(self.required(&format!("{prefix}.attack_down"))?, [0] | [1])
            || !(0..=200).contains(&read("refire")?)
            || !(0..=crate::weapons::MAX_BOB_BITS).contains(&read("bob")?)
            || !(0..=1).contains(&light)
        {
            return Err(DiagnosticError::Schema);
        }
        for channel in 0..2 {
            let state = u16::from_le_bytes(number(
                self.required(&format!("{prefix}.psprite.{channel}.state"))?,
            )?);
            let tics = read(&format!("psprite.{channel}.tics"))?;
            let x = read(&format!("psprite.{channel}.x"))?;
            let y = read(&format!("psprite.{channel}.y"))?;
            if !(-15 * 65536..=17 * 65536).contains(&x) || !(0..=128 * 65536).contains(&y) {
                return Err(DiagnosticError::Schema);
            }
            let limit = if channel == 0 {
                let state =
                    OriginalState::try_from(state).map_err(|_| DiagnosticError::Identity)?;
                if !matches!(
                    state,
                    OriginalState::S_PISTOL
                        | OriginalState::S_PISTOLUP
                        | OriginalState::S_PISTOLDOWN
                        | OriginalState::S_PISTOL1
                        | OriginalState::S_PISTOL2
                        | OriginalState::S_PISTOL3
                        | OriginalState::S_PISTOL4
                ) {
                    return Err(DiagnosticError::Identity);
                }
                state.descriptor().tics
            } else if state == u16::MAX {
                if tics != 0 || light != 0 {
                    return Err(DiagnosticError::Schema);
                }
                continue;
            } else if state == OriginalState::S_PISTOLFLASH.ordinal() && light == 1 {
                OriginalState::S_PISTOLFLASH.descriptor().tics
            } else {
                return Err(DiagnosticError::Identity);
            };
            if !(1..=limit).contains(&tics) {
                return Err(DiagnosticError::Schema);
            }
            if channel == 0
                && self.required("original_player.0.state")? == [1]
                && state != OriginalState::S_PISTOLDOWN.ordinal()
            {
                return Err(DiagnosticError::Schema);
            }
        }
        Ok(())
    }
    fn validate_player(&self) -> Result<(), DiagnosticError> {
        let hp = i32::from_le_bytes(number(self.required("original_player.0.health")?)?);
        let damage = i32::from_le_bytes(number(self.required("original_player.0.damage_count")?)?);
        let frags = i32::from_le_bytes(number(self.required("original_player.0.frags")?)?);
        let state = self.required("original_player.0.state")?;
        if !(0..=if matches!(self.version, 27..=40) {
            200
        } else {
            100
        })
            .contains(&hp)
            || !(0..=100).contains(&damage)
            || frags < 0
            || !(matches!(state, [0] | [1]) || (matches!(self.version, 35..=40) && state == [2]))
            || (state != [0]) != (hp == 0)
        {
            return Err(DiagnosticError::Identity);
        }
        let raw = self.required("original_player.0.mo")?;
        let mo = if raw[0] == 1 {
            Some(ActorId::from_parts(
                u32::from_le_bytes(number(&raw[1..5])?),
                u64::from_le_bytes(number(&raw[5..13])?),
            ))
        } else {
            None
        };
        let attacker = self.required("original_player.0.attacker")?;
        if attacker[0] == 1 {
            let handle = ActorId::from_parts(
                u32::from_le_bytes(number(&attacker[1..5])?),
                u64::from_le_bytes(number(&attacker[5..13])?),
            );
            if let Some(DiagnosticIdentity::Original { definition, .. }) = self.actor(handle)? {
                let monster = |ty: OriginalActorType| {
                    original_registry::is_policy_monster(self.policy(), ActorTypeId(ty.ordinal()))
                };
                if !matches!(
                    definition,
                    DefinitionIdentity::Original(OriginalActorType::MT_PLAYER)
                ) && !matches!(definition, DefinitionIdentity::Original(ty) if monster(ty))
                {
                    return Err(DiagnosticError::Identity);
                }
            }
        }
        for slot in 0..self.slots()? {
            let prefix = format!("actor.{slot}");
            if self.required(&format!("{prefix}.occupied"))? != [1] {
                continue;
            }
            let generation =
                u64::from_le_bytes(number(self.required(&format!("{prefix}.generation"))?)?);
            let current = mo
                == Some(ActorId::from_parts(
                    u32::try_from(slot).map_err(|_| DiagnosticError::Identity)?,
                    generation,
                ));
            let ty = u16::from_le_bytes(number(self.required(&format!("{prefix}.type"))?)?);
            if self.required(&format!("{prefix}.player"))? != [if current { 0 } else { u8::MAX }]
                || current && ty != OriginalActorType::MT_PLAYER.ordinal()
            {
                return Err(DiagnosticError::Identity);
            }
            if current {
                let actor_hp =
                    i32::from_le_bytes(number(self.required(&format!("{prefix}.health"))?)?);
                if actor_hp.max(0) != hp {
                    return Err(DiagnosticError::Identity);
                }
            }
        }
        Ok(())
    }
    /// Re-encode this read-only observation, preserving exact field/event order.
    pub fn encode(&self, limits: DiagnosticLimits) -> Result<Vec<u8>, DiagnosticError> {
        // Reapply caller budgets, including a smaller field/path/value policy.
        if self.bytes.len() > limits.bytes
            || self.fields.len() > limits.fields
            || self
                .fields
                .iter()
                .any(|f| f.path.len() > limits.path || f.value.len() > limits.value)
        {
            return Err(DiagnosticError::Limit);
        }
        let mut out = reserve(self.bytes.len())?;
        out.extend_from_slice(self.bytes);
        Ok(out)
    }
}
/// Handle slots are bounded by the frame's store capacity: 64 for every version
/// before DSIM30, `MAP_SCALE_ACTOR_SLOTS` for the map-scale policy.
fn validate_handle(bytes: &[u8], slots: u32) -> Result<(), DiagnosticError> {
    let raw = number::<12>(bytes)?;
    let slot = u32::from_le_bytes(number(&raw[..4])?);
    let generation = u64::from_le_bytes(number(&raw[4..])?);
    if slot >= slots || generation == 0 {
        return Err(DiagnosticError::Identity);
    }
    Ok(())
}
/// `MF_SHADOW` (p_mobj.h): partial invisibility.
const MF_SHADOW: u32 = 0x40000;
/// A DSIM39 normalized flat name (decision D2): at least one byte, upper-case
/// ASCII graphic bytes up to the first NUL, NUL padding after it.
fn normalized_flat_name(name: &[u8; 8]) -> bool {
    let end = name.iter().position(|b| *b == 0).unwrap_or(8);
    end > 0
        && name[..end]
            .iter()
            .all(|b| b.is_ascii_graphic() && !b.is_ascii_lowercase())
        && name[end..].iter().all(|b| *b == 0)
}
/// Store capacity admitted by a DSIM version: 1,024 for DSIM39 (the Episode 1
/// policy), 384 for DSIM37 and DSIM38 (the barrels and level policies),
/// `MAP_SCALE_ACTOR_SLOTS` for DSIM30–36, 64 before.
const fn slot_bound_for(version: u32) -> u32 {
    if matches!(version, 39 | 40) {
        crate::player_map_scale::EPISODE1_ACTOR_SLOTS
    } else if matches!(version, 37 | 38) {
        crate::player_map_scale::BARRELS_ACTOR_SLOTS
    } else if matches!(version, 30..=36) {
        crate::player_map_scale::MAP_SCALE_ACTOR_SLOTS
    } else {
        64
    }
}
/// Item bound admitted by a DSIM version (`level.total_items`,
/// `pickups.item_count` and the pickup events): the frame's own `P` for DSIM39
/// (`e1`, counted from `map.things`), 64 for DSIM37 and DSIM38, 16 before.
fn pickup_bound_for(version: u32, e1: Option<Episode1Bounds>) -> i32 {
    if let (39 | 40, Some(b)) = (version, e1) {
        i32::try_from(b.pickups).unwrap_or(i32::MAX)
    } else if matches!(version, 39 | 40) {
        0
    } else if matches!(version, 37 | 38) {
        world_pickups::BARRELS_MAX_PICKUPS as i32
    } else {
        MAX_WORLD_PICKUPS as i32
    }
}
/// `bonuscount` bound admitted by a DSIM version: `BONUSADD` per admitted item;
/// for DSIM39 `6·(P + R + keycards)` of the frame's own THINGS.
fn bonus_bound_for(version: u32, e1: Option<Episode1Bounds>) -> i32 {
    if let (39 | 40, Some(b)) = (version, e1) {
        i32::try_from(b.max_bonus()).unwrap_or(i32::MAX)
    } else if matches!(version, 39 | 40) {
        0
    } else if matches!(version, 37 | 38) {
        world_pickups::max_bonus(OriginalExecutionProfile::PlayerMonstersBarrelsV1)
    } else {
        world_pickups::MAX_BONUS
    }
}
impl Snapshot {
    /// Bounded Result-returning DSIM20/21/22/23/24 encoder. Trusted convenience APIs retain
    /// their old allocation policy; callers at input/output boundaries use this.
    pub fn diagnostic_bytes(&self, limits: DiagnosticLimits) -> Result<Vec<u8>, DiagnosticError> {
        if !matches!(self.version(), 20..=40) {
            return Err(DiagnosticError::Version(self.version()));
        }
        let mut size = 16_usize;
        let mut count = 0_usize;
        for field in self.fields() {
            count = count.checked_add(1).ok_or(DiagnosticError::Limit)?;
            if count > limits.fields
                || field.path().len() > limits.path
                || u32::try_from(field.path().len()).is_err()
                || field.value().len() > limits.value
            {
                return Err(DiagnosticError::Limit);
            }
            size = size
                .checked_add(12)
                .and_then(|s| s.checked_add(field.path().len()))
                .and_then(|s| s.checked_add(field.value().len()))
                .ok_or(DiagnosticError::Limit)?;
            if size > limits.bytes {
                return Err(DiagnosticError::Limit);
            }
        }
        let mut out = reserve(size)?;
        out.extend_from_slice(b"DSIM");
        out.extend_from_slice(&self.version().to_le_bytes());
        out.extend_from_slice(
            &u64::try_from(count)
                .map_err(|_| DiagnosticError::Limit)?
                .to_le_bytes(),
        );
        for f in self.fields() {
            out.extend_from_slice(
                &u32::try_from(f.path().len())
                    .map_err(|_| DiagnosticError::Limit)?
                    .to_le_bytes(),
            );
            out.extend_from_slice(f.path().as_bytes());
            out.extend_from_slice(
                &u64::try_from(f.value().len())
                    .map_err(|_| DiagnosticError::Limit)?
                    .to_le_bytes(),
            );
            out.extend_from_slice(f.value());
        }
        Ok(out)
    }
}

/// `sectors` is the static sector count under DSIM31, where a noise flood visits any
/// sector with `blocks` 0 or 1 (`P_RecursiveSound`); earlier versions flood sector 0.
/// `episode1` (DSIM39) adds the rocket launcher and the chainsaw (their psprite
/// states and actions, the rocket ammo, weapons 4 and 7) and the doubled maxima.
fn validate_weapon_event(
    event: &[u8],
    pack: bool,
    episode1: bool,
    slots: u32,
    sectors: Option<u32>,
) -> Result<(), DiagnosticError> {
    let invalid = DiagnosticError::Identity;
    match event.get(1) {
        Some(0 | 1) if event.len() == 6 && event[2] == 0 && event[3] <= 1 => {
            let id = u16::from_le_bytes(number(&event[4..6])?);
            let valid = if event[1] == 0 {
                let state = OriginalState::try_from(id).map_err(|_| invalid)?;
                if event[3] == 0 {
                    (pack
                        && [
                            crate::Weapon::Fist,
                            crate::Weapon::Shotgun,
                            crate::Weapon::Chaingun,
                        ]
                        .into_iter()
                        .any(|w| crate::player_weapons::weapon_state(w, state)))
                        || (episode1
                            && [crate::Weapon::Rocket, crate::Weapon::Chainsaw]
                                .into_iter()
                                .any(|w| crate::player_weapons::weapon_state(w, state)))
                        || (pack && state == OriginalState::S_CHAIN3)
                        || matches!(
                            state,
                            OriginalState::S_PISTOL
                                | OriginalState::S_PISTOLUP
                                | OriginalState::S_PISTOLDOWN
                                | OriginalState::S_PISTOL1
                                | OriginalState::S_PISTOL2
                                | OriginalState::S_PISTOL3
                                | OriginalState::S_PISTOL4
                        )
                } else {
                    (pack && crate::player_weapons::flash_light(state).is_some())
                        || matches!(
                            state,
                            OriginalState::S_NULL
                                | OriginalState::S_LIGHTDONE
                                | OriginalState::S_PISTOLFLASH
                        )
                }
            } else {
                let name = crate::lifecycle::SOURCE_ACTIONS
                    .get(usize::from(id))
                    .copied();
                if event[3] == 0 {
                    (pack && matches!(name, Some("A_Punch" | "A_FireShotgun" | "A_FireCGun")))
                        || (episode1
                            && matches!(name, Some("A_Saw" | "A_FireMissile" | "A_GunFlash")))
                        || matches!(
                            name,
                            Some(
                                "A_Raise"
                                    | "A_Lower"
                                    | "A_WeaponReady"
                                    | "A_ReFire"
                                    | "A_FirePistol"
                            )
                        )
                } else {
                    matches!(name, Some("A_Light0" | "A_Light1"))
                        || (pack && name == Some("A_Light2"))
                }
            };
            if valid { Ok(()) } else { Err(invalid) }
        }
        Some(2)
            if event.len() == 12
                && event[2] == 0
                && (event[3] == 0 || (pack && event[3] == 1) || (episode1 && event[3] == 3)) =>
        {
            let before = i32::from_le_bytes(number(&event[4..8])?);
            let after = i32::from_le_bytes(number(&event[8..12])?);
            // The backpack doubles the maxima under DSIM39.
            let max = crate::weapons::AMMO_CAPACITIES[usize::from(event[3])] << u8::from(episode1);
            if (1..=max).contains(&before) && after == before - 1 {
                Ok(())
            } else {
                Err(invalid)
            }
        }
        Some(3) if event.len() == 7 && event[2] == 0 => {
            if (0..=if pack { 2 } else { 1 }).contains(&i32::from_le_bytes(number(&event[3..7])?)) {
                Ok(())
            } else {
                Err(invalid)
            }
        }
        Some(4)
            if event.len() == 8
                && event[2] == 0
                && (event[3] == 1
                    || (pack && event[3] <= 3)
                    || (episode1 && matches!(event[3], 4 | 7))) =>
        {
            if (0..=if pack { i32::MAX } else { 200 })
                .contains(&i32::from_le_bytes(number(&event[4..8])?))
            {
                Ok(())
            } else {
                Err(invalid)
            }
        }
        Some(5) if event.len() == 26 => {
            validate_handle(&event[2..14], slots)?;
            let slope = i32::from_le_bytes(number(&event[18..22])?);
            let damage = i32::from_le_bytes(number(&event[22..26])?);
            if (-40960..=40960).contains(&slope)
                && ([5, 10, 15].contains(&damage)
                    || (pack && (2..=20).contains(&damage) && damage % 2 == 0))
            {
                Ok(())
            } else {
                Err(invalid)
            }
        }
        Some(6) if event.len() == 17 => {
            let sector = u32::from(u16::from_le_bytes([event[2], event[3]]));
            let flooded = match sectors {
                Some(count) => sector < count && event[4] <= 1,
                None => event[2..5] == [0, 0, 0],
            };
            if !flooded {
                return Err(DiagnosticError::Structure);
            }
            validate_handle(&event[5..17], slots)
        }
        Some(7) if event.len() == 15 && event[2] == 0 => validate_handle(&event[3..15], slots),
        Some(8) if event.len() == 4 && event[2] <= 2 => Ok(()),
        _ => Err(DiagnosticError::Structure),
    }
}

/// Independent DSIM29 fireball event widths and source domains; plasma's optional
/// autoaim form and every other missile type stay outside this reader.
fn validate_projectile_event(
    event: &[u8],
    slots: u32,
    episode1: bool,
    baron: bool,
) -> Result<(), DiagnosticError> {
    const FIREBALL: u16 = OriginalActorType::MT_TROOPSHOT.ordinal();
    // DSIM40: `A_BruisAttack`'s `P_SpawnMissile` names `MT_BRUISERSHOT` (16).
    const BARON_BALL: u16 = OriginalActorType::MT_BRUISERSHOT.ordinal();
    match (event.get(1), event.len()) {
        // DSIM39: `P_SpawnPlayerMissile`'s rocket (aim kind 1, no destination).
        (Some(5), 17) if episode1 && event[14] == 1 => {
            validate_handle(&event[2..14], slots)?;
            if u16::from_le_bytes(number(&event[15..17])?) != OriginalActorType::MT_ROCKET.ordinal()
            {
                return Err(DiagnosticError::Identity);
            }
            Ok(())
        }
        (Some(0), 14) => validate_handle(&event[2..14], slots),
        (Some(1), 30) => validate_handle(&event[2..14], slots),
        // Spawn tics, shadow pair, contact damage and explosion tics.
        (Some(2), 4) if event[2] <= 4 => Ok(()),
        (Some(4), 30) => {
            validate_handle(&event[2..14], slots)?;
            if i32::from_le_bytes(number(&event[26..30])?) < 1 {
                return Err(DiagnosticError::Identity);
            }
            Ok(())
        }
        (Some(5), 29) if event[14] == 0 => {
            validate_handle(&event[2..14], slots)?;
            validate_handle(&event[15..27], slots)?;
            let ty = u16::from_le_bytes(number(&event[27..29])?);
            if ty != FIREBALL && !(baron && ty == BARON_BALL) {
                return Err(DiagnosticError::Identity);
            }
            Ok(())
        }
        _ => Err(DiagnosticError::Structure),
    }
}
fn validate_player_event(
    event: &[u8],
    pack: bool,
    bonuses: bool,
    episode1: bool,
    slots: u32,
) -> Result<(), DiagnosticError> {
    match event.get(1) {
        Some(0) if event.len() == 14 => validate_handle(&event[2..], slots),
        Some(1) if event.len() == 27 => {
            let hp = i32::from_le_bytes(number(&event[2..6])?);
            let count = i32::from_le_bytes(number(&event[19..23])?);
            let tactile = i32::from_le_bytes(number(&event[23..27])?);
            if !(0..=if bonuses { 200 } else { 100 }).contains(&hp)
                || !(0..=100).contains(&count)
                || !(40..=240).contains(&tactile)
                || tactile % 2 != 0
            {
                return Err(DiagnosticError::Identity);
            }
            match event[6] {
                0 if event[7..19].iter().all(|b| *b == 0) => Ok(()),
                1 => validate_handle(&event[7..19], slots),
                _ => Err(DiagnosticError::Identity),
            }
        }
        Some(2) if event.len() == 8 => {
            if i32::from_le_bytes(number(&event[2..6])?) < 0
                || !([OriginalState::S_PISTOLDOWN]
                    .into_iter()
                    .chain(
                        if pack {
                            Some([
                                OriginalState::S_PUNCHDOWN,
                                OriginalState::S_SGUNDOWN,
                                OriginalState::S_CHAINDOWN,
                            ])
                        } else {
                            None
                        }
                        .into_iter()
                        .flatten(),
                    )
                    .chain(
                        episode1
                            .then_some([OriginalState::S_MISSILEDOWN, OriginalState::S_SAWDOWN])
                            .into_iter()
                            .flatten(),
                    )
                    .any(|s| s.ordinal() == u16::from_le_bytes([event[6], event[7]])))
            {
                return Err(DiagnosticError::Identity);
            }
            Ok(())
        }
        _ => Err(DiagnosticError::Structure),
    }
}

fn diagnostic_armor(
    kind: u8,
    points: &[u8],
    policy: OriginalExecutionProfile,
) -> Result<PlayerArmorState, DiagnosticError> {
    let value = PlayerArmorState {
        kind: ArmorType::try_from(kind).map_err(|_| DiagnosticError::Identity)?,
        points: i32::from_le_bytes(number(points)?),
    };
    value
        .validate_for(policy)
        .map_err(|_| DiagnosticError::Identity)?;
    Ok(value)
}
/// The DSIM34 sector-thinker kind `P_SpawnSpecials` spawns for a map sector
/// special: 1 flash, 2 strobe, 3 glow, 4 flicker; `None` for every other special.
fn light_kind(special: i16) -> Option<u8> {
    match special {
        1 => Some(1),
        2 | 3 | 4 | 12 | 13 => Some(2),
        8 => Some(3),
        17 => Some(4),
        _ => None,
    }
}
/// DSIM34 tag 25: 0 secret (u16 sector, i32 count ≥ 1), 1 sky removal (fireball
/// handle, u16 line), 2 sky shot (u8 reason 0 or 1, u16 line, i32 z).
fn validate_special_event(event: &[u8], slots: u32, sectors: u32) -> Result<(), DiagnosticError> {
    match event.get(1) {
        Some(0) if event.len() == 8 => {
            let sector = u16::from_le_bytes(number(&event[2..4])?);
            if u32::from(sector) >= sectors || i32::from_le_bytes(number(&event[4..8])?) < 1 {
                return Err(DiagnosticError::Identity);
            }
        }
        Some(1) if event.len() == 16 => validate_handle(&event[2..14], slots)?,
        Some(2) if event.len() == 9 => {
            if event[2] > 1 {
                return Err(DiagnosticError::Identity);
            }
        }
        _ => return Err(DiagnosticError::Structure),
    }
    Ok(())
}
fn validate_armor_event(event: &[u8], version: u32, slots: u32) -> Result<(), DiagnosticError> {
    if event.len() != 35 {
        return Err(DiagnosticError::Structure);
    }
    validate_handle(&event[1..13], slots)?;
    let incoming = i32::from_le_bytes(number(&event[13..17])?);
    let saved = i32::from_le_bytes(number(&event[17..21])?);
    let effective = i32::from_le_bytes(number(&event[21..25])?);
    let policy = if version == 40 {
        OriginalExecutionProfile::PlayerMonstersEpisode1V2
    } else if version == 39 {
        OriginalExecutionProfile::PlayerMonstersEpisode1V1
    } else if version == 38 {
        OriginalExecutionProfile::PlayerMonstersLevelV1
    } else if version == 37 {
        OriginalExecutionProfile::PlayerMonstersBarrelsV1
    } else if version == 36 {
        OriginalExecutionProfile::PlayerMonstersE1M1V1
    } else if version == 35 {
        OriginalExecutionProfile::PlayerMonstersMoversV1
    } else if version == 34 {
        OriginalExecutionProfile::PlayerMonstersSpecialsV1
    } else if version == 33 {
        OriginalExecutionProfile::PlayerMonstersRealMapV1
    } else if version == 32 {
        OriginalExecutionProfile::PlayerMonstersDiagonalV1
    } else if version == 31 {
        OriginalExecutionProfile::PlayerMonstersHeightsV1
    } else if version == 30 {
        OriginalExecutionProfile::PlayerMonstersMapScaleV1
    } else if version == 29 {
        OriginalExecutionProfile::PlayerMonstersImpV1
    } else if version == 28 {
        OriginalExecutionProfile::PlayerMonstersV1
    } else if version == 27 {
        OriginalExecutionProfile::PlayerDemonBonusesV1
    } else {
        OriginalExecutionProfile::PlayerDemonArmorV1
    };
    let before = diagnostic_armor(event[25], &event[26..30], policy)?;
    let after = diagnostic_armor(event[30], &event[31..35], policy)?;
    let expected = ArmorEvent::plan_for(
        policy,
        ActorId::from_parts(
            u32::from_le_bytes(number(&event[1..5])?),
            u64::from_le_bytes(number(&event[5..13])?),
        ),
        incoming,
        before,
    )
    .map_err(|_| DiagnosticError::Identity)?;
    if saved != expected.saved || effective != expected.effective || after != expected.after {
        return Err(DiagnosticError::Identity);
    }
    Ok(())
}
// Independent DSIM25/26 widths and bounded source domains; no world restoration.
fn validate_pickup_event(
    event: &[u8],
    version: u32,
    slots: u32,
    e1: Option<Episode1Bounds>,
) -> Result<(), DiagnosticError> {
    let armor = matches!(version, 26..=40);
    let bonuses = matches!(version, 27..=40);
    // A ready (`pending` false) or pending (`pending` true: 10 is wp_nochange)
    // weapon number; DSIM39 adds the launcher (4) and the chainsaw (7).
    let weapon = |w: u8, pending: bool| {
        w <= 3 || (pending && w == 10) || (matches!(version, 39 | 40) && matches!(w, 4 | 7))
    };
    let policy = if version == 40 {
        OriginalExecutionProfile::PlayerMonstersEpisode1V2
    } else if version == 39 {
        OriginalExecutionProfile::PlayerMonstersEpisode1V1
    } else if version == 38 {
        OriginalExecutionProfile::PlayerMonstersLevelV1
    } else if version == 37 {
        OriginalExecutionProfile::PlayerMonstersBarrelsV1
    } else if version == 36 {
        OriginalExecutionProfile::PlayerMonstersE1M1V1
    } else if version == 35 {
        OriginalExecutionProfile::PlayerMonstersMoversV1
    } else if version == 34 {
        OriginalExecutionProfile::PlayerMonstersSpecialsV1
    } else if version == 33 {
        OriginalExecutionProfile::PlayerMonstersRealMapV1
    } else if version == 32 {
        OriginalExecutionProfile::PlayerMonstersDiagonalV1
    } else if version == 31 {
        OriginalExecutionProfile::PlayerMonstersHeightsV1
    } else if version == 30 {
        OriginalExecutionProfile::PlayerMonstersMapScaleV1
    } else if version == 29 {
        OriginalExecutionProfile::PlayerMonstersImpV1
    } else if version == 28 {
        OriginalExecutionProfile::PlayerMonstersV1
    } else if bonuses {
        OriginalExecutionProfile::PlayerDemonBonusesV1
    } else {
        OriginalExecutionProfile::PlayerDemonArmorV1
    };
    let get = |a: usize, b: usize| event.get(a..b).ok_or(DiagnosticError::Structure);
    let ok = match event.get(1) {
        Some(0) => {
            if event.len() != 40 {
                return Err(DiagnosticError::Structure);
            }
            validate_handle(get(2, 14)?, slots)?;
            validate_handle(get(14, 26)?, slots)?;
            let sprite = u16::from_le_bytes(number(get(26, 28)?)?);
            [
                OriginalSprite::SPR_STIM,
                OriginalSprite::SPR_MEDI,
                OriginalSprite::SPR_CLIP,
                OriginalSprite::SPR_AMMO,
                OriginalSprite::SPR_SHEL,
                OriginalSprite::SPR_SBOX,
                OriginalSprite::SPR_MGUN,
                OriginalSprite::SPR_SHOT,
            ]
            .map(OriginalSprite::ordinal)
            .contains(&sprite)
                || (armor
                    && [
                        OriginalSprite::SPR_ARM1.ordinal(),
                        OriginalSprite::SPR_ARM2.ordinal(),
                    ]
                    .contains(&sprite))
                || (bonuses
                    && [
                        OriginalSprite::SPR_BON1.ordinal(),
                        OriginalSprite::SPR_BON2.ordinal(),
                    ]
                    .contains(&sprite))
                || (matches!(version, 39 | 40)
                    && [
                        OriginalSprite::SPR_BKEY,
                        OriginalSprite::SPR_YKEY,
                        OriginalSprite::SPR_RKEY,
                        OriginalSprite::SPR_SOUL,
                        OriginalSprite::SPR_PINS,
                        OriginalSprite::SPR_SUIT,
                        OriginalSprite::SPR_PMAP,
                        OriginalSprite::SPR_PVIS,
                        OriginalSprite::SPR_BPAK,
                        OriginalSprite::SPR_ROCK,
                        OriginalSprite::SPR_BROK,
                        OriginalSprite::SPR_CSAW,
                        OriginalSprite::SPR_LAUN,
                    ]
                    .map(OriginalSprite::ordinal)
                    .contains(&sprite))
        }
        Some(1) => {
            event.len() == 14
                && (event[2] <= 1 || (matches!(version, 39 | 40) && event[2] <= 3))
                && event[3] <= 1
                && weapon(event[12], false)
                && weapon(event[13], true)
                && {
                    let before = i32::from_le_bytes(number(get(4, 8)?)?);
                    let after = i32::from_le_bytes(number(get(8, 12)?)?);
                    // DSIM39: the backpack may have doubled the maximum.
                    let max = crate::weapons::AMMO_CAPACITIES[usize::from(event[2])]
                        << u32::from(matches!(version, 39 | 40));
                    (0..=max).contains(&before) && (before..=max).contains(&after)
                }
        }
        Some(2) => {
            event.len() == 15 && event[2] <= 1 && {
                let before = i32::from_le_bytes(number(get(3, 7)?)?);
                let after = i32::from_le_bytes(number(get(7, 11)?)?);
                let body = i32::from_le_bytes(number(get(11, 15)?)?);
                if bonuses && (100..=200).contains(&before) {
                    event[2] == 0 && after == before && after == body
                } else {
                    (1..=100).contains(&before) && (before..=100).contains(&after) && after == body
                }
            }
        }
        Some(3) => {
            // DSIM39 also gives the launcher (4) and the chainsaw (7).
            let owned = if matches!(version, 39 | 40) { 0x9f } else { 15 };
            event.len() == 10
                && (matches!(event[2], 2 | 3)
                    || (matches!(version, 39 | 40) && matches!(event[2], 4 | 7)))
                && event[3] <= 1
                && u16::from_le_bytes(number(get(4, 6)?)?) & !owned == 0
                && u16::from_le_bytes(number(get(6, 8)?)?) & !owned == 0
                && weapon(event[8], true)
                && weapon(event[9], true)
        }
        Some(4) => {
            event.len() == 3
                && (matches!(event[2],1..=6|8..=9)
                    || (armor && matches!(event[2], 10..=11))
                    || (bonuses && matches!(event[2], 12..=13))
                    || (matches!(version, 39 | 40) && matches!(event[2], 14..=29)))
        }
        Some(5) => {
            event.len() == 10
                && (0..=bonus_bound_for(version, e1))
                    .contains(&i32::from_le_bytes(number(get(2, 6)?)?))
                && if bonuses {
                    (0..=pickup_bound_for(version, e1))
                        .contains(&i32::from_le_bytes(number(get(6, 10)?)?))
                } else {
                    get(6, 10)? == [0; 4]
                }
        }
        Some(6) => {
            if event.len() != 15 {
                return Err(DiagnosticError::Structure);
            }
            validate_handle(get(2, 14)?, slots)?;
            event[14] <= 3
        }
        // DSIM39: P_GiveCard, P_GivePower, the soulsphere and the backpack.
        Some(11) if matches!(version, 39 | 40) => {
            event.len() == 4 && event[2] <= 2 && event[3] <= 1
        }
        Some(12) if matches!(version, 39 | 40) => {
            event.len() == 12 && matches!(event[2], 2..=5) && event[3] <= 1 && {
                let before = i32::from_le_bytes(number(get(4, 8)?)?);
                let after = i32::from_le_bytes(number(get(8, 12)?)?);
                let max = if event[2] == 5 {
                    4200
                } else if event[2] == 4 {
                    1
                } else {
                    2100
                };
                (0..=max).contains(&before)
                    && (0..=max).contains(&after)
                    && (event[3] == 1 || after == before)
            }
        }
        Some(13) if matches!(version, 39 | 40) => {
            event.len() == 14 && {
                let before = i32::from_le_bytes(number(get(2, 6)?)?);
                let after = i32::from_le_bytes(number(get(6, 10)?)?);
                let body = i32::from_le_bytes(number(get(10, 14)?)?);
                (1..=200).contains(&before) && after == (before + 100).min(200) && body == after
            }
        }
        Some(14) if matches!(version, 39 | 40) => event.len() == 3 && event[2] <= 1,
        Some(10) if bonuses => {
            if event.len() != 10 {
                return Err(DiagnosticError::Structure);
            }
            let before = i32::from_le_bytes(number(get(2, 6)?)?);
            let after = i32::from_le_bytes(number(get(6, 10)?)?);
            (0..pickup_bound_for(version, e1)).contains(&before) && after == before + 1
        }
        Some(8) if bonuses => {
            if event.len() != 14 {
                return Err(DiagnosticError::Structure);
            }
            let before = i32::from_le_bytes(number(get(2, 6)?)?);
            let after = i32::from_le_bytes(number(get(6, 10)?)?);
            let body = i32::from_le_bytes(number(get(10, 14)?)?);
            (1..=200).contains(&before) && after == (before + 1).min(200) && body == after
        }
        Some(9) if bonuses => {
            if event.len() != 12 {
                return Err(DiagnosticError::Structure);
            }
            let before = diagnostic_armor(event[2], get(3, 7)?, policy)?;
            let after = diagnostic_armor(event[7], get(8, 12)?, policy)?;
            after.points == (before.points + 1).min(200)
                && after.kind
                    == if before.kind == ArmorType::None {
                        ArmorType::Green
                    } else {
                        before.kind
                    }
        }
        Some(7) if armor => {
            if event.len() != 14 || event[3] > 1 {
                return Err(DiagnosticError::Structure);
            }
            let requested = ArmorType::try_from(event[2]).map_err(|_| DiagnosticError::Identity)?;
            if requested == ArmorType::None {
                return Err(DiagnosticError::Identity);
            }
            let before = diagnostic_armor(event[4], get(5, 9)?, policy)?;
            let after = diagnostic_armor(event[9], get(10, 14)?, policy)?;
            let accepted = before.points < requested.capacity();
            accepted == (event[3] == 1)
                && after
                    == (if accepted {
                        PlayerArmorState {
                            kind: requested,
                            points: requested.capacity(),
                        }
                    } else {
                        before
                    })
        }
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(DiagnosticError::Structure)
    }
}
