//! Per-map bounds of the Episode 1 policy (`PlayerMonstersEpisode1V1`, M4a
//! decision H4).
//!
//! Every older map policy fixed its roster, pickup, decoration and barrel bounds
//! as constants and proved from them that the actor store never fills. Episode 1
//! maps hold 41 to 118 monsters, so wire 23 derives each bound once at load from
//! the map's own THINGS lump, counted with the filter `P_SpawnMapThing` applies
//! in single player at the medium skill (p_mobj.c:708–797: player and deathmatch
//! starts are not spawned as things; `MTF_NOTSINGLE` (16) is skipped; the medium
//! skill bit (2) is required), and classified by DoomEd number. A spawn site that
//! finds the 1,024-slot store full refuses the tic with `StoreError::Capacity`
//! instead (the store-full witness). The bounds are immutable, never canonical
//! state, and the DSIM39 reader derives the same values from `map.things`
//! (`Episode1Bounds::from_thing_records`).
use crate::identity::OriginalActorType;
use crate::lifecycle::Skill;
use doom_map::{Map, Thing};

/// DoomEd numbers of the monsters an Episode 1 map may place: the former human,
/// the sergeant, the chaingunner, the Imp, the Demon and the Spectre (info.c).
pub(crate) const EPISODE1_MONSTERS: [i16; 6] = [3004, 9, 65, 3001, 3002, 58];
/// The monsters among them whose `P_KillMobj` drops an item (p_inter.c:704–722):
/// the former human (clip), the sergeant (shotgun) and the chaingunner (chaingun).
pub(crate) const EPISODE1_DROPPERS: [i16; 3] = [3004, 9, 65];
/// DoomEd numbers of every Episode 1 pickup (info.c): ammunition, health, armor,
/// the six weapons and power-ups of part 1, the three keycards and the backpack.
pub(crate) const EPISODE1_PICKUPS: [i16; 25] = [
    2007, 2048, 2008, 2049, 2011, 2012, 2001, 2002, 2018, 2019, 2014, 2015, // wire 22
    5, 6, 13, // blue, yellow and red keycard
    2013, 2024, 2025, 2026, 2045, // soulsphere, invisibility, suit, map, visor
    8, 2010, 2046, 2005, 2003, // backpack, rocket, rocket box, chainsaw, launcher
];
/// DoomEd numbers of the keycards (`P_GiveCard` adds a second `BONUSADD`,
/// p_inter.c:274–283).
pub(crate) const EPISODE1_KEYCARDS: [i16; 3] = [5, 6, 13];
/// DoomEd number of the exploding barrel (info.c `MT_BARREL`).
pub(crate) const EPISODE1_BARREL: i16 = 2035;
/// DoomEd numbers of the static things beyond E1M1's seven decorations: the candle
/// (`MT_MISC49`, 34) and the firestick (`MT_MISC43`, 46).
pub(crate) const EPISODE1_EXTRA_STATICS: [i16; 2] = [34, 46];
/// DoomEd number of the teleport landing (info.c `MT_TELEPORTMAN`, wire 24).
pub(crate) const LANDING: i16 = 14;
/// DoomEd number of the Baron of Hell (info.c `MT_BRUISER`), a monster of wire 24
/// alone; `P_KillMobj` drops nothing for it (p_inter.c:737–754).
pub(crate) const BARON: i16 = 3003;
/// `BONUSADD` (p_inter.c): the `bonuscount` one pickup adds.
const BONUS_ADD: usize = 6;
/// The THING option bit of the medium skill (`MTF_NORMAL`, doomdef.h:140–145):
/// the bit every policy before wire 24 counts with.
const MEDIUM_BIT: u16 = 2;

/// The THING option bit `P_SpawnMapThing` requires at `skill` in single player
/// (p_mobj.c:743–751): 1 for baby and easy, `1<<(gameskill−1)` = 2 for medium,
/// 4 for hard and nightmare (`MTF_EASY`, `MTF_NORMAL`, `MTF_HARD`,
/// doomdef.h:140–145). The spawner applies the same mapping
/// (`Simulation::spawn_map_things`). Wire 24 admits medium and hard (M4b
/// decision I4); every older policy is medium only and counts with bit 2.
pub(crate) const fn skill_bit(skill: Skill) -> u16 {
    match skill {
        Skill::Baby | Skill::Easy => 1,
        Skill::Medium => 2,
        Skill::Hard | Skill::Nightmare => 4,
    }
}

