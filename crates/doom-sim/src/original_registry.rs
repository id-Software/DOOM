//! Original definitions and explicit execution roles; no native fixture aliases.
use crate::{identity::*, *};
use std::{error::Error, fmt, sync::OnceLock};

/// Immutable original information package identified by DSIM-v21.
pub const ORIGINAL_REGISTRY_SCHEMA: &[u8] = b"DOOM-INFO-1.10-v1";
/// Explicit synthetic context, separate from original actor identities.
pub const ORIGINAL_CONTEXT_SCHEMA: &[u8] = b"UNBOUND-BODY-v1";

/// Execution policy identifier, independent of diagnostic format version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OriginalExecutionProfile {
    /// Existing normal Demon closure with bounded unbound bodies and effects.
    DemonNormalV1,
    /// One genuine Medium/commercial player body; no input/inflictor/roster execution.
    PlayerBodyLifecycleV1,
    /// One grounded original player with bounded physical thrust and axis-wall motion.
    PlayerBodyKnockbackV1,
    /// One original player's bounded commands and source reaction-time gate.
    PlayerCommandMovementV1,
    /// Fixed active player0 and one canonical Demon: visual acquisition/nonmelee chase.
    PlayerDemonAcquisitionV1,
    /// The same fixed roster with source-timed autonomous Demon physical melee.
    PlayerDemonMeleeV1,
    /// The fixed combat roster with genuine source pistol/psprite execution.
    PlayerDemonPistolV1,
    /// Four source weapons sharing the genuine player and autonomous Demon.
    PlayerDemonWeaponsV1,
    /// World pickups composed with the existing player/four-weapon/Demon closure.
    PlayerDemonPickupsV1,
    /// Original animated armor and source protection in the shared combat world.
    PlayerDemonArmorV1,
    /// Original animated bonuses and counted items in the shared player world.
    PlayerDemonBonusesV1,
    /// The bonus world plus original former humans, a bounded mixed roster,
    /// source infighting and P_KillMobj drops.
    PlayerMonstersV1,
    /// The former-human world plus the original Imp and its fireball.
    PlayerMonstersImpV1,
    /// The Imp world at real-map scale: wall-derived travel bounds inside a walled
    /// rectangle up to 2,048 units, 320 actor slots and 208 live fireballs.
    PlayerMonstersMapScaleV1,
    /// The map-scale world with two-sided lines, static floor/ceiling heights and
    /// vertical movement: steps, ledges, windows, fences and falls inside the
    /// walled rectangle. Moving sectors, specials and sky ceilings stay rejected.
    PlayerMonstersHeightsV1,
    /// The heights world with linedefs of any direction inside a closed boundary of
    /// one-sided walls: wall slides at any angle, diagonal risers, windows, pillars
    /// and corridors. The corner jump is the one refused legal source move.
    PlayerMonstersDiagonalV1,
    /// The diagonal world at the scale of classic maps: every static-geometry cap
    /// taken from the WAD format's own `short` fields, an arithmetic certificate
    /// computed from the loaded map, player starts admitted as records, and the
    /// 129th intercept of one trace refused (`MapScaleError::Intercepts`), the
    /// second refused legal source behavior after the corner jump.
    PlayerMonstersRealMapV1,
    /// The real-map world plus sky ceilings and the static sector specials of the
    /// M3h7 note §4.2: light thinkers in the shared thinker order, damage floors,
    /// secrets and one-sided scrolling walls (line special 48, presentation only),
    /// with sky-ceiling fireball removal and sky-ended hitscans. Every other line
    /// special, sector specials 10, 11, 14 and `BT_USE` stay rejected.
    PlayerMonstersSpecialsV1,
    /// The specials world plus the doors, lifts, floors, switches and exit of the
    /// M3h8 note §4.0: line specials 1, 63, 109, 88, 36, 38 and 11 under the §7
    /// load rules, moving floors and ceilings through `T_MovePlane` and
    /// `P_ChangeSector`, with the sector-thinker slot class bounded by the light
    /// sectors plus twice the mover sectors. Every other line special, a two-sided
    /// scrolling wall, crushers and sky ceilings over movers stay rejected.
    PlayerMonstersMoversV1,
    /// The movers world plus the two E1M1 admissions of the M3h9 note §5.1: line
    /// special 48 on two-sided lines, blocking or not (`P_SpawnSpecials`,
    /// p_spec.c:1336–1346, lists every 48 line; `P_UpdateSpecials`, 1114–1124, only
    /// scrolls its front side's `textureoffset`, read by no game code; decision E1),
    /// and the seven static decoration types of info.c (DoomEd 2028, 48, 35, 24, 15,
    /// 10, 12: `tics` −1, no action, not shootable or special, `MF_SOLID` or flags 0;
    /// decision E2), at most 32 per map. Barrels, every other THING type, crushers
    /// and the other deferred specials stay rejected at load.
    PlayerMonstersE1M1V1,
    /// The E1M1 world plus the M3h9b admissions of the M3h9b note §5.1, with the
    /// bounds of decision F2: up to 64 pickups (bonuses up to 6 × 64) and 384
    /// actor slots, every THING angle (decision F8, converted as
    /// `P_SpawnMapThing`, p_mobj.c:794, does) and the relaxed spawn rule S′
    /// (decision F3: centre parity and the vertex rectangle for every spawn, a
    /// margin of `(31 − r)·(|dx| + |dy|)` against each one-sided wall a mover's
    /// open box meets). Captures are DSIM37. Exploding barrels (DoomEd 2035) are
    /// the headline admission of the policy.
    PlayerMonstersBarrelsV1,
    /// The barrels world, byte for byte, plus the three behaviours of decision G4
    /// (M3h9c) that let a whole `P_SetupLevel` and `P_Ticker` be compared tic for
    /// tic: the player body spawned when `P_LoadThings` meets the type-1 THINGS
    /// record, with `ANG45 * (angle/45)` (p_setup.c:313–346, p_mobj.c:642–686,
    /// 729–736), the `damagecount` decay of `P_PlayerThink` (p_user.c:355–356)
    /// and `P_DeathThink` (p_user.c:182–228: the turn toward the attacker, the
    /// dead `damagecount` decay, any command admitted and only `BT_USE` read).
    /// Captures are DSIM38, the DSIM37 layout under policy 22.
    PlayerMonstersLevelV1,
    /// Episode 1, part 1 (M4a, decisions H2–H4): the level world of wire 22 with
    /// the load-time proofs that refuse Episode 1 maps replaced by per-tic
    /// witnesses (H3) and the fixed roster, pickup, decoration and barrel bounds
    /// replaced by per-map values derived from the THINGS lump, with 1,024 actor
    /// slots (H4). Captures are DSIM39, read by `Diagnostic::decode_episode1`.
    PlayerMonstersEpisode1V1,
    /// Episode 1, part 2 (M4b, decisions I1–I4): a strict superset of wire 23
    /// whose constructor takes the map number (1–9) and the skill, recorded in
    /// DSIM40 (`level.map`, `actors.options`), read by
    /// `Diagnostic::decode_episode1_v2`. Its new behaviours (teleport, the Baron,
    /// `A_BossDeath`, hard skill) each have their own predicate and are refused
    /// until admitted.
    PlayerMonstersEpisode1V2,
}
impl OriginalExecutionProfile {
    /// Stable wire policy ID; not the Rust Profile discriminant or DSIM version.
    pub const fn wire_id(self) -> u32 {
        match self {
            Self::DemonNormalV1 => 1,
            Self::PlayerBodyLifecycleV1 => 2,
            Self::PlayerBodyKnockbackV1 => 3,
            Self::PlayerCommandMovementV1 => 4,
            Self::PlayerDemonAcquisitionV1 => 5,
            Self::PlayerDemonMeleeV1 => 6,
            Self::PlayerDemonPistolV1 => 7,
            Self::PlayerDemonWeaponsV1 => 8,
            Self::PlayerDemonPickupsV1 => 9,
            Self::PlayerDemonArmorV1 => 10,
            Self::PlayerDemonBonusesV1 => 11,
            Self::PlayerMonstersV1 => 12,
            Self::PlayerMonstersImpV1 => 13,
            Self::PlayerMonstersMapScaleV1 => 14,
            Self::PlayerMonstersHeightsV1 => 15,
            Self::PlayerMonstersDiagonalV1 => 16,
            Self::PlayerMonstersRealMapV1 => 17,
            Self::PlayerMonstersSpecialsV1 => 18,
            Self::PlayerMonstersMoversV1 => 19,
            Self::PlayerMonstersE1M1V1 => 20,
            Self::PlayerMonstersBarrelsV1 => 21,
            Self::PlayerMonstersLevelV1 => 22,
            Self::PlayerMonstersEpisode1V1 => 23,
            Self::PlayerMonstersEpisode1V2 => 24,
        }
    }
    /// Exact immutable execution-context schema; not an actor-instance identity.
    pub const fn context_schema(self) -> &'static [u8] {
        match self {
            Self::DemonNormalV1 => ORIGINAL_CONTEXT_SCHEMA,
            Self::PlayerBodyLifecycleV1 => PLAYER_BODY_CONTEXT_SCHEMA,
            Self::PlayerBodyKnockbackV1 => PLAYER_KNOCKBACK_CONTEXT_SCHEMA,
            Self::PlayerCommandMovementV1 => PLAYER_COMMAND_CONTEXT_SCHEMA,
            Self::PlayerDemonAcquisitionV1 => PLAYER_DEMON_CONTEXT_SCHEMA,
            Self::PlayerDemonMeleeV1 => PLAYER_DEMON_MELEE_CONTEXT_SCHEMA,
            Self::PlayerDemonPistolV1 => PLAYER_DEMON_PISTOL_CONTEXT_SCHEMA,
            Self::PlayerDemonWeaponsV1 => PLAYER_DEMON_WEAPONS_CONTEXT_SCHEMA,
            Self::PlayerDemonPickupsV1 => PLAYER_DEMON_PICKUPS_CONTEXT_SCHEMA,
            Self::PlayerDemonArmorV1 => PLAYER_DEMON_ARMOR_CONTEXT_SCHEMA,
            Self::PlayerDemonBonusesV1 => PLAYER_DEMON_BONUSES_CONTEXT_SCHEMA,
            Self::PlayerMonstersV1 => PLAYER_MONSTERS_CONTEXT_SCHEMA,
            Self::PlayerMonstersImpV1 => PLAYER_IMPS_CONTEXT_SCHEMA,
            Self::PlayerMonstersMapScaleV1 => PLAYER_MAP_SCALE_CONTEXT_SCHEMA,
            Self::PlayerMonstersHeightsV1 => PLAYER_HEIGHTS_CONTEXT_SCHEMA,
            Self::PlayerMonstersDiagonalV1 => PLAYER_DIAGONAL_CONTEXT_SCHEMA,
            Self::PlayerMonstersRealMapV1 => PLAYER_REAL_MAP_CONTEXT_SCHEMA,
            Self::PlayerMonstersSpecialsV1 => PLAYER_SPECIALS_CONTEXT_SCHEMA,
            Self::PlayerMonstersMoversV1 => PLAYER_MOVERS_CONTEXT_SCHEMA,
            Self::PlayerMonstersE1M1V1 => PLAYER_E1M1_CONTEXT_SCHEMA,
            Self::PlayerMonstersBarrelsV1 => PLAYER_BARRELS_CONTEXT_SCHEMA,
            Self::PlayerMonstersLevelV1 => PLAYER_LEVEL_CONTEXT_SCHEMA,
            Self::PlayerMonstersEpisode1V1 => PLAYER_EPISODE1_CONTEXT_SCHEMA,
            Self::PlayerMonstersEpisode1V2 => PLAYER_EPISODE1_V2_CONTEXT_SCHEMA,
        }
    }
    pub(crate) const fn has_player_armor(self) -> bool {
        matches!(
            self,
            Self::PlayerDemonArmorV1
                | Self::PlayerDemonBonusesV1
                | Self::PlayerMonstersV1
                | Self::PlayerMonstersImpV1
                | Self::PlayerMonstersMapScaleV1
                | Self::PlayerMonstersHeightsV1
                | Self::PlayerMonstersDiagonalV1
                | Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    pub(crate) const fn has_player_bonuses(self) -> bool {
        matches!(
            self,
            Self::PlayerDemonBonusesV1
                | Self::PlayerMonstersV1
                | Self::PlayerMonstersImpV1
                | Self::PlayerMonstersMapScaleV1
                | Self::PlayerMonstersHeightsV1
                | Self::PlayerMonstersDiagonalV1
                | Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// Original former humans, a bounded mixed roster, infighting and drops.
    pub(crate) const fn has_monsters(self) -> bool {
        matches!(
            self,
            Self::PlayerMonstersV1
                | Self::PlayerMonstersImpV1
                | Self::PlayerMonstersMapScaleV1
                | Self::PlayerMonstersHeightsV1
                | Self::PlayerMonstersDiagonalV1
                | Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// The original Imp and its fireball in the mixed roster.
    pub(crate) const fn has_imps(self) -> bool {
        matches!(
            self,
            Self::PlayerMonstersImpV1
                | Self::PlayerMonstersMapScaleV1
                | Self::PlayerMonstersHeightsV1
                | Self::PlayerMonstersDiagonalV1
                | Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// Wall-derived travel bounds, 320 slots and 208 live fireballs inside a walled
    /// rectangle; the historical 994-unit margins and travel envelope do not apply.
    pub(crate) const fn has_map_scale(self) -> bool {
        matches!(
            self,
            Self::PlayerMonstersMapScaleV1
                | Self::PlayerMonstersHeightsV1
                | Self::PlayerMonstersDiagonalV1
                | Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// Two-sided lines, static floor/ceiling heights, vertical movement, the
    /// landing sound and cross-sector noise inside the walled rectangle; the
    /// flat-world witnesses of the older policies do not apply.
    pub(crate) const fn has_heights(self) -> bool {
        matches!(
            self,
            Self::PlayerMonstersHeightsV1
                | Self::PlayerMonstersDiagonalV1
                | Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// Linedefs of any direction inside a closed one-sided boundary, the parity
    /// spawn rule and the escape witness of the M3h5 note.
    pub(crate) const fn has_diagonals(self) -> bool {
        matches!(
            self,
            Self::PlayerMonstersDiagonalV1
                | Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// The real-map policy: format-sized static geometry, the arithmetic and
    /// slide-window certificates, starts as records, the intercept witness.
    pub(crate) const fn has_real_map(self) -> bool {
        matches!(
            self,
            Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// The specials policy (M3h7 Stage A) and its supersets: sky ceilings, the
    /// static sector specials and their light thinkers, secrets, damage floors and
    /// one-sided scrolling walls. Every other policy rejects them at load.
    pub(crate) const fn has_specials(self) -> bool {
        matches!(
            self,
            Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// The movers policy (M3h8 Stage B) and its E1M1 superset: doors, lifts,
    /// floors, switches and the exit (line specials 1, 63, 109, 88, 36, 38, 11)
    /// under the §7 load rules, moving sector heights and `P_ChangeSector`. Every
    /// other policy rejects them.
    pub(crate) const fn has_movers(self) -> bool {
        matches!(
            self,
            Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// The E1M1 policy (M3h9 note §5.1): two-sided scrolling walls (48) and the
    /// seven static decoration types, under DSIM36. Every policy before wire 20
    /// rejects them at load; the barrels policy (wire 21) inherits the admission.
    pub(crate) const fn has_e1m1(self) -> bool {
        matches!(
            self,
            Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// The barrels policy (M3h9b note §5.1): exploding barrels, up to 64 pickups,
    /// 384 actor slots, every THING angle and the spawn rule S′ (decision F3),
    /// under DSIM37; the level policy (wire 22) inherits all of it. Every other
    /// policy keeps its bounds and rules.
    pub(crate) const fn has_barrels(self) -> bool {
        matches!(
            self,
            Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// The level policy (M3h9c, decision G4, wire 22 only): the barrels world with
    /// the player body spawned at its THINGS record's lump position with its angle
    /// (`P_LoadThings` → `P_SpawnMapThing` → `P_SpawnPlayer`, p_setup.c:313–346,
    /// p_mobj.c:729–736 and 642–686), the `damagecount` decay of `P_PlayerThink`
    /// (p_user.c:355–356) and `P_DeathThink` (p_user.c:182–228) for a dead player,
    /// under DSIM38. Every older policy keeps its spawn order, counters and dead
    /// player contract.
    pub(crate) const fn has_level(self) -> bool {
        matches!(
            self,
            Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// The Episode 1 policy (M4a, wire 23 only, decisions H2–H4): the level world
    /// with the per-map bounds of `episode1::Episode1Bounds` and 1,024 actor slots,
    /// the arithmetic certificate's clauses (i), (ii) and (vii) and the ±512
    /// static-height rule replaced by checked per-tic arithmetic, an optional
    /// one-sided boundary, and the mover load rules of
    /// `mover_rules::validate_episode1`, under DSIM39. Every older policy keeps its
    /// bounds, proofs and rules byte for byte. Wire 24 inherits all of it (M4b,
    /// decision I2: a strict superset); a site that selects a schema or an exact
    /// version tests [`Self::has_episode1_v2`] first.
    pub(crate) const fn has_episode1(self) -> bool {
        matches!(
            self,
            Self::PlayerMonstersEpisode1V1 | Self::PlayerMonstersEpisode1V2
        )
    }
    /// Episode 1, part 2 (M4b, wire 24 only, decision I2): the identity of the
    /// policy: the map number (1–9) held by the simulation, DSIM40 captures
    /// (`level.map` after `level.secret_exit`) and the
    /// `PLAYER-MONSTERS-EPISODE1-2-v1` context. Every older policy carries no map
    /// number and keeps its schema.
    pub(crate) const fn has_episode1_v2(self) -> bool {
        matches!(self, Self::PlayerMonstersEpisode1V2)
    }
    /// Line special 97 (`EV_Teleport`, p_telept.c:47–131), the landing
    /// (`MT_TELEPORTMAN`, doomednum 14), the teleport fog and the telefrag (M4b
    /// decisions I3, I7 and I8): wire 24 only, admitted in M4b step 4. Until then
    /// the load rules refuse the special and the thing under every policy.
    pub(crate) const fn has_teleport(self) -> bool {
        matches!(self, Self::PlayerMonstersEpisode1V2)
    }
    /// The Baron of Hell (`MT_BRUISER`, doomednum 3003), `A_BruisAttack`
    /// (p_enemy.c:979–996) and its fireball (`MT_BRUISERSHOT`): wire 24 only,
    /// admitted in M4b step 5. Every older policy refuses doomednum 3003 in
    /// `spawn_map_things` (`MapSpawnError::UnsupportedType`).
    pub(crate) const fn has_baron(self) -> bool {
        matches!(self, Self::PlayerMonstersEpisode1V2)
    }
    /// `A_BossDeath` (p_enemy.c:1609–1757, decision I9): wire 24 only, admitted in
    /// M4b step 5 with the Baron, whose `S_BOSS_DIE7` alone names it.
    pub(crate) const fn has_boss_death(self) -> bool {
        matches!(self, Self::PlayerMonstersEpisode1V2)
    }
    /// Skill 3 (hard, THING option bit 4; decision I4): wire 24 only, admitted in
    /// M4b step 6. In single player nothing but `P_SpawnMapThing`'s option bit
    /// (p_mobj.c:743–751) differs between skills 2 and 3 (M4b note §3 part C);
    /// every older player policy refuses any options but the default (medium).
    pub(crate) const fn has_hard_skill(self) -> bool {
        matches!(self, Self::PlayerMonstersEpisode1V2)
    }
    /// A Demon or Spectre bite (`A_SargAttack`, p_enemy.c:935–948) on a body that
    /// carries `A_Saw`'s `MF_JUSTATTACKED`: wire 24 only (M4b step 7, the E1M5
    /// `M5WALK3` refusal). `A_Saw` sets the flag (p_pspr.c:542) only after a hit
    /// (519–523) during `P_PlayerThink`, which `P_Ticker` runs before every thinker
    /// (p_tick.c:150–152), so a Demon can bite the flagged body later in the same
    /// tic; the next `P_PlayerThink` clears it before its dead test
    /// (p_user.c:249–255). Nothing on the bite path reads the flag: `A_FaceTarget`
    /// reads only `MF_SHADOW` (p_enemy.c:782–796), `P_CheckMeleeRange` no flag
    /// (174–192), `P_DamageMobj` `MF_SHOOTABLE`, `MF_SKULLFLY` and `MF_NOCLIP`
    /// (p_inter.c:775–918), and `P_KillMobj` leaves it set (668–770); the only
    /// readers are `P_PlayerThink` and `A_Chase` on its own actor (p_enemy.c:716).
    /// Every older policy keeps refusing such a bite with
    /// `PlayerBodyError::Invariant`.
    pub(crate) const fn has_bite_on_lunging_body(self) -> bool {
        matches!(self, Self::PlayerMonstersEpisode1V2)
    }
    pub(crate) const fn has_world_pickups(self) -> bool {
        matches!(
            self,
            Self::PlayerDemonPickupsV1
                | Self::PlayerDemonArmorV1
                | Self::PlayerDemonBonusesV1
                | Self::PlayerMonstersV1
                | Self::PlayerMonstersImpV1
                | Self::PlayerMonstersMapScaleV1
                | Self::PlayerMonstersHeightsV1
                | Self::PlayerMonstersDiagonalV1
                | Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    pub(crate) const fn has_player_body(self) -> bool {
        matches!(
            self,
            Self::PlayerBodyLifecycleV1
                | Self::PlayerBodyKnockbackV1
                | Self::PlayerCommandMovementV1
                | Self::PlayerDemonAcquisitionV1
                | Self::PlayerDemonMeleeV1
                | Self::PlayerDemonPistolV1
                | Self::PlayerDemonWeaponsV1
                | Self::PlayerDemonPickupsV1
                | Self::PlayerDemonArmorV1
                | Self::PlayerDemonBonusesV1
                | Self::PlayerMonstersV1
                | Self::PlayerMonstersImpV1
                | Self::PlayerMonstersMapScaleV1
                | Self::PlayerMonstersHeightsV1
                | Self::PlayerMonstersDiagonalV1
                | Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    pub(crate) const fn has_physical_player(self) -> bool {
        matches!(
            self,
            Self::PlayerBodyKnockbackV1
                | Self::PlayerCommandMovementV1
                | Self::PlayerDemonAcquisitionV1
                | Self::PlayerDemonMeleeV1
                | Self::PlayerDemonPistolV1
                | Self::PlayerDemonWeaponsV1
                | Self::PlayerDemonPickupsV1
                | Self::PlayerDemonArmorV1
                | Self::PlayerDemonBonusesV1
                | Self::PlayerMonstersV1
                | Self::PlayerMonstersImpV1
                | Self::PlayerMonstersMapScaleV1
                | Self::PlayerMonstersHeightsV1
                | Self::PlayerMonstersDiagonalV1
                | Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    pub(crate) const fn has_player_weapons(self) -> bool {
        matches!(
            self,
            Self::PlayerDemonPistolV1
                | Self::PlayerDemonWeaponsV1
                | Self::PlayerDemonPickupsV1
                | Self::PlayerDemonArmorV1
                | Self::PlayerDemonBonusesV1
                | Self::PlayerMonstersV1
                | Self::PlayerMonstersImpV1
                | Self::PlayerMonstersMapScaleV1
                | Self::PlayerMonstersHeightsV1
                | Self::PlayerMonstersDiagonalV1
                | Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
    /// The four-weapon pack (fists, shotgun, chaingun, manual switching): every
    /// weapon policy except the pistol-only one.
    pub(crate) const fn has_weapon_pack(self) -> bool {
        self.has_player_weapons() && !matches!(self, Self::PlayerDemonPistolV1)
    }
    pub(crate) const fn has_player_commands(self) -> bool {
        matches!(
            self,
            Self::PlayerCommandMovementV1
                | Self::PlayerDemonAcquisitionV1
                | Self::PlayerDemonMeleeV1
                | Self::PlayerDemonPistolV1
                | Self::PlayerDemonWeaponsV1
                | Self::PlayerDemonPickupsV1
                | Self::PlayerDemonArmorV1
                | Self::PlayerDemonBonusesV1
                | Self::PlayerMonstersV1
                | Self::PlayerMonstersImpV1
                | Self::PlayerMonstersMapScaleV1
                | Self::PlayerMonstersHeightsV1
                | Self::PlayerMonstersDiagonalV1
                | Self::PlayerMonstersRealMapV1
                | Self::PlayerMonstersSpecialsV1
                | Self::PlayerMonstersMoversV1
                | Self::PlayerMonstersE1M1V1
                | Self::PlayerMonstersBarrelsV1
                | Self::PlayerMonstersLevelV1
                | Self::PlayerMonstersEpisode1V1
                | Self::PlayerMonstersEpisode1V2
        )
    }
}
/// A non-source definition supplied by the explicit execution context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextRole {
    /// Unbound damage/perception body; never an original player identity.
    UnboundBody,
}
/// Definition identity. An actor's generational handle is a separate namespace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionIdentity {
    /// Original mobjtype_t, with pristine metadata at its original ordinal.
    Original(OriginalActorType),
    /// An explicitly identified context definition, not a canonical count sentinel.
    Context(ContextRole),
}
/// Registry/role admission failed; recognition alone never grants execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OriginalRegistryError {
    /// Complete source or context records differ from their identified package.
    Registry,
    /// Candidate is neither an original nor a declared context definition slot.
    Definition(u16),
    /// A valid definition has no executable role in this profile.
    Unsupported(DefinitionIdentity),
    /// State is outside the definition's admitted normal closure.
    State(u16),
    /// Unsupported original graph/action or policy configuration.
    Graph,
    /// Operation requires the explicit original-ID profile or a current actor.
    ProfileOrActor,
    /// Existing shared simulation rejected the operation.
    Runtime(SimError),
}
impl fmt::Display for OriginalRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "original registry: {self:?}")
    }
}
impl Error for OriginalRegistryError {}
impl From<SimError> for OriginalRegistryError {
    fn from(value: SimError) -> Self {
        Self::Runtime(value)
    }
}

#[derive(Clone, Copy)]
struct StateSet([u64; 16]);
impl StateSet {
    const fn empty() -> Self {
        Self([0; 16])
    }
    fn contains(self, state: OriginalState) -> bool {
        let n = usize::from(state.ordinal());
        self.0
            .get(n / 64)
            .is_some_and(|word| word & (1_u64 << (n % 64)) != 0)
    }
    fn insert(&mut self, state: OriginalState) -> Result<(), OriginalRegistryError> {
        let n = usize::from(state.ordinal());
        let word = self.0.get_mut(n / 64).ok_or(OriginalRegistryError::Graph)?;
        *word |= 1_u64 << (n % 64);
        Ok(())
    }
    fn graph(
        roots: impl IntoIterator<Item = OriginalState>,
    ) -> Result<Self, OriginalRegistryError> {
        let mut set = Self::empty();
        for mut current in roots {
            let mut work = 0;
            while current != OriginalState::S_NULL && !set.contains(current) {
                work += 1;
                if work > usize::from(OriginalState::COUNT) {
                    return Err(OriginalRegistryError::Graph);
                }
                set.insert(current)?;
                current = current.descriptor().next;
            }
        }
        Ok(set)
    }
}
struct ActorPolicy {
    normal: StateSet,
    direct: StateSet,
    death: StateSet,
    before_fall: StateSet,
}
impl ActorPolicy {
    fn source(ty: OriginalActorType) -> Result<Self, OriginalRegistryError> {
        let r = ty.states();
        let normal = StateSet::graph([
            r.spawn,
            r.see,
            r.pain,
            r.melee,
            r.missile,
            r.death,
            r.extreme_death,
        ])?;
        let direct = StateSet::graph([r.spawn, r.see, r.melee])?;
        let death = StateSet::graph([r.death, r.extreme_death])?;
        let mut before_fall = StateSet::empty();
        let mut current = r.death;
        for _ in 0..OriginalState::COUNT {
            if current == OriginalState::S_NULL || current.descriptor().action == Some("A_Fall") {
                break;
            }
            if before_fall.contains(current) {
                return Err(OriginalRegistryError::Graph);
            }
            before_fall.insert(current)?;
            current = current.descriptor().next;
        }
        Ok(Self {
            normal,
            direct,
            death,
            before_fall,
        })
    }
}
fn demon_policy() -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
    POLICY
        .get_or_init(|| {
            let p = ActorPolicy::source(OriginalActorType::MT_SERGEANT)?;
            for s in OriginalState::all().filter(|s| p.normal.contains(*s)) {
                if !matches!(
                    s.descriptor().action,
                    None | Some(
                        "A_Look"
                            | "A_Chase"
                            | "A_FaceTarget"
                            | "A_SargAttack"
                            | "A_Pain"
                            | "A_Scream"
                            | "A_Fall"
                    )
                ) {
                    return Err(OriginalRegistryError::Graph);
                }
            }
            if p.normal
                .contains(OriginalActorType::MT_SERGEANT.states().raise)
            {
                return Err(OriginalRegistryError::Graph);
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
/// p_mobj.h/info.c former humans admitted only by `PlayerMonstersV1`.
pub(crate) const FORMER_HUMANS: [OriginalActorType; 3] = [
    OriginalActorType::MT_POSSESSED,
    OriginalActorType::MT_SHOTGUY,
    OriginalActorType::MT_CHAINGUY,
];
/// Original attack callbacks reachable from each former human's normal closure.
fn former_human_attacks(ty: OriginalActorType) -> &'static [&'static str] {
    match ty {
        OriginalActorType::MT_POSSESSED => &["A_PosAttack"],
        OriginalActorType::MT_SHOTGUY => &["A_SPosAttack"],
        _ => &["A_CPosAttack", "A_CPosRefire"],
    }
}
/// Normal spawn/see/pain/missile/death/xdeath closure; raise metadata stays inert.
fn former_human_policy(
    ty: OriginalActorType,
) -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICIES: [OnceLock<Result<ActorPolicy, OriginalRegistryError>>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let index = FORMER_HUMANS
        .iter()
        .position(|t| *t == ty)
        .ok_or(OriginalRegistryError::Graph)?;
    POLICIES[index]
        .get_or_init(|| {
            let p = ActorPolicy::source(ty)?;
            for s in OriginalState::all().filter(|s| p.normal.contains(*s)) {
                let action = s.descriptor().action;
                if !(matches!(
                    action,
                    None | Some(
                        "A_Look"
                            | "A_Chase"
                            | "A_FaceTarget"
                            | "A_Pain"
                            | "A_Scream"
                            | "A_XScream"
                            | "A_Fall"
                    )
                ) || action.is_some_and(|a| former_human_attacks(ty).contains(&a)))
                {
                    return Err(OriginalRegistryError::Graph);
                }
            }
            if p.normal.contains(ty.states().raise) {
                return Err(OriginalRegistryError::Graph);
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
/// Normal Imp closure; raise metadata stays inert (info.c S_TROO_*).
fn imp_policy() -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
    POLICY
        .get_or_init(|| {
            let ty = OriginalActorType::MT_TROOP;
            let p = ActorPolicy::source(ty)?;
            for s in OriginalState::all().filter(|s| p.normal.contains(*s)) {
                if !matches!(
                    s.descriptor().action,
                    None | Some(
                        "A_Look"
                            | "A_Chase"
                            | "A_FaceTarget"
                            | "A_TroopAttack"
                            | "A_Pain"
                            | "A_Scream"
                            | "A_XScream"
                            | "A_Fall"
                    )
                ) {
                    return Err(OriginalRegistryError::Graph);
                }
            }
            if p.normal.contains(ty.states().raise) {
                return Err(OriginalRegistryError::Graph);
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
/// Spawn/explosion closure of the fireball; no state has an action.
fn fireball_policy() -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
    POLICY
        .get_or_init(|| {
            let p = ActorPolicy::source(OriginalActorType::MT_TROOPSHOT)?;
            if OriginalState::all()
                .filter(|s| p.normal.contains(*s))
                .any(|s| s.descriptor().action.is_some())
            {
                return Err(OriginalRegistryError::Graph);
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
/// The teleport fog's closure (`MT_TFOG`, info.c:266–277, 2122–2146): twelve
/// states of 6 tics, none with an action (wire 24, M4b step 4).
fn tfog_policy() -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
    POLICY
        .get_or_init(|| {
            let p = ActorPolicy::source(OriginalActorType::MT_TFOG)?;
            if OriginalState::all()
                .filter(|s| p.normal.contains(*s))
                .any(|s| s.descriptor().action.is_some())
            {
                return Err(OriginalRegistryError::Graph);
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
/// The Baron of Hell's closure (`MT_BRUISER`, info.c:663–684, 1498–1522): spawn,
/// see, pain, melee = missile (`S_BOSS_ATK1`) and death, with `A_BruisAttack` at
/// `S_BOSS_ATK3` and `A_BossDeath` at `S_BOSS_DIE7`; `S_BOSS_RAISE1`–`7` stay
/// metadata (wire 24, M4b step 5).
fn bruiser_policy() -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
    POLICY
        .get_or_init(|| {
            let ty = OriginalActorType::MT_BRUISER;
            let p = ActorPolicy::source(ty)?;
            for s in OriginalState::all().filter(|s| p.normal.contains(*s)) {
                let action = s.descriptor().action;
                let ok = match action {
                    None
                    | Some(
                        "A_Look" | "A_Chase" | "A_FaceTarget" | "A_Pain" | "A_Scream" | "A_Fall",
                    ) => true,
                    Some("A_BruisAttack") => s == OriginalState::S_BOSS_ATK3,
                    Some("A_BossDeath") => s == OriginalState::S_BOSS_DIE7,
                    Some(_) => false,
                };
                if !ok {
                    return Err(OriginalRegistryError::Graph);
                }
            }
            if p.normal.contains(ty.states().raise) {
                return Err(OriginalRegistryError::Graph);
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
/// The Baron fireball's spawn/explosion closure (`MT_BRUISERSHOT`, info.c:658–662,
/// 1524–1548): `S_BRBALL1`/`2` and `S_BRBALLX1`–`3`, no action (no `A_Explode`).
fn bruisershot_policy() -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
    POLICY
        .get_or_init(|| {
            let p = ActorPolicy::source(OriginalActorType::MT_BRUISERSHOT)?;
            if OriginalState::all()
                .filter(|s| p.normal.contains(*s))
                .any(|s| s.descriptor().action.is_some())
            {
                return Err(OriginalRegistryError::Graph);
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
/// The rocket's spawn/explosion closure (info.c:250, 263–265, 1966–1990): `S_ROCKET`
/// and `S_EXPLODE1..3`; `A_Explode` in `S_EXPLODE1` is the one action (Episode 1
/// policy, M4a plan §E.4).
fn rocket_policy() -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
    POLICY
        .get_or_init(|| {
            let p = ActorPolicy::source(OriginalActorType::MT_ROCKET)?;
            for s in OriginalState::all().filter(|s| p.normal.contains(*s)) {
                let action = s.descriptor().action;
                if !(action.is_none()
                    || (s == OriginalState::S_EXPLODE1 && action == Some("A_Explode")))
                {
                    return Err(OriginalRegistryError::Graph);
                }
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
/// The Spectre (`MT_SHADOWS`, info.c:1446–1470): field for field the Demon
/// (`MT_SERGEANT`, info.c:1420–1444) except its DoomEd number (58) and the
/// `MF_SHADOW` flag; the same `S_SARG_*` states, sounds and actions. Its closure
/// must equal the Demon's (Episode 1 policy, M4a plan §E.5).
fn spectre_policy() -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
    POLICY
        .get_or_init(|| {
            let p = ActorPolicy::source(OriginalActorType::MT_SHADOWS)?;
            let demon = demon_policy()?;
            if p.normal.0 != demon.normal.0
                || p.direct.0 != demon.direct.0
                || p.death.0 != demon.death.0
                || p.before_fall.0 != demon.before_fall.0
            {
                return Err(OriginalRegistryError::Graph);
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
/// `MT_SHADOWS`, policy-free (the Episode 1 role gates it).
pub(crate) const fn is_spectre(ty: ActorTypeId) -> bool {
    ty.0 == OriginalActorType::MT_SHADOWS.ordinal()
}
/// `MT_ROCKET`, policy-free (the Episode 1 role gates it).
pub(crate) const fn is_rocket(ty: ActorTypeId) -> bool {
    ty.0 == OriginalActorType::MT_ROCKET.ordinal()
}
/// `MT_BRUISER`, policy-free (the wire-24 role gates it).
pub(crate) const fn is_bruiser(ty: ActorTypeId) -> bool {
    ty.0 == OriginalActorType::MT_BRUISER.ordinal()
}
/// `MT_BRUISERSHOT`, policy-free (the wire-24 role gates it).
pub(crate) const fn is_bruiser_shot(ty: ActorTypeId) -> bool {
    ty.0 == OriginalActorType::MT_BRUISERSHOT.ordinal()
}
/// A missile whose flight the checked thinker pass owns: the Imp fireball
/// (`PlayerMonstersImpV1` and its supersets), the rocket (Episode 1 policy) and
/// the Baron's fireball (wire 24). Policy-free: each type exists only where its
/// role admits it, and no reader of an older version consults this predicate.
pub(crate) const fn is_missile_type(ty: ActorTypeId) -> bool {
    is_fireball(ty) || is_rocket(ty) || is_bruiser_shot(ty)
}
pub(crate) fn is_former_human(ty: ActorTypeId) -> bool {
    FORMER_HUMANS.iter().any(|t| t.ordinal() == ty.0)
}
pub(crate) const fn is_imp(ty: ActorTypeId) -> bool {
    ty.0 == OriginalActorType::MT_TROOP.ordinal()
}
pub(crate) const fn is_fireball(ty: ActorTypeId) -> bool {
    ty.0 == OriginalActorType::MT_TROOPSHOT.ordinal()
}
/// Bullet puffs and blood: spawned at an absolute z by `P_SpawnPuff`/`P_SpawnBlood`
/// and clipped later by `P_ZMovement`, so the vertical invariant excludes them.
pub(crate) const fn is_impact(ty: ActorTypeId) -> bool {
    ty.0 == OriginalActorType::MT_PUFF.ordinal() || ty.0 == OriginalActorType::MT_BLOOD.ordinal()
}
/// Original (non-owner-slot) roster definition with a registry-derived closure.
pub(crate) fn is_extended_monster(ty: ActorTypeId) -> bool {
    is_former_human(ty) || is_imp(ty) || is_spectre(ty) || is_bruiser(ty)
}
/// Autonomous monster definitions executable by the selected player policy.
pub(crate) fn is_policy_monster(policy: OriginalExecutionProfile, ty: ActorTypeId) -> bool {
    ty == owner_slot()
        || (policy.has_monsters() && is_former_human(ty))
        || (policy.has_imps() && is_imp(ty))
        || (policy.has_episode1() && is_spectre(ty))
        || (policy.has_baron() && is_bruiser(ty))
}
fn extended_policy(ty: ActorTypeId) -> Result<&'static ActorPolicy, OriginalRegistryError> {
    if is_imp(ty) {
        imp_policy()
    } else if is_spectre(ty) {
        spectre_policy()
    } else if is_bruiser(ty) {
        bruiser_policy()
    } else {
        former_human_policy(
            OriginalActorType::try_from(ty.0).map_err(|_| OriginalRegistryError::Graph)?,
        )
    }
}
pub(crate) fn body_slot() -> ActorTypeId {
    ActorTypeId(OriginalActorType::COUNT)
}
pub(crate) fn owner_slot() -> ActorTypeId {
    ActorTypeId(OriginalActorType::MT_SERGEANT.ordinal())
}
fn body_properties() -> [i32; 23] {
    let mut properties = *OriginalActorType::MT_PLAYER.properties();
    for (field, value) in [(0, -1), (2, 1000), (8, 0), (16, 16 * 65536), (21, 6)] {
        properties[field] = value;
    }
    properties
}
fn body_definition() -> ActorTypeDef {
    ActorTypeDef {
        name: "CONTEXT_UNBOUND_BODY".into(),
        properties: body_properties(),
    }
}
fn definitions() -> Result<ActorRegistry, OriginalRegistryError> {
    let mut source = ActorRegistry::vanilla();
    source.types.push(body_definition());
    ActorRegistry::new(source.states, source.types).map_err(|_| OriginalRegistryError::Registry)
}
/// Owned immutable source registry and its explicitly selected role contract.
///
/// Every original record is pristine. Context definitions are separate keys,
/// appended after the original portion; this never expands live actor capacity.
#[derive(Clone, Debug)]
pub struct OriginalExecutionRegistry {
    registry: ActorRegistry,
    profile: OriginalExecutionProfile,
}
impl OriginalExecutionRegistry {
    /// Construct the immutable source package and explicitly selected execution policy.
    pub fn new(profile: OriginalExecutionProfile) -> Result<Self, OriginalRegistryError> {
        match profile {
            OriginalExecutionProfile::DemonNormalV1 => {
                demon_policy()?;
            }
            OriginalExecutionProfile::PlayerBodyLifecycleV1
            | OriginalExecutionProfile::PlayerBodyKnockbackV1
            | OriginalExecutionProfile::PlayerCommandMovementV1
            | OriginalExecutionProfile::PlayerDemonAcquisitionV1
            | OriginalExecutionProfile::PlayerDemonMeleeV1
            | OriginalExecutionProfile::PlayerDemonPistolV1
            | OriginalExecutionProfile::PlayerDemonWeaponsV1
            | OriginalExecutionProfile::PlayerDemonPickupsV1
            | OriginalExecutionProfile::PlayerDemonArmorV1
            | OriginalExecutionProfile::PlayerDemonBonusesV1
            | OriginalExecutionProfile::PlayerMonstersV1
            | OriginalExecutionProfile::PlayerMonstersImpV1
            | OriginalExecutionProfile::PlayerMonstersMapScaleV1
            | OriginalExecutionProfile::PlayerMonstersHeightsV1
            | OriginalExecutionProfile::PlayerMonstersDiagonalV1
            | OriginalExecutionProfile::PlayerMonstersRealMapV1
            | OriginalExecutionProfile::PlayerMonstersSpecialsV1
            | OriginalExecutionProfile::PlayerMonstersMoversV1
            | OriginalExecutionProfile::PlayerMonstersE1M1V1
            | OriginalExecutionProfile::PlayerMonstersBarrelsV1
            | OriginalExecutionProfile::PlayerMonstersLevelV1
            | OriginalExecutionProfile::PlayerMonstersEpisode1V1
            | OriginalExecutionProfile::PlayerMonstersEpisode1V2 => {
                demon_policy()?;
                player_policy()?;
                if profile.has_monsters() {
                    for ty in FORMER_HUMANS {
                        former_human_policy(ty)?;
                    }
                }
                if profile.has_imps() {
                    imp_policy()?;
                    fireball_policy()?;
                }
                if profile.has_baron() {
                    bruiser_policy()?;
                    bruisershot_policy()?;
                }
            }
        }
        Ok(Self {
            registry: definitions()?,
            profile,
        })
    }
    /// Validate all source/context records before accepting externally built data.
    pub fn admit(
        registry: ActorRegistry,
        profile: OriginalExecutionProfile,
    ) -> Result<Self, OriginalRegistryError> {
        Self::validate(&registry, profile)?;
        Ok(Self { registry, profile })
    }
    pub(crate) fn validate(
        registry: &ActorRegistry,
        profile: OriginalExecutionProfile,
    ) -> Result<(), OriginalRegistryError> {
        match profile {
            OriginalExecutionProfile::DemonNormalV1 => {
                demon_policy()?;
            }
            OriginalExecutionProfile::PlayerBodyLifecycleV1
            | OriginalExecutionProfile::PlayerBodyKnockbackV1
            | OriginalExecutionProfile::PlayerCommandMovementV1
            | OriginalExecutionProfile::PlayerDemonAcquisitionV1
            | OriginalExecutionProfile::PlayerDemonMeleeV1
            | OriginalExecutionProfile::PlayerDemonPistolV1
            | OriginalExecutionProfile::PlayerDemonWeaponsV1
            | OriginalExecutionProfile::PlayerDemonPickupsV1
            | OriginalExecutionProfile::PlayerDemonArmorV1
            | OriginalExecutionProfile::PlayerDemonBonusesV1
            | OriginalExecutionProfile::PlayerMonstersV1
            | OriginalExecutionProfile::PlayerMonstersImpV1
            | OriginalExecutionProfile::PlayerMonstersMapScaleV1
            | OriginalExecutionProfile::PlayerMonstersHeightsV1
            | OriginalExecutionProfile::PlayerMonstersDiagonalV1
            | OriginalExecutionProfile::PlayerMonstersRealMapV1
            | OriginalExecutionProfile::PlayerMonstersSpecialsV1
            | OriginalExecutionProfile::PlayerMonstersMoversV1
            | OriginalExecutionProfile::PlayerMonstersE1M1V1
            | OriginalExecutionProfile::PlayerMonstersBarrelsV1
            | OriginalExecutionProfile::PlayerMonstersLevelV1
            | OriginalExecutionProfile::PlayerMonstersEpisode1V1
            | OriginalExecutionProfile::PlayerMonstersEpisode1V2 => {
                demon_policy()?;
                player_policy()?;
                if profile.has_monsters() {
                    for ty in FORMER_HUMANS {
                        former_human_policy(ty)?;
                    }
                }
                if profile.has_imps() {
                    imp_policy()?;
                    fireball_policy()?;
                }
                if profile.has_baron() {
                    bruiser_policy()?;
                    bruisershot_policy()?;
                }
            }
        }
        if registry.states().len() != usize::from(OriginalState::COUNT)
            || registry.types().len() != usize::from(OriginalActorType::COUNT) + 1
            || OriginalActorType::all().any(|ty| ty.resolve(registry).is_err())
            || OriginalState::all().any(|state| state.resolve(registry).is_err())
        {
            return Err(OriginalRegistryError::Registry);
        }
        let body = registry
            .actor_type(body_slot())
            .ok_or(OriginalRegistryError::Registry)?;
        if body.name != "CONTEXT_UNBOUND_BODY" || body.properties != body_properties() {
            return Err(OriginalRegistryError::Registry);
        }
        Ok(())
    }
    /// Immutable definitions. Metadata lookup does not authorize simulation behavior.
    pub fn registry(&self) -> &ActorRegistry {
        &self.registry
    }
    /// Resolve a typed definition key without native actor/state projection.
    pub fn resolve(&self, identity: DefinitionIdentity) -> ActorTypeId {
        match identity {
            DefinitionIdentity::Original(ty) => ActorTypeId(ty.ordinal()),
            DefinitionIdentity::Context(ContextRole::UnboundBody) => body_slot(),
        }
    }
    /// Identify a registry slot, keeping original count sentinels separate from context.
    pub fn identity(&self, slot: ActorTypeId) -> Result<DefinitionIdentity, OriginalRegistryError> {
        identity(slot)
    }
    /// Check executable capability independently of metadata recognition.
    pub fn admit_definition(
        &self,
        identity: DefinitionIdentity,
    ) -> Result<ActorTypeId, OriginalRegistryError> {
        let slot = self.resolve(identity);
        require_role_for(self.profile, slot)?;
        Ok(slot)
    }
    /// Check normal state closure; raise metadata never gains execution permission.
    pub fn admit_state(
        &self,
        identity: DefinitionIdentity,
        state: OriginalState,
    ) -> Result<StateId, OriginalRegistryError> {
        let slot = self.admit_definition(identity)?;
        state_role_for(self.profile, slot, StateId(state.ordinal()))?;
        Ok(StateId(state.ordinal()))
    }
    pub(crate) fn into_registry(self) -> ActorRegistry {
        self.registry
    }
}
pub(crate) fn identity(slot: ActorTypeId) -> Result<DefinitionIdentity, OriginalRegistryError> {
    if slot == body_slot() {
        return Ok(DefinitionIdentity::Context(ContextRole::UnboundBody));
    }
    OriginalActorType::try_from(slot.0)
        .map(DefinitionIdentity::Original)
        .map_err(|_| OriginalRegistryError::Definition(slot.0))
}
#[inline]
pub(crate) fn require_role_for(
    profile: OriginalExecutionProfile,
    slot: ActorTypeId,
) -> Result<(), OriginalRegistryError> {
    if world_pickups::kind_for(profile, slot).is_some() {
        return Ok(());
    }
    if profile.has_player_body() {
        return match identity(slot)? {
            DefinitionIdentity::Original(
                OriginalActorType::MT_PLAYER
                | OriginalActorType::MT_SERGEANT
                | OriginalActorType::MT_PUFF
                | OriginalActorType::MT_BLOOD,
            ) => Ok(()),
            DefinitionIdentity::Original(ty)
                if profile.has_monsters() && FORMER_HUMANS.contains(&ty) =>
            {
                Ok(())
            }
            DefinitionIdentity::Original(
                OriginalActorType::MT_TROOP | OriginalActorType::MT_TROOPSHOT,
            ) if profile.has_imps() => Ok(()),
            // The Spectre and the rocket (M4a plan §E.4–E.5), Episode 1 policy only.
            DefinitionIdentity::Original(
                OriginalActorType::MT_SHADOWS | OriginalActorType::MT_ROCKET,
            ) if profile.has_episode1() => Ok(()),
            // The seven static decoration types of E1M1 (info.c; M3h9 note §2.5),
            // spawned by `P_SpawnMapThing` under the E1M1 policy and its barrels
            // superset.
            DefinitionIdentity::Original(ty)
                if profile.has_e1m1() && e1m1_rules::E1M1_DECORATIONS.contains(&ty) =>
            {
                Ok(())
            }
            // The candle and the firestick (M4a plan §E.6), Episode 1 policy only.
            DefinitionIdentity::Original(ty)
                if profile.has_episode1() && e1m1_rules::EPISODE1_DECORATIONS.contains(&ty) =>
            {
                Ok(())
            }
            // The exploding barrel (info.c:1888–1912; M3h9b note §2.1, decision F4),
            // spawned by `P_SpawnMapThing` under the barrels policy alone.
            DefinitionIdentity::Original(OriginalActorType::MT_BARREL) if profile.has_barrels() => {
                Ok(())
            }
            // The teleport landing and fog (info.c:2122–2146, 2174–2198), wire 24
            // only (M4b step 4); after every older arm, so older policies pay nothing.
            DefinitionIdentity::Original(
                OriginalActorType::MT_TFOG | OriginalActorType::MT_TELEPORTMAN,
            ) if profile.has_teleport() => Ok(()),
            // The Baron of Hell and its fireball (info.c:1498–1548), wire 24 only
            // (M4b step 5), likewise after every older arm.
            DefinitionIdentity::Original(
                OriginalActorType::MT_BRUISER | OriginalActorType::MT_BRUISERSHOT,
            ) if profile.has_baron() => Ok(()),
            other => Err(OriginalRegistryError::Unsupported(other)),
        };
    }
    match identity(slot)? {
        DefinitionIdentity::Original(
            OriginalActorType::MT_SERGEANT
            | OriginalActorType::MT_PUFF
            | OriginalActorType::MT_BLOOD,
        )
        | DefinitionIdentity::Context(ContextRole::UnboundBody) => Ok(()),
        other => Err(OriginalRegistryError::Unsupported(other)),
    }
}
pub(crate) fn state_role_for(
    profile: OriginalExecutionProfile,
    ty: ActorTypeId,
    state: StateId,
) -> Result<(), OriginalRegistryError> {
    require_role_for(profile, ty)?;
    let s = OriginalState::try_from(state.0).map_err(|_| OriginalRegistryError::State(state.0))?;
    let allowed = if world_pickups::kind_for(profile, ty).is_some() {
        world_pickups::state_allowed(ty, s)
    } else if e1m1_rules::is_decoration(profile, ty) {
        // A decoration holds its spawn state alone: `tics` −1 and no action
        // (p_mobj.c:441–449), never killed or gibbed (health 1000, not shootable).
        e1m1_rules::spawn_state(ty, s)
    } else if crate::barrels::is_barrel(profile, ty) {
        // `S_BAR1`/`S_BAR2` alive and the `S_BEXP` chain once killed; `S_GIBS`
        // after a crunch is admitted by the type-keyed gibs rule, not the role.
        crate::barrels::state_allowed(s)
    } else if crate::teleport::is_landing(profile, ty) {
        // `S_NULL`, its spawn state, forever: `P_SpawnMobj` assigns it directly and
        // `tics` −1 never counts down (p_mobj.c:497–501, 441–449).
        s == OriginalState::S_NULL
    } else if crate::teleport::is_fog(profile, ty) {
        // `S_TFOG` … `S_TFOG10` (info.c:266–277), no action; the chain ends in
        // `S_NULL`, which removes the fog like a puff.
        tfog_policy()?.normal.contains(s)
    } else if ty.0 == OriginalActorType::MT_PLAYER.ordinal() {
        player_policy()?.normal.contains(s)
            || (profile.has_player_weapons()
                && matches!(s, OriginalState::S_PLAY_ATK1 | OriginalState::S_PLAY_ATK2))
    } else if is_extended_monster(ty) {
        extended_policy(ty)?.normal.contains(s)
    } else if is_fireball(ty) {
        fireball_policy()?.normal.contains(s)
    } else if is_rocket(ty) {
        rocket_policy()?.normal.contains(s)
    } else if is_bruiser_shot(ty) {
        bruisershot_policy()?.normal.contains(s)
    } else if ty == owner_slot() {
        demon_policy()?.normal.contains(s)
            && !(profile == OriginalExecutionProfile::PlayerDemonAcquisitionV1
                && matches!(
                    s,
                    OriginalState::S_SARG_ATK1
                        | OriginalState::S_SARG_ATK2
                        | OriginalState::S_SARG_ATK3
                ))
    } else {
        let source = if ty == body_slot() {
            OriginalActorType::MT_PLAYER
        } else {
            OriginalActorType::try_from(ty.0)
                .map_err(|_| OriginalRegistryError::Definition(ty.0))?
        };
        // Derived once; no graph traversal or allocation on a state-entry read.
        static BODY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
        static PUFF: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
        static BLOOD: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
        let cache = match source {
            OriginalActorType::MT_PLAYER => &BODY,
            OriginalActorType::MT_PUFF => &PUFF,
            OriginalActorType::MT_BLOOD => &BLOOD,
            _ => return Err(OriginalRegistryError::Graph),
        };
        cache
            .get_or_init(|| ActorPolicy::source(source))
            .as_ref()
            .map_err(|e| *e)?
            .normal
            .contains(s)
    };
    if allowed {
        Ok(())
    } else {
        Err(OriginalRegistryError::State(state.0))
    }
}
pub(crate) fn direct(ty: ActorTypeId, state: StateId) -> bool {
    OriginalState::try_from(state.0).ok().is_some_and(|s| {
        if is_extended_monster(ty) {
            extended_policy(ty).is_ok_and(|p| p.direct.contains(s))
        } else {
            demon_policy().is_ok_and(|p| p.direct.contains(s))
        }
    })
}
pub(crate) fn death_phase(
    ty: ActorTypeId,
    state: StateId,
) -> Result<(bool, bool), OriginalRegistryError> {
    let s = OriginalState::try_from(state.0).map_err(|_| OriginalRegistryError::State(state.0))?;
    let p = if is_extended_monster(ty) {
        extended_policy(ty)?
    } else {
        demon_policy()?
    };
    Ok((p.death.contains(s), p.before_fall.contains(s)))
}
impl Simulation {
    /// Opt-in original-ID policy using shared services. The selected registry policy
    /// determines Demon-only or bounded genuine-player execution.
    pub fn with_original_registry(
        map: Map,
        limit: u32,
        registry: OriginalExecutionRegistry,
        options: ActorOptions,
        context: MapContextOptions,
    ) -> Result<Self, SimError> {
        let policy = registry.profile;
        let heights = policy.has_heights();
        let real_map = policy.has_real_map();
        // Player policies run the default options (medium skill); wire 24 alone
        // also admits hard skill (M4b decision I4), and no other option.
        let hard = ActorOptions {
            skill: Skill::Hard,
            ..ActorOptions::default()
        };
        if policy.has_player_body()
            && ((options != ActorOptions::default()
                && !(policy.has_hard_skill() && options == hard))
                || (!heights
                    && (map.sectors().len() != 1
                        || map.sectors().iter().any(|s| s.heights != [0, 128]))))
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        if policy.has_diagonals() {
            player_map_scale::validate_diagonal_geometry(&map)?;
        } else if heights {
            player_map_scale::validate_heights_geometry(&map)?;
        } else if policy.has_physical_player() {
            original_player::validate_grounded_geometry(&map)?;
        }
        // The map-scale proof needs its full slot budget (320, or 384 under the
        // barrels policy) and the walled rectangle, both decided here, before
        // construction; historical policies keep 64.
        let actor_cap = if policy.has_map_scale() {
            let slots = player_map_scale::actor_slots(policy);
            if limit != slots {
                return Err(StoreError::InvalidLimit.into());
            }
            slots
        } else {
            64
        };
        // Under the real-map policy the arithmetic certificate (M3h6 note §4.2) and
        // the slide-window rule (§4.4) are admission requirements, checked before
        // the boundary so that a failing map names its arithmetic first.
        // Under the Episode 1 policy (decision H3) clauses (i), (ii) and (vii)
        // become the checked intercept and height arithmetic of the traces
        // (`StaticWorld::checked_arithmetic`); the other clauses stay load rules.
        if real_map {
            if policy.has_episode1() {
                player_map_scale::ArithmeticCertificate::derive_episode1(&map)?;
            } else {
                player_map_scale::ArithmeticCertificate::derive(&map)?;
            }
            player_map_scale::validate_slide_window(&map)?;
        }
        // Under diagonals the boundary is the closed one-sided chain and the
        // BLOCKMAP follows the closed-cell rule; under heights only one-sided lines
        // may cover the rectangle; the map-scale policy keeps its original rule.
        // Under the Episode 1 policy the boundary is optional (decision D4): a map
        // whose one-sided lines fail C1–C3 has none, its escape witness is
        // inactive and spawns fall back to room containment.
        let boundary = if policy.has_episode1() {
            player_map_scale::Boundary::derive(&map).ok().map(Arc::new)
        } else if policy.has_diagonals() {
            Some(Arc::new(player_map_scale::Boundary::derive(&map)?))
        } else {
            None
        };
        let room = if real_map {
            Some(player_map_scale::RoomBounds::derive_real_map(&map)?)
        } else if policy.has_diagonals() {
            Some(player_map_scale::RoomBounds::derive_diagonal(&map)?)
        } else if policy.has_map_scale() {
            Some(player_map_scale::RoomBounds::derive_for(&map, heights)?)
        } else {
            None
        };
        if policy.has_player_weapons()
            && map.linedefs().len() > player_map_scale::admitted_lines(policy)
        {
            return Err(CombatError::InterceptLimit.into());
        }
        let sector_lines = real_map.then(|| player_map_scale::sector_lines(&map));
        let episode1 = policy.has_episode1().then(|| {
            if policy.has_baron() {
                crate::episode1::Episode1Bounds::from_map_v2(&map, options.skill)
            } else {
                crate::episode1::Episode1Bounds::from_map(&map)
            }
        });
        // Wire 24: room for at least one teleport's two fogs when every placed thing
        // is live (M4b step 4); the store-full witness covers everything beyond.
        if let (Some(bounds), true) = (episode1, policy.has_teleport()) {
            let landings = crate::episode1::Episode1Bounds::landings(&map, options.skill);
            if bounds
                .transient_allowance(landings, actor_cap as usize)
                .is_none_or(|n| n < 2)
            {
                return Err(StoreError::Capacity.into());
            }
        }
        let mut sim = Self::with_demon_expanded_profile(
            map,
            limit,
            registry.into_registry(),
            options,
            context,
            demon_expanded_map::ExpandedSelection {
                profile: Profile::OriginalDemon,
                actor_cap,
                type_envelope: heights,
                real_map,
                specials: policy.has_specials(),
                movers: policy.has_movers(),
                e1m1: policy.has_e1m1(),
                episode1,
                teleport: policy.has_teleport(),
            },
        )?;
        sim.boundary = boundary;
        sim.world.sector_lines = sector_lines;
        if policy.has_player_body() {
            sim.original_player = Some(original_player::PlayerRuntime {
                source: OriginalPlayer::default(),
                room,
                weapon: None,
                armor: policy
                    .has_player_armor()
                    .then_some(PlayerArmorState::default()),
                pickups: policy
                    .has_world_pickups()
                    .then_some(PlayerPickupState::default()),
                policy,
                secret_count: 0,
                use_down: true,
                reborn: false,
                powers: policy
                    .has_episode1()
                    .then_some(original_player::PlayerPowers::default()),
                episode1,
                weapon_stage: original_player::WeaponStage::Idle,
            });
        }
        Ok(sim)
    }
    /// Construct the identified original package and bounded Demon context.
    pub fn original_id_map(
        map: Map,
        limit: u32,
        options: ActorOptions,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let registry = OriginalExecutionRegistry::new(OriginalExecutionProfile::DemonNormalV1)?;
        Self::with_original_registry(map, limit, registry, options, context).map_err(Into::into)
    }
    /// Run the existing atomic THING batch for the canonical Demon context.
    /// The pickup policy also admits its declared world items in that same batch;
    /// use `spawn_map_things` directly when setting up that mixed world.
    pub fn spawn_original_actor(
        &mut self,
        ty: OriginalActorType,
    ) -> Result<MapSpawnReport, OriginalRegistryError> {
        self.require_original_profile()?;
        if ty != OriginalActorType::MT_SERGEANT {
            return Err(OriginalRegistryError::Unsupported(
                DefinitionIdentity::Original(ty),
            ));
        }
        self.spawn_map_things().map_err(Into::into)
    }
    /// Spawn the explicit non-player context body through ordinary bounded ownership.
    pub fn spawn_context_body(
        &mut self,
        position: [Fixed; 2],
        height: SpawnHeight,
    ) -> Result<ActorId, OriginalRegistryError> {
        self.require_original_profile()?;
        self.spawn_type(body_slot(), position, height)
            .map_err(Into::into)
    }
    /// Enter a source state; current role/health and damage-controlled guards still apply.
    pub fn enter_original_state(
        &mut self,
        actor: ActorId,
        state: OriginalState,
    ) -> Result<(), OriginalRegistryError> {
        self.require_original_profile()?;
        self.enter_actor_state(actor, StateId(state.ordinal()))
            .map_err(Into::into)
    }
    /// Observe actual definition/state storage, with context keys kept distinct.
    pub fn original_actor_identity(
        &self,
        actor: ActorId,
    ) -> Result<(DefinitionIdentity, Option<OriginalState>), OriginalRegistryError> {
        self.require_original_profile()?;
        let life = self
            .actor(actor)
            .and_then(Actor::life)
            .ok_or(OriginalRegistryError::ProfileOrActor)?;
        require_role_for(self.player_policy(), life.actor_type)?;
        if let Some(s) = life.state {
            // A gibbed corpse of the movers policy is in S_GIBS (p_map.c:1272).
            if !self.actor(actor).is_some_and(|a| self.gibbed(a)) {
                state_role_for(self.player_policy(), life.actor_type, s)?;
            }
        }
        Ok((
            identity(life.actor_type)?,
            life.state
                .map(|s| {
                    OriginalState::try_from(s.0).map_err(|_| OriginalRegistryError::State(s.0))
                })
                .transpose()?,
        ))
    }
    pub(crate) fn with_original_actor_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
        context: MapContextOptions,
    ) -> Result<Self, SimError> {
        let registry =
            OriginalExecutionRegistry::admit(registry, OriginalExecutionProfile::DemonNormalV1)
                .map_err(|_| MonsterError::Definition)?;
        Self::with_original_registry(map, limit, registry, options, context)
    }
    fn require_original_profile(&self) -> Result<(), OriginalRegistryError> {
        if self.profile == Profile::OriginalDemon {
            Ok(())
        } else {
            Err(OriginalRegistryError::ProfileOrActor)
        }
    }
}

fn player_policy() -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
    POLICY
        .get_or_init(|| {
            let roots = OriginalActorType::MT_PLAYER.states();
            let mut p = ActorPolicy::source(OriginalActorType::MT_PLAYER)?;
            p.normal = StateSet::graph([
                roots.spawn,
                roots.see,
                roots.pain,
                roots.death,
                roots.extreme_death,
            ])?;
            for s in OriginalState::all().filter(|s| p.normal.contains(*s)) {
                if !matches!(
                    s.descriptor().action,
                    None | Some("A_Pain" | "A_PlayerScream" | "A_XScream" | "A_Fall")
                ) {
                    return Err(OriginalRegistryError::Graph);
                }
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