/// The per-map bounds of the Episode 1 policy, counted from a THINGS lump with
/// the single-player, medium-skill `P_SpawnMapThing` filter.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Episode1Bounds {
    /// `K`: placed monsters.
    pub monsters: usize,
    /// `R`: placed monsters that drop an item when killed (each drops once).
    pub droppers: usize,
    /// `P`: placed pickups, keycards included.
    pub pickups: usize,
    /// Placed keycards (a subset of `pickups`).
    pub keycards: usize,
    /// `B`: placed exploding barrels.
    pub barrels: usize,
    /// `D`: placed static decorations.
    pub statics: usize,
    /// Placed pickups whose definition carries `MF_COUNTITEM` (the source of
    /// `totalitems`).
    pub countitems: usize,
}
impl Episode1Bounds {
    /// Count the bounds of a loaded map.
    pub fn from_map(map: &Map) -> Self {
        Self::from_things(map.things().iter().map(|t: &Thing| (t.kind, t.options)))
    }
    /// The bounds of wire 24 at `skill` (M4b steps 5 and 6): those of
    /// [`Self::from_map`], counted with the option bit of `skill`
    /// (`skill_bit`: 2 for medium, 4 for hard; p_mobj.c:743–751) instead of the
    /// fixed medium bit, and with the Baron of Hell (doomednum 3003) counted in
    /// `K` as a monster that drops nothing. Wire 23 refuses doomednum 3003 at
    /// spawn and any skill but medium at load, and keeps [`Self::from_map`], so
    /// its bounds and the DSIM39 reader's are unchanged.
    pub fn from_map_v2(map: &Map, skill: Skill) -> Self {
        Self::from_things_v2(
            map.things().iter().map(|t: &Thing| (t.kind, t.options)),
            skill_bit(skill),
        )
    }
    /// [`Self::from_thing_records`] for the DSIM40 reader: the wire-24 count of
    /// [`Self::from_map_v2`] at `skill`.
    pub fn from_thing_records_v2(records: &[u8], skill: Skill) -> Option<Self> {
        if records.len() % 10 != 0 {
            return None;
        }
        Some(Self::from_things_v2(
            records.chunks_exact(10).map(|r| {
                (
                    i16::from_le_bytes([r[6], r[7]]),
                    u16::from_le_bytes([r[8], r[9]]),
                )
            }),
            skill_bit(skill),
        ))
    }
    fn from_things_v2(things: impl IntoIterator<Item = (i16, u16)>, bit: u16) -> Self {
        let things: Vec<(i16, u16)> = things.into_iter().collect();
        let mut b = Self::from_things_with(things.iter().copied(), bit);
        b.monsters += things
            .iter()
            .filter(|&&(kind, options)| kind == BARON && options & 16 == 0 && options & bit != 0)
            .count();
        b
    }
    /// Count the bounds of a raw THINGS lump in the DSIM `map.things` layout (ten
    /// bytes per record: x, y, angle, type and options as little-endian `i16`);
    /// `None` when the length is not a whole number of records.
    pub fn from_thing_records(records: &[u8]) -> Option<Self> {
        if records.len() % 10 != 0 {
            return None;
        }
        Some(Self::from_things(records.chunks_exact(10).map(|r| {
            (
                i16::from_le_bytes([r[6], r[7]]),
                u16::from_le_bytes([r[8], r[9]]),
            )
        })))
    }
    /// The teleport landings (doomednum 14) a map spawns under wire 24 at
    /// `skill`, counted with the same single-player filter and skill bit as
    /// [`Self::from_map_v2`] (M4b steps 4 and 6). Wire 23 refuses doomednum 14 at
    /// spawn and never counts it, so the bounds of the struct stay those of wire
    /// 23.
    pub fn landings(map: &Map, skill: Skill) -> usize {
        let bit = skill_bit(skill);
        map.things()
            .iter()
            .filter(|t| t.kind == LANDING && t.options & 16 == 0 && t.options & bit != 0)
            .count()
    }
    /// The transient allowance of wire 24 (M4b step 4): the actor slots left for
    /// actors no THING places (teleport fogs, puffs, blood, missiles) when the player,
    /// every placed monster, its drop, every pickup, barrel, decoration and landing
    /// are all live at once: `slots − (1 + K + R + P + B + D + L)`. `None` when the
    /// placed things alone exceed `slots`. Wire 24 refuses at load a map whose
    /// allowance is below 2, the two fogs of one teleport; beyond that the
    /// store-full witness refuses the tic whose spawns find no slot.
    pub const fn transient_allowance(&self, landings: usize, slots: usize) -> Option<usize> {
        let placed = 1
            + self.monsters
            + self.droppers
            + self.pickups
            + self.barrels
            + self.statics
            + landings;
        slots.checked_sub(placed)
    }
    fn from_things(things: impl IntoIterator<Item = (i16, u16)>) -> Self {
        Self::from_things_with(things, MEDIUM_BIT)
    }
    fn from_things_with(things: impl IntoIterator<Item = (i16, u16)>, bit: u16) -> Self {
        let mut b = Self::default();
        for (kind, options) in things {
            // The first two clauses only skip early: kinds 1–4 and 11 are in no
            // counted category below (`tests::starts_are_in_no_counted_category`), so
            // the filter counts the same without them (p_mobj.c:718–737).
            if (1..=4).contains(&kind) || kind == 11 || options & 16 != 0 || options & bit == 0 {
                continue;
            }
            if EPISODE1_MONSTERS.contains(&kind) {
                b.monsters += 1;
                b.droppers += usize::from(EPISODE1_DROPPERS.contains(&kind));
            } else if EPISODE1_PICKUPS.contains(&kind) {
                b.pickups += 1;
                b.keycards += usize::from(EPISODE1_KEYCARDS.contains(&kind));
                b.countitems += usize::from(counts_item(kind));
            } else if kind == EPISODE1_BARREL {
                b.barrels += 1;
            } else if is_static(kind) {
                b.statics += 1;
            }
        }
        b
    }
    /// Live world items: the placed pickups plus one drop per dropping monster.
    pub const fn world_items(&self) -> usize {
        self.pickups + self.droppers
    }
    /// The `bonuscount` bound: `BONUSADD` per pickup and drop, and a second one
    /// per keycard.
    pub const fn max_bonus(&self) -> usize {
        BONUS_ADD * (self.pickups + self.droppers + self.keycards)
    }
    /// The per-operation monster sight-query allowance of decision D8:
    /// `8·(K + 1)`, eight per monster and one more share, and never below the
    /// older policies' constant 64, so that every workload an older policy admits
    /// is admitted (a Rust-only work witness, not source behaviour).
    pub const fn sight_queries(&self) -> usize {
        let queries = 8 * (self.monsters + 1);
        if queries > 64 { queries } else { 64 }
    }
    /// The sight queries one explosion may make (decision F6 with per-map values):
    /// the player, every monster and every barrel (a rocket's window may hold all
    /// of them), each revisited once per relinking monster: `(1 + K + B)·(1 + K)`,
    /// and never below the barrels policy's 216 (`EXPLOSION_SIGHT_QUERIES`).
    pub const fn explosion_sight_queries(&self) -> usize {
        let queries = (1 + self.monsters + self.barrels) * (1 + self.monsters);
        if queries > crate::EXPLOSION_SIGHT_QUERIES {
            queries
        } else {
            crate::EXPLOSION_SIGHT_QUERIES
        }
    }
    /// The roster term of the explosion's defensive visit bound
    /// (`9·order·(K + 2)`): `K`, and never below the older policies' 8.
    pub const fn visit_roster(&self) -> usize {
        if self.monsters > crate::MAX_PLAYER_MONSTERS {
            self.monsters
        } else {
            crate::MAX_PLAYER_MONSTERS
        }
    }
}
fn counts_item(kind: i16) -> bool {
    OriginalActorType::all().any(|ty| {
        let d = ty.descriptor();
        d.doomed_number == i32::from(kind) && d.flags & crate::world_pickups::MF_COUNTITEM != 0
    })
}
fn is_static(kind: i16) -> bool {
    EPISODE1_EXTRA_STATICS.contains(&kind)
        || crate::E1M1_DECORATIONS
            .iter()
            .any(|ty| ty.descriptor().doomed_number == i32::from(kind))
}

impl crate::Simulation {
    /// Copy of the source `powers` and `cards` of the Episode 1 policy; no
    /// mutable player alias escapes. `PlayerBodyError::Profile` under every other
    /// policy.
    pub fn original_player_powers(
        &self,
        _slot: crate::PlayerSlot,
    ) -> Result<crate::PlayerPowers, crate::PlayerBodyError> {
        self.original_player
            .as_ref()
            .and_then(|p| p.powers)
            .ok_or(crate::PlayerBodyError::Profile)
    }
    /// The per-map bounds of the Episode 1 policy, `None` under every other
    /// policy.
    pub fn episode1_bounds(&self) -> Option<Episode1Bounds> {
        self.world.episode1
    }
    /// Sector `i`'s runtime `floorpic` as DSIM39 encodes it (decision D2): the
    /// normalized name of its floor flat, changed by the Episode 1 movers.
    pub(crate) fn floorpic(&self, i: usize) -> [u8; 8] {
        self.specials
            .as_ref()
            .and_then(|s| s.sectors.get(i))
            .map_or_else(
                || crate::sector_thinkers::normalized_flat(&self.world.map().sectors()[i].flats[0]),
                |s| s.floorpic,
            )
    }
    /// The decorations one map may spawn: `D` under the Episode 1 policy,
    /// [`crate::MAX_DECORATIONS`] under every other one.
    pub(crate) fn decoration_limit(&self) -> usize {
        self.world
            .episode1
            .map_or(crate::MAX_DECORATIONS, |b| b.statics)
    }
    /// The exploding barrels one map may spawn: `B` under the Episode 1 policy,
    /// [`crate::MAX_BARRELS`] under every other one.
    pub(crate) fn barrel_limit(&self) -> usize {
        self.world
            .episode1
            .map_or(crate::MAX_BARRELS, |b| b.barrels)
    }
}

/// The power countdowns of `P_PlayerThink`'s live branch (p_user.c:339–353),
/// after `P_MovePsprites` and before the `damagecount`/`bonuscount` decays
/// ([`crate::world_pickups::bonus_tick`]), under the Episode 1 policy only:
/// strength grows while nonzero; invulnerability, invisibility, infrared and
/// ironfeet count down; `pw_allmap` never counts. When invisibility runs out the
/// body loses `MF_SHADOW` (1.10 clears it only then, 345–347; the blinking is
/// the colormap's, 363–383, presentation). A dead player runs `P_DeathThink`
/// and returns before the counters (258–262). The counters' domains are checked
/// first, so a refused call changes nothing.
pub(crate) fn powers_tick(
    player: Option<&mut crate::original_player::PlayerRuntime>,
    actors: &mut crate::ThinkerStore<crate::Actor>,
    think_live: bool,
) -> Result<(), crate::SimError> {
    let Some(p) = player else {
        return Ok(());
    };
    let mo = p.mo;
    let Some(powers) = p.powers.as_mut() else {
        return Ok(());
    };
    validate_powers(powers)?;
    if !think_live {
        return Ok(());
    }
    let id = mo.ok_or(crate::PlayerBodyError::Binding)?;
    let expiring = powers.powers[2] == 1;
    if expiring {
        // Resolve the body before any write.
        actors
            .get(id)
            .and_then(crate::Actor::life)
            .ok_or(crate::PlayerBodyError::Binding)?;
    }
    if powers.powers[1] != 0 {
        powers.powers[1] += 1;
    }
    for i in [0, 2, 5, 3] {
        if powers.powers[i] != 0 {
            powers.powers[i] -= 1;
        }
    }
    if expiring {
        if let Some(life) = actors.get_mut(id).and_then(|a| a.life.as_mut()) {
            life.flags &= !crate::original_player::MF_SHADOW;
        }
    }
    Ok(())
}
/// The Episode 1 domains of the power counters (no invulnerability or berserk in
/// Episode 1 single player; `pw_allmap` 0 or 1). The cards are booleans; the
/// skull keys, absent from Episode 1, are read by the locked doors as the source
/// reads them and checked by the DSIM39 reader.
pub(crate) fn validate_powers(p: &crate::PlayerPowers) -> Result<(), crate::SimError> {
    use crate::world_pickups::{
        INFRATICS, INVISTICS, IRONTICS, POWER_INVULNERABILITY, POWER_STRENGTH,
    };
    if p.powers[POWER_INVULNERABILITY] != 0
        || p.powers[POWER_STRENGTH] != 0
        || !(0..=INVISTICS).contains(&p.powers[2])
        || !(0..=IRONTICS).contains(&p.powers[3])
        || !(0..=1).contains(&p.powers[4])
        || !(0..=INFRATICS).contains(&p.powers[5])
    {
        return Err(crate::PlayerBodyError::Invariant.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins why the first two clauses of the single-player filter in
    /// `from_things_with` (`(1..=4).contains(&kind) || kind == 11`) never change a
    /// count: the deathmatch start 11 and the player starts 1–4 are
    /// `P_SpawnMapThing`'s own early-return cases (p_mobj.c:718–737), not spawned
    /// things, and they are in no category the loop counts after the filter, so
    /// skipping them early and falling through to the empty `else` count the same.
    /// This is also the equivalence proof of the mutant `||` → `&&` at that filter
    /// (Mutants run 43543402068).
    #[test]
    fn starts_are_in_no_counted_category() {
        for kind in [1_i16, 2, 3, 4, 11] {
            assert!(!EPISODE1_MONSTERS.contains(&kind), "{kind}");
            assert!(!EPISODE1_PICKUPS.contains(&kind), "{kind}");
            assert_ne!(kind, EPISODE1_BARREL);
            assert!(!is_static(kind), "{kind}");
            // Hence a lump of nothing but starts counts as an empty map.
            assert_eq!(
                Episode1Bounds::from_things_with([(kind, 7)], 2),
                Episode1Bounds::default()
            );
        }
    }
}
