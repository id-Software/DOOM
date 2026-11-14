//! Source sprite-dispatched world contacts, using the existing player and store.
use crate::{identity::*, lifecycle::ActorAccess, player_weapons::PlayerWeaponRuntime, *};

/// Explicit pickup capability; historical no-pickup policies remain unchanged.
pub const PLAYER_DEMON_PICKUPS_CONTEXT_SCHEMA: &[u8] = b"PLAYER-DEMON-1-MEDIUM-GROUNDED-PICKUPS-v1";
/// New placed plus controlled-dropped item budget, within the existing64 slots.
pub const MAX_WORLD_PICKUPS: usize = 16;
pub(crate) const MF_COUNTITEM: u32 = 0x800000;
pub(crate) const MF_SPECIAL: u32 = 1;
pub(crate) const MF_COUNTKILL: u32 = 0x0040_0000;
pub(crate) const MF_DROPPED: u32 = 0x0002_0000;
pub(crate) const MF_AMBUSH: u32 = 0x20;
const MF_PICKUP: u32 = 0x800;
/// `MF_NOTDMATCH` (p_mobj.h): the keycards' "not in deathmatch" flag.
pub(crate) const MF_NOTDMATCH: u32 = 0x0200_0000;
const BONUS_ADD: i32 = 6; // p_inter.c BONUSADD
pub(crate) const MAX_BONUS: i32 = BONUS_ADD * MAX_WORLD_PICKUPS as i32;
/// Placed plus controlled-dropped item budget of the barrels policy (M3h9b note
/// §5.1 item 2, decision F2); every older policy keeps [`MAX_WORLD_PICKUPS`].
pub const BARRELS_MAX_PICKUPS: usize = 64;
/// The placed plus controlled-dropped item budget of `policy`:
/// [`BARRELS_MAX_PICKUPS`] under the barrels policy, [`MAX_WORLD_PICKUPS`] under
/// every other one.
pub const fn max_pickups(policy: OriginalExecutionProfile) -> usize {
    if policy.has_barrels() {
        BARRELS_MAX_PICKUPS
    } else {
        MAX_WORLD_PICKUPS
    }
}
/// The `bonuscount` bound of `policy`: `BONUSADD` (6, p_inter.c) per admitted
/// item, 6 × 64 = 384 under the barrels policy and 6 × 16 = 96 otherwise.
pub(crate) const fn max_bonus(policy: OriginalExecutionProfile) -> i32 {
    BONUS_ADD * max_pickups(policy) as i32
}
/// The placed plus dropped item budget with the per-map bounds of the Episode 1
/// policy (decision H4): `P` from its THINGS lump under that policy,
/// [`max_pickups`] under every other one.
pub(crate) fn pickup_bound(
    policy: OriginalExecutionProfile,
    bounds: Option<crate::episode1::Episode1Bounds>,
) -> usize {
    match bounds {
        Some(b) if policy.has_episode1() => b.pickups,
        _ => max_pickups(policy),
    }
}
/// The `bonuscount` bound with the per-map bounds of the Episode 1 policy:
/// `6·(P + R + keycards)` under that policy, [`max_bonus`] under every other one.
pub(crate) fn bonus_bound(
    policy: OriginalExecutionProfile,
    bounds: Option<crate::episode1::Episode1Bounds>,
) -> i32 {
    match bounds {
        Some(b) if policy.has_episode1() => i32::try_from(b.max_bonus()).unwrap_or(i32::MAX),
        _ => max_bonus(policy),
    }
}
/// The live world-item bound with the per-map bounds of the Episode 1 policy:
/// `P + R` (every dropping monster drops once) under that policy,
/// [`max_world_items`] under every other one.
pub(crate) fn world_item_bound(
    policy: OriginalExecutionProfile,
    bounds: Option<crate::episode1::Episode1Bounds>,
) -> usize {
    match bounds {
        Some(b) if policy.has_episode1() => b.world_items(),
        _ => max_world_items(policy),
    }
}

/// Executable item definitions; each remains its original canonical actor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PickupKind {
    /// MT_CLIP, DoomEd2007.
    Clip,
    /// MT_MISC17, DoomEd2048.
    BulletBox,
    /// MT_MISC22, DoomEd2008.
    Shells,
    /// MT_MISC23, DoomEd2049.
    ShellBox,
    /// MT_MISC10, DoomEd2011.
    Stimpack,
    /// MT_MISC11, DoomEd2012.
    Medikit,
    /// MT_SHOTGUN, DoomEd2001; distinct from MT_SHOTGUY.
    Shotgun,
    /// MT_CHAINGUN, DoomEd2002; distinct from MT_CHAINGUY.
    Chaingun,
    /// Original MT_MISC0, DoomEd2018, animated green armor.
    GreenArmor,
    /// Original MT_MISC1, DoomEd2019, animated blue armor.
    BlueArmor,
    /// Canonical MT_MISC2, DoomEd2014, SPR_BON1.
    HealthBonus,
    /// Canonical MT_MISC3, DoomEd2015, SPR_BON2.
    ArmorBonus,
    /// MT_MISC4, DoomEd 5, SPR_BKEY (Episode 1 policy).
    BlueCard,
    /// MT_MISC6, DoomEd 6, SPR_YKEY (Episode 1 policy).
    YellowCard,
    /// MT_MISC5, DoomEd 13, SPR_RKEY (Episode 1 policy).
    RedCard,
    /// MT_MISC12, DoomEd 2013, SPR_SOUL (Episode 1 policy).
    Soulsphere,
    /// MT_INS, DoomEd 2024, SPR_PINS: partial invisibility (Episode 1 policy).
    Invisibility,
    /// MT_MISC14, DoomEd 2025, SPR_SUIT: radiation suit (Episode 1 policy).
    RadSuit,
    /// MT_MISC15, DoomEd 2026, SPR_PMAP: computer map (Episode 1 policy).
    ComputerMap,
    /// MT_MISC16, DoomEd 2045, SPR_PVIS: light amplification (Episode 1 policy).
    LightAmp,
    /// MT_MISC24, DoomEd 8, SPR_BPAK: backpack (Episode 1 policy).
    Backpack,
    /// MT_MISC18, DoomEd 2010, SPR_ROCK: one rocket (Episode 1 policy).
    Rocket,
    /// MT_MISC19, DoomEd 2046, SPR_BROK: box of rockets (Episode 1 policy).
    RocketBox,
    /// MT_MISC26, DoomEd 2005, SPR_CSAW: chainsaw (Episode 1 policy).
    Chainsaw,
    /// MT_MISC27, DoomEd 2003, SPR_LAUN: rocket launcher (Episode 1 policy).
    RocketLauncher,
}
impl PickupKind {
    /// Original definition, never a synthetic pickup alias.
    pub const fn original_type(self) -> OriginalActorType {
        match self {
            Self::Clip => OriginalActorType::MT_CLIP,
            Self::BulletBox => OriginalActorType::MT_MISC17,
            Self::Shells => OriginalActorType::MT_MISC22,
            Self::ShellBox => OriginalActorType::MT_MISC23,
            Self::Stimpack => OriginalActorType::MT_MISC10,
            Self::Medikit => OriginalActorType::MT_MISC11,
            Self::Shotgun => OriginalActorType::MT_SHOTGUN,
            Self::Chaingun => OriginalActorType::MT_CHAINGUN,
            Self::GreenArmor => OriginalActorType::MT_MISC0,
            Self::BlueArmor => OriginalActorType::MT_MISC1,
            Self::HealthBonus => OriginalActorType::MT_MISC2,
            Self::ArmorBonus => OriginalActorType::MT_MISC3,
            Self::BlueCard => OriginalActorType::MT_MISC4,
            Self::YellowCard => OriginalActorType::MT_MISC6,
            Self::RedCard => OriginalActorType::MT_MISC5,
            Self::Soulsphere => OriginalActorType::MT_MISC12,
            Self::Invisibility => OriginalActorType::MT_INS,
            Self::RadSuit => OriginalActorType::MT_MISC14,
            Self::ComputerMap => OriginalActorType::MT_MISC15,
            Self::LightAmp => OriginalActorType::MT_MISC16,
            Self::Backpack => OriginalActorType::MT_MISC24,
            Self::Rocket => OriginalActorType::MT_MISC18,
            Self::RocketBox => OriginalActorType::MT_MISC19,
            Self::Chainsaw => OriginalActorType::MT_MISC26,
            Self::RocketLauncher => OriginalActorType::MT_MISC27,
        }
    }
    fn message(self) -> PickupMessage {
        match self {
            Self::Clip => PickupMessage::Clip,
            Self::BulletBox => PickupMessage::BulletBox,
            Self::Shells => PickupMessage::Shells,
            Self::ShellBox => PickupMessage::ShellBox,
            Self::Stimpack => PickupMessage::Stimpack,
            Self::Medikit => PickupMessage::Medikit,
            Self::Shotgun => PickupMessage::Shotgun,
            Self::Chaingun => PickupMessage::Chaingun,
            Self::GreenArmor => PickupMessage::GreenArmor,
            Self::BlueArmor => PickupMessage::BlueArmor,
            Self::HealthBonus => PickupMessage::HealthBonus,
            Self::ArmorBonus => PickupMessage::ArmorBonus,
            Self::BlueCard => PickupMessage::BlueCard,
            Self::YellowCard => PickupMessage::YellowCard,
            Self::RedCard => PickupMessage::RedCard,
            Self::Soulsphere => PickupMessage::Soulsphere,
            Self::Invisibility => PickupMessage::Invisibility,
            Self::RadSuit => PickupMessage::RadSuit,
            Self::ComputerMap => PickupMessage::ComputerMap,
            Self::LightAmp => PickupMessage::LightAmp,
            Self::Backpack => PickupMessage::Backpack,
            Self::Rocket => PickupMessage::Rocket,
            Self::RocketBox => PickupMessage::RocketBox,
            Self::Chainsaw => PickupMessage::Chainsaw,
            Self::RocketLauncher => PickupMessage::Launcher,
        }
    }
    /// One of the pickups admitted by the Episode 1 policy alone.
    pub(crate) fn is_episode1(self) -> bool {
        matches!(
            self,
            Self::BlueCard
                | Self::YellowCard
                | Self::RedCard
                | Self::Soulsphere
                | Self::Invisibility
                | Self::RadSuit
                | Self::ComputerMap
                | Self::LightAmp
                | Self::Backpack
                | Self::Rocket
                | Self::RocketBox
                | Self::Chainsaw
                | Self::RocketLauncher
        )
    }
    /// The keycard index (`card_t`: blue, yellow, red) of a keycard pickup.
    pub(crate) fn card(self) -> Option<usize> {
        match self {
            Self::BlueCard => Some(0),
            Self::YellowCard => Some(1),
            Self::RedCard => Some(2),
            _ => None,
        }
    }
    /// The Episode 1 pickups whose spawn state loops through several timed
    /// frames with no action (keycards 2 × 10 tics, soulsphere and computer map
    /// 6 × 6, invisibility 4 × 6, light amplification 2 × 6; info.c:964–1005).
    fn is_cycling(self) -> bool {
        matches!(
            self,
            Self::BlueCard
                | Self::YellowCard
                | Self::RedCard
                | Self::Soulsphere
                | Self::Invisibility
                | Self::ComputerMap
                | Self::LightAmp
        )
    }
    fn can_drop(self) -> bool {
        matches!(self, Self::Clip | Self::Shotgun | Self::Chaingun)
    }
    pub(crate) fn is_bonus(self) -> bool {
        matches!(self, Self::HealthBonus | Self::ArmorBonus)
    }
    pub(crate) fn is_animated(self) -> bool {
        self.is_armor() || self.is_bonus() || self.is_cycling()
    }
    pub(crate) fn definition_flags(self) -> u32 {
        MF_SPECIAL
            | match self {
                Self::HealthBonus
                | Self::ArmorBonus
                | Self::Soulsphere
                | Self::Invisibility
                | Self::ComputerMap
                | Self::LightAmp => MF_COUNTITEM,
                Self::BlueCard | Self::YellowCard | Self::RedCard => MF_NOTDMATCH,
                _ => 0,
            }
    }
    pub(crate) fn is_armor(self) -> bool {
        matches!(self, Self::GreenArmor | Self::BlueArmor)
    }
}
pub(crate) fn kind(ty: ActorTypeId) -> Option<PickupKind> {
    Some(match OriginalActorType::try_from(ty.0).ok()? {
        OriginalActorType::MT_CLIP => PickupKind::Clip,
        OriginalActorType::MT_MISC17 => PickupKind::BulletBox,
        OriginalActorType::MT_MISC22 => PickupKind::Shells,
        OriginalActorType::MT_MISC23 => PickupKind::ShellBox,
        OriginalActorType::MT_MISC10 => PickupKind::Stimpack,
        OriginalActorType::MT_MISC11 => PickupKind::Medikit,
        OriginalActorType::MT_SHOTGUN => PickupKind::Shotgun,
        OriginalActorType::MT_CHAINGUN => PickupKind::Chaingun,
        OriginalActorType::MT_MISC0 => PickupKind::GreenArmor,
        OriginalActorType::MT_MISC1 => PickupKind::BlueArmor,
        OriginalActorType::MT_MISC2 => PickupKind::HealthBonus,
        OriginalActorType::MT_MISC3 => PickupKind::ArmorBonus,
        OriginalActorType::MT_MISC4 => PickupKind::BlueCard,
        OriginalActorType::MT_MISC6 => PickupKind::YellowCard,
        OriginalActorType::MT_MISC5 => PickupKind::RedCard,
        OriginalActorType::MT_MISC12 => PickupKind::Soulsphere,
        OriginalActorType::MT_INS => PickupKind::Invisibility,
        OriginalActorType::MT_MISC14 => PickupKind::RadSuit,
        OriginalActorType::MT_MISC15 => PickupKind::ComputerMap,
        OriginalActorType::MT_MISC16 => PickupKind::LightAmp,
        OriginalActorType::MT_MISC24 => PickupKind::Backpack,
        OriginalActorType::MT_MISC18 => PickupKind::Rocket,
        OriginalActorType::MT_MISC19 => PickupKind::RocketBox,
        OriginalActorType::MT_MISC26 => PickupKind::Chainsaw,
        OriginalActorType::MT_MISC27 => PickupKind::RocketLauncher,
        _ => return None,
    })
}
pub(crate) fn kind_for(profile: OriginalExecutionProfile, ty: ActorTypeId) -> Option<PickupKind> {
    // Historical non-pickup execution must not walk item identity metadata.
    if !profile.has_world_pickups() {
        return None;
    }
    let k = kind(ty)?;
    (!k.is_armor() || profile.has_player_armor())
        .then_some(k)
        .filter(|k| !k.is_bonus() || profile.has_player_bonuses())
        .filter(|k| !k.is_episode1() || profile.has_episode1())
}
/// The spawn loop of a cycling Episode 1 pickup: its spawn state and each `next`
/// until the loop closes (at most eight states, the longest is six).
fn spawn_loop(root: OriginalState) -> impl Iterator<Item = OriginalState> {
    let mut state = Some(root);
    let mut steps = 0;
    std::iter::from_fn(move || {
        let current = state?;
        steps += 1;
        let next = current.descriptor().next;
        state = (next != root && steps < 8).then_some(next);
        Some(current)
    })
}
pub(crate) fn state_allowed(ty: ActorTypeId, state: OriginalState) -> bool {
    kind(ty).is_some_and(|k| {
        let root = k.original_type().states().spawn;
        if k.is_bonus() {
            state
                .ordinal()
                .checked_sub(root.ordinal())
                .is_some_and(|offset| offset < 6)
        } else if k.is_cycling() {
            spawn_loop(root).any(|s| s == state)
        } else {
            state == root || (k.is_armor() && state == root.descriptor().next)
        }
    })
}
/// Stable identifiers for actual dstrings.h messages; no HUD timeout is implied.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum PickupMessage {
    /// No source message assigned.
    #[default]
    None = 0,
    /// GOTCLIP.
    Clip = 1,
    /// GOTCLIPBOX.
    BulletBox = 2,
    /// GOTSHELLS.
    Shells = 3,
    /// GOTSHELLBOX.
    ShellBox = 4,
    /// GOTSTIM.
    Stimpack = 5,
    /// GOTMEDIKIT.
    Medikit = 6,
    /// GOTMEDINEED, source metadata only in this positive-health domain.
    MedikitNeeded = 7,
    /// GOTSHOTGUN.
    Shotgun = 8,
    /// GOTCHAINGUN.
    Chaingun = 9,
    /// GOTARMOR, ordinary green armor.
    GreenArmor = 10,
    /// GOTMEGA, blue armor; not a MegaSphere.
    BlueArmor = 11,
    /// GOTHTHBONUS.
    HealthBonus = 12,
    /// GOTARMBONUS.
    ArmorBonus = 13,
    /// GOTSUPER (soulsphere; Episode 1 policy, DSIM39).
    Soulsphere = 14,
    /// GOTBLUECARD.
    BlueCard = 15,
    /// GOTYELWCARD.
    YellowCard = 16,
    /// GOTREDCARD.
    RedCard = 17,
    /// GOTINVIS.
    Invisibility = 18,
    /// GOTSUIT.
    RadSuit = 19,
    /// GOTMAP.
    ComputerMap = 20,
    /// GOTVISOR.
    LightAmp = 21,
    /// GOTROCKET.
    Rocket = 22,
    /// GOTROCKBOX.
    RocketBox = 23,
    /// GOTBACKPACK.
    Backpack = 24,
    /// GOTCHAINSAW (the chainsaw pickup arrives with the M4a weapons step).
    Chainsaw = 25,
    /// GOTLAUNCHER (likewise).
    Launcher = 26,
    /// PD_BLUEK: a blue-locked door used without the key (`EV_VerticalDoor`,
    /// p_doors.c:378; Episode 1 policy, decision D7).
    BlueKeyNeeded = 27,
    /// PD_YELLOWK (p_doors.c:392).
    YellowKeyNeeded = 28,
    /// PD_REDK (p_doors.c:405).
    RedKeyNeeded = 29,
    /// `"%s is turbo!"` with `player_names[0]` (`HUSTR_PLRGREEN`): `G_Ticker`
    /// assigns it to the console player's message when the command's
    /// `forwardmove > TURBOTHRESHOLD` (50) on a gametic with `!(gametic & 31)`
    /// and `((gametic >> 5) & 3) == 0` (g_game.c:671–679), before `P_Ticker`;
    /// Episode 1 policy only (wire 23, M4a step 8).
    Turbo = 30,
}
/// Source pickup-only fields; health, body and inventory retain their owners.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlayerPickupState {
    /// Source bonuscount; live admitted fragment decrements before thinkers.
    pub bonus_count: i32,
    /// Source itemcount; the admitted definitions lack MF_COUNTITEM.
    pub item_count: i32,
    /// Last assigned source message, without presentation-side clearing.
    pub message: PickupMessage,
}
impl PlayerPickupState {
    fn validate(
        self,
        profile: OriginalExecutionProfile,
        bounds: Option<crate::episode1::Episode1Bounds>,
    ) -> Result<(), SimError> {
        if !(0..=bonus_bound(profile, bounds)).contains(&self.bonus_count)
            || if profile.has_player_bonuses() {
                !(0..=pickup_bound(profile, bounds) as i32).contains(&self.item_count)
            } else {
                self.item_count != 0
            }
            || self.message == PickupMessage::MedikitNeeded
            || (bounds.is_none() && self.message as u8 >= PickupMessage::Soulsphere as u8)
            || (!profile.has_player_bonuses()
                && matches!(
                    self.message,
                    PickupMessage::HealthBonus | PickupMessage::ArmorBonus
                ))
            || (!profile.has_player_armor()
                && matches!(
                    self.message,
                    PickupMessage::GreenArmor | PickupMessage::BlueArmor
                ))
        {
            return Err(PlayerBodyError::Invariant.into());
        }
        Ok(())
    }
}
/// Source touch return classification, independent of handle lifetime policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PickupOutcome {
    /// Vertical reach rejected.
    OutOfReach = 0,
    /// Toucher health is nonpositive.
    Dead = 1,
    /// The resource was already full/owned with no ammo benefit.
    NotNeeded = 2,
    /// Resource granted and item marked for deferred retirement.
    Collected = 3,
}
/// Ordered source observations; DSIM25 event23 subtypes0–6, DSIM26 also7.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PickupEvent {
    /// Item/body/sprite, vertical delta, body height and signed health.
    Touch(ActorId, ActorId, u16, Fixed, Fixed, i32),
    /// Ammo kind, accepted, before/after, current ready and resulting pending.
    Ammo(u8, bool, i32, i32, Weapon, Option<Weapon>),
    /// Accepted and player before/after plus resulting mobj health.
    Body(bool, i32, i32, i32),
    /// Weapon, accepted, ownership before/after, pending before/after.
    Weapon(Weapon, bool, u16, u16, Option<Weapon>, Option<Weapon>),
    /// Assigned actual source message.
    Message(PickupMessage),
    /// Resulting source bonuscount and itemcount.
    Bonus(i32, i32),
    /// Actual source touch return classification.
    Result(ActorId, PickupOutcome),
    /// Requested source class, accepted, armor before and after P_GiveArmor.
    Armor(ArmorType, bool, PlayerArmorState, PlayerArmorState),
    /// Direct BON1 effect, without entering P_GiveBody.
    HealthBonus(i32, i32, i32),
    /// Direct BON2 effect; ordinary armor grant thresholds remain separate.
    ArmorBonus(PlayerArmorState, PlayerArmorState),
    /// Original COUNTITEM increment, after message and before source removal.
    Counted(i32, i32),
    /// `P_GiveCard` (Episode 1 policy): the `card_t` index and whether the card
    /// was already held (then nothing changes and no message is assigned).
    Card(u8, bool),
    /// `P_GivePower` (Episode 1 policy): the `powertype_t` index, accepted, and
    /// the counter before and after.
    Power(u8, bool, i32, i32),
    /// `SPR_SOUL` (Episode 1 policy): player health before and after, and the
    /// resulting mobj health.
    Soulsphere(i32, i32, i32),
    /// `SPR_BPAK` (Episode 1 policy): whether this backpack was the first (the
    /// maxima doubled); the four `P_GiveAmmo` observations follow.
    Backpack(bool),
}
pub(crate) enum ContactEvent {
    Pickup(PickupEvent),
    Removed(ActorId),
    Sound(u16),
}
impl ContactEvent {
    pub(crate) fn event(self) -> Event {
        match self {
            Self::Pickup(e) => Event::Pickup(e),
            Self::Removed(id) => Event::RemovalMarked(id),
            Self::Sound(sound) => Event::ActorState(ActorStateEvent::Sound { actor: None, sound }),
        }
    }
}
/// Pure function of the type ordinal and the compile-time identity tables, so its
/// result is computed once per process and reused by every later preflight.
pub(crate) fn validate_definition(ty: ActorTypeId) -> Result<(), SimError> {
    static MEMO: [std::sync::OnceLock<Result<(), SimError>>; 256] =
        [const { std::sync::OnceLock::new() }; 256];
    match MEMO.get(usize::from(ty.0)) {
        Some(cell) => *cell.get_or_init(|| validate_definition_uncached(ty)),
        None => validate_definition_uncached(ty),
    }
}
fn validate_definition_uncached(ty: ActorTypeId) -> Result<(), SimError> {
    let k = kind(ty).ok_or(ActorError::InvalidType)?;
    let d = k.original_type().descriptor();
    let s = d.states.spawn.descriptor();
    let timing = if k.is_bonus() {
        let mut state = d.states.spawn;
        let mut valid = true;
        for frame in [0, 1, 2, 3, 2, 1] {
            let descriptor = state.descriptor();
            valid &= descriptor.tics == 6
                && descriptor.sprite == s.sprite
                && descriptor.frame == frame
                && descriptor.action.is_none();
            state = descriptor.next;
        }
        valid && state == d.states.spawn
    } else if k.is_cycling() {
        // Closed within eight states, every state timed, with the spawn sprite
        // and no action (the keycard, soulsphere, invisibility, computer-map and
        // light-amplification loops, info.c:964–1005).
        let states: Vec<_> = spawn_loop(d.states.spawn).collect();
        states.len() < 8
            && states
                .last()
                .is_some_and(|l| l.descriptor().next == d.states.spawn)
            && states.iter().all(|state| {
                let descriptor = state.descriptor();
                descriptor.tics > 0 && descriptor.sprite == s.sprite && descriptor.action.is_none()
            })
    } else if k.is_armor() {
        let next = s.next.descriptor();
        s.tics > 0
            && next.tics > 0
            && next.next == d.states.spawn
            && next.sprite == s.sprite
            && next.action.is_none()
    } else {
        s.tics == -1 && s.next == OriginalState::S_NULL
    };
    if d.flags != k.definition_flags() || !timing || s.action.is_some() {
        return Err(ActorError::InvalidType.into());
    }
    Ok(())
}
/// The height rule of the item witness.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ItemHeights<'a> {
    /// The flat world's `0/128`.
    Flat,
    /// The heights policies: the item sits on its own center sector's floor under
    /// that sector's ceiling (static sectors).
    Sector(crate::staging::Sectors<'a>),
    /// The movers policy (M3h8 note §4.2): sector heights move and the item keeps
    /// its own clipped `floorz`/`ceilingz`, so only `z == floorz` is required with
    /// those fields; the item either fits (`ceilingz − floorz ≥ height`) or is an
    /// undropped item left under a closed door (V′ case (b), p_map.c:1290–1294; a
    /// dropped one is removed, 1282–1288). Both alternatives are `z == floorz`, and
    /// the tic-boundary V′ check (`Simulation::vertical_invariant`) classifies them.
    Own,
}
impl<'a> ItemHeights<'a> {
    pub(crate) fn for_policy(
        policy: OriginalExecutionProfile,
        sectors: crate::staging::Sectors<'a>,
    ) -> Self {
        if policy.has_movers() {
            Self::Own
        } else if policy.has_heights() {
            Self::Sector(sectors)
        } else {
            Self::Flat
        }
    }
}
pub(crate) fn validate_item(
    room: Option<crate::player_map_scale::RoomBounds>,
    heights: ItemHeights<'_>,
    a: Actor,
) -> Result<(), SimError> {
    let l = a.life().ok_or(ActorError::InvalidType)?;
    let k = kind(l.actor_type).ok_or(ActorError::InvalidType)?;
    validate_definition(l.actor_type)?;
    let [floor, ceiling] = match heights {
        ItemHeights::Sector(sectors) => {
            let location = a.location.ok_or(PlayerBodyError::Invariant)?;
            let sector = sectors
                .get(usize::from(location.sector))
                .ok_or(PlayerBodyError::Invariant)?;
            [sector.floor, sector.ceiling]
        }
        ItemHeights::Own => [a.floor, a.ceiling],
        ItemHeights::Flat => [Fixed::ZERO, Fixed::from_map_units(128)],
    };
    let state = l
        .state
        .and_then(|v| OriginalState::try_from(v.0).ok())
        .ok_or(ActorError::InvalidState)?;
    let s = state.descriptor();
    let dropped = l.flags & MF_DROPPED != 0;
    if !state_allowed(l.actor_type, state)
        || l.sprite != s.sprite.ordinal()
        || l.frame != s.frame
        || (if k.is_animated() {
            !(1..=s.tics).contains(&l.tics)
        } else {
            l.tics != -1
        })
        || l.health != 1000
        || l.target.is_some()
        || l.reaction_time != 8
        || l.threshold != 0
        || l.movedir != 0
        || l.movecount != 0
        || l.death_requested
        || a.lastlook() > 3
        || a.radius() != Fixed::from_map_units(20)
        || a.height() != Fixed::from_map_units(16)
        || a.momentum() != [Fixed::ZERO; 2]
        || a.momz != Fixed::ZERO
        || a.z != a.floor
        || a.floor != floor
        || a.ceiling != ceiling
        || !crate::player_map_scale::position_admitted(room, a.position(), 978)
        || l.flags & !(k.definition_flags() | MF_AMBUSH | MF_DROPPED) != 0
        || l.flags & k.definition_flags() != k.definition_flags()
        || (dropped && (!k.can_drop() || l.spawnpoint.is_some() || l.flags & MF_AMBUSH != 0))
        || (!dropped
            && l.spawnpoint.is_none_or(|p| {
                i32::from(p.kind) != k.original_type().descriptor().doomed_number
                    || a.position()
                        != [
                            Fixed::from_map_units(p.position.x),
                            Fixed::from_map_units(p.position.y),
                        ]
            }))
    {
        return Err(PlayerBodyError::Invariant.into());
    }
    Ok(())
}
/// The counter countdowns at the end of `P_PlayerThink`'s live branch
/// (p_user.c:355–359), after `P_MovePsprites` (334) and the power countdowns
/// (339–353, not modelled): `damagecount` (355–356, the level policy only), then
/// `bonuscount` (358–359, every pickup policy). `think_live` is whether the live
/// branch ran this tic; the level policy reads it, as the source does, so a
/// player killed by its own `P_PlayerInSpecialSector` (274–275) still counts
/// down; every older policy keeps reading the state after the player stage.
pub(crate) fn bonus_tick(
    player: Option<&mut original_player::PlayerRuntime>,
    think_live: bool,
) -> Result<(), SimError> {
    if let Some(p) = player {
        let level = p.policy.has_level();
        if level && think_live && p.damage_count > 0 {
            p.damage_count -= 1;
        }
        if p.policy.has_world_pickups() {
            let live = if level {
                think_live
            } else {
                p.state == PlayerBodyState::Live
            };
            let state = p.pickups.as_mut().ok_or(PlayerBodyError::Invariant)?;
            state.validate(p.policy, p.episode1)?;
            if live && state.bonus_count > 0 {
                state.bonus_count -= 1;
            }
        }
    }
    Ok(())
}
fn observe(events: &mut Vec<movement::MotionEvent>, event: PickupEvent) {
    events.push(movement::MotionEvent::Contact(ContactEvent::Pickup(event)));
}
/// `powertype_t` indices (doomdef.h:214–222) and the `P_GivePower` durations
/// (doomdef.h:226–229: `INVISTICS`, `IRONTICS` 60·TICRATE, `INFRATICS` 120·TICRATE).
pub(crate) const POWER_INVULNERABILITY: usize = 0;
pub(crate) const POWER_STRENGTH: usize = 1;
pub(crate) const POWER_INVISIBILITY: usize = 2;
pub(crate) const POWER_IRONFEET: usize = 3;
pub(crate) const POWER_ALLMAP: usize = 4;
pub(crate) const POWER_INFRARED: usize = 5;
pub(crate) const INVISTICS: i32 = 60 * 35;
pub(crate) const IRONTICS: i32 = 60 * 35;
pub(crate) const INFRATICS: i32 = 120 * 35;
/// `clipammo` (p_inter.c:45): the rounds of one clip load per `ammotype_t`.
const CLIP_AMMO: [u16; 4] = [10, 4, 20, 1];
/// `P_GiveAmmo` (p_inter.c:73–160) for `ammo` in `ammotype_t` order (clip,
/// shell, cell, misl): refused at `maxammo`; `loads` clip loads, or half a clip
/// for 0; no doubling at the medium skill; capped at `maxammo` (doubled once by
/// the backpack); and from zero the weapon preference switch. Only the Episode 1
/// policy reaches cells and rockets.
fn ammo(
    p: &mut PlayerWeaponState,
    ammo: usize,
    loads: u16,
    events: &mut Vec<movement::MotionEvent>,
) -> Result<bool, SimError> {
    let counts = [p.bullets, p.shells, p.cells, p.rockets];
    let before = *counts.get(ammo).ok_or(WeaponError::Inventory)?;
    let cap = u16::try_from(*p.max_ammo().get(ammo).ok_or(WeaponError::Inventory)?)
        .map_err(|_| WeaponError::Inventory)?;
    if before > cap {
        return Err(WeaponError::Inventory.into());
    }
    let accepted = before != cap;
    let mut after = before;
    if accepted {
        let clip = CLIP_AMMO[ammo];
        let count = if loads == 0 {
            clip / 2
        } else {
            loads.checked_mul(clip).ok_or(WeaponError::Inventory)?
        };
        // Private callers supply at most five loads: before<=600 and count<=100,
        // so the checked source addition is representable before capping.
        after = before
            .checked_add(count)
            .ok_or(WeaponError::Inventory)?
            .min(cap);
        match ammo {
            0 => p.bullets = after,
            1 => p.shells = after,
            2 => p.cells = after,
            _ => p.rockets = after,
        }
        if before == 0 {
            let owns = |w: Weapon| p.owned & (1 << w as u8) != 0;
            match ammo {
                0 if p.ready == Weapon::Fist => {
                    p.pending = Some(if owns(Weapon::Chaingun) {
                        Weapon::Chaingun
                    } else {
                        Weapon::Pistol
                    });
                }
                1 if matches!(p.ready, Weapon::Fist | Weapon::Pistol) && owns(Weapon::Shotgun) => {
                    p.pending = Some(Weapon::Shotgun);
                }
                2 if matches!(p.ready, Weapon::Fist | Weapon::Pistol) && owns(Weapon::Plasma) => {
                    p.pending = Some(Weapon::Plasma);
                }
                3 if p.ready == Weapon::Fist && owns(Weapon::Rocket) => {
                    p.pending = Some(Weapon::Rocket);
                }
                _ => {}
            }
        }
    }
    observe(
        events,
        PickupEvent::Ammo(
            u8::try_from(ammo).map_err(|_| WeaponError::Inventory)?,
            accepted,
            i32::from(before),
            i32::from(after),
            p.ready,
            p.pending,
        ),
    );
    Ok(accepted)
}
/// Same actor access and player state used by the shared thinker transaction.
pub(crate) struct Contacts<'a, A> {
    pub access: &'a mut A,
    pub player: &'a mut original_player::PlayerRuntime,
    /// Static sectors, for the item validation of the heights policy.
    pub sectors: crate::staging::Sectors<'a>,
    /// The one-sided boundary of the diagonal policy, for the escape witness.
    pub boundary: Option<&'a crate::player_map_scale::Boundary>,
}
impl<A: ActorAccess> movement::MotionAccess for Contacts<'_, A> {
    type Error = SimError;
    fn get(&self, id: ActorId) -> Option<Actor> {
        self.access.get(id).copied()
    }
    /// The escape witness of the diagonal policy: a projected slide move whose
    /// accepted destination has even parity left the room (the corner jump); the
    /// tic fails and the transaction is dropped. Exact and never a false positive
    /// (M3h5 note §4.5); Theorem B makes it unreachable below 30 units per axis.
    fn after_slide(&mut self, body: &Actor) -> Result<(), SimError> {
        match self.boundary {
            Some(boundary) if !boundary.inside(body.position()) => {
                Err(crate::MapScaleError::Escape.into())
            }
            _ => Ok(()),
        }
    }
    /// The intercept witness of the real-map policy (M3h6 note §4.4): a slide
    /// trace that would write the source's 129th intercept fails the tic.
    fn intercept_limit(&self) -> Option<usize> {
        self.player
            .policy
            .has_real_map()
            .then_some(crate::player_map_scale::MAX_INTERCEPTS)
    }
    fn intercept_overflow(&self) -> SimError {
        crate::CombatError::InterceptLimit.into()
    }
    fn intercept_arithmetic(&self) -> SimError {
        crate::MapScaleError::Arithmetic.into()
    }
    fn contacts(
        &mut self,
        id: ActorId,
        body: &mut Actor,
        attempt: &ActorMoveAttempt,
        spatial: &mut movement::SpatialIndex,
        events: &mut Vec<movement::MotionEvent>,
    ) -> Result<(), SimError> {
        if self.player.mo != Some(id) || !self.player.policy.has_world_pickups() {
            return Err(PlayerBodyError::Binding.into());
        }
        for &item_id in &attempt.actors {
            if item_id == id {
                continue;
            }
            let item = self.get(item_id).ok_or(StoreError::InvalidHandle)?;
            let l = item.life().ok_or(ActorError::InvalidType)?;
            if l.flags & MF_SPECIAL == 0 {
                continue;
            }
            let distance = i64::from(body.radius().bits()) + i64::from(item.radius().bits());
            if (0..2).any(|axis| {
                (i64::from(item.position()[axis].bits()) - i64::from(attempt.target[axis].bits()))
                    .abs()
                    >= distance
            }) {
                continue;
            }
            if body.life().is_none_or(|v| v.flags & MF_PICKUP == 0) {
                continue;
            }
            validate_item(
                self.player.room,
                ItemHeights::for_policy(self.player.policy, self.sectors),
                item,
            )?;
            let k = kind_for(self.player.policy, l.actor_type).ok_or(ActorError::InvalidType)?;
            let delta = item.z - body.z;
            let health = body.life().ok_or(ActorError::InvalidType)?.health;
            observe(
                events,
                PickupEvent::Touch(item_id, id, l.sprite, delta, body.height(), health),
            );
            let mut outcome = if delta > body.height() || delta < Fixed::from_map_units(-8) {
                PickupOutcome::OutOfReach
            } else if health <= 0 {
                PickupOutcome::Dead
            } else {
                PickupOutcome::NotNeeded
            };
            if outcome == PickupOutcome::NotNeeded {
                let weapon = self.player.weapon.as_mut().ok_or(WeaponError::Player)?;
                let PlayerWeaponRuntime::Pack(w) = weapon else {
                    return Err(WeaponError::Inventory.into());
                };
                // The assigned message; a keycard already held assigns none
                // (p_inter.c:421–443).
                let mut message = Some(k.message());
                let mut sound = if matches!(
                    k,
                    PickupKind::Shotgun
                        | PickupKind::Chaingun
                        | PickupKind::Chainsaw
                        | PickupKind::RocketLauncher
                ) {
                    "sfx_wpnup"
                } else {
                    "sfx_itemup"
                };
                let accepted = match k {
                    PickupKind::HealthBonus => {
                        let before = self.player.health;
                        let after = before
                            .checked_add(1)
                            .ok_or(PlayerBodyError::Invariant)?
                            .min(200);
                        self.player.health = after;
                        body.life.as_mut().ok_or(ActorError::InvalidType)?.health = after;
                        self.access
                            .get_mut(id)
                            .and_then(|a| a.life.as_mut())
                            .ok_or(StoreError::InvalidHandle)?
                            .health = after;
                        observe(events, PickupEvent::HealthBonus(before, after, after));
                        true
                    }
                    PickupKind::ArmorBonus => {
                        let policy = self.player.policy;
                        let armor = self
                            .player
                            .armor
                            .as_mut()
                            .ok_or(PlayerBodyError::Invariant)?;
                        armor.validate_for(policy)?;
                        let before = *armor;
                        armor.points = armor
                            .points
                            .checked_add(1)
                            .ok_or(PlayerBodyError::Invariant)?
                            .min(200);
                        if armor.kind == ArmorType::None {
                            armor.kind = ArmorType::Green;
                        }
                        armor.validate_for(policy)?;
                        observe(events, PickupEvent::ArmorBonus(before, *armor));
                        true
                    }
                    PickupKind::GreenArmor | PickupKind::BlueArmor => {
                        let requested = if k == PickupKind::GreenArmor {
                            ArmorType::Green
                        } else {
                            ArmorType::Blue
                        };
                        let armor = self
                            .player
                            .armor
                            .as_mut()
                            .ok_or(PlayerBodyError::Invariant)?;
                        let before = *armor;
                        let accepted = armor.grant_for(requested, self.player.policy)?;
                        observe(
                            events,
                            PickupEvent::Armor(requested, accepted, before, *armor),
                        );
                        accepted
                    }
                    PickupKind::Clip => {
                        ammo(w, 0, if l.flags & MF_DROPPED != 0 { 0 } else { 1 }, events)?
                    }
                    PickupKind::BulletBox => ammo(w, 0, 5, events)?,
                    PickupKind::Shells => ammo(w, 1, 1, events)?,
                    PickupKind::ShellBox => ammo(w, 1, 5, events)?,
                    PickupKind::Stimpack | PickupKind::Medikit => {
                        let before = self.player.health;
                        let accepted = before < 100;
                        if accepted {
                            let after = before
                                .checked_add(if k == PickupKind::Stimpack { 10 } else { 25 })
                                .ok_or(PlayerBodyError::Invariant)?
                                .min(100);
                            self.player.health = after;
                            body.life.as_mut().ok_or(ActorError::InvalidType)?.health = after;
                            self.access
                                .get_mut(id)
                                .and_then(|a| a.life.as_mut())
                                .ok_or(StoreError::InvalidHandle)?
                                .health = after;
                        }
                        observe(
                            events,
                            PickupEvent::Body(
                                accepted,
                                before,
                                self.player.health,
                                body.life().ok_or(ActorError::InvalidType)?.health,
                            ),
                        );
                        accepted
                    }
                    PickupKind::BlueCard | PickupKind::YellowCard | PickupKind::RedCard => {
                        // `if (!cards[c]) message = …; P_GiveCard`: a held card
                        // changes nothing; a new one assigns `bonuscount = BONUSADD`
                        // (p_inter.c:274–283). Single player then breaks to the tail.
                        let card = k.card().ok_or(ActorError::InvalidType)?;
                        let powers = self
                            .player
                            .powers
                            .as_mut()
                            .ok_or(PlayerBodyError::Profile)?;
                        let held = powers.cards[card];
                        if held {
                            message = None;
                        } else {
                            powers.cards[card] = true;
                            self.player
                                .pickups
                                .as_mut()
                                .ok_or(PlayerBodyError::Invariant)?
                                .bonus_count = BONUS_ADD;
                        }
                        observe(
                            events,
                            PickupEvent::Card(u8::try_from(card).unwrap_or(u8::MAX), held),
                        );
                        true
                    }
                    PickupKind::Soulsphere => {
                        // p_inter.c:400–407: +100, capped at 200, always taken.
                        let before = self.player.health;
                        let after = before
                            .checked_add(100)
                            .ok_or(PlayerBodyError::Invariant)?
                            .min(200);
                        self.player.health = after;
                        body.life.as_mut().ok_or(ActorError::InvalidType)?.health = after;
                        self.access
                            .get_mut(id)
                            .and_then(|a| a.life.as_mut())
                            .ok_or(StoreError::InvalidHandle)?
                            .health = after;
                        observe(events, PickupEvent::Soulsphere(before, after, after));
                        sound = "sfx_getpow";
                        true
                    }
                    PickupKind::Invisibility
                    | PickupKind::RadSuit
                    | PickupKind::ComputerMap
                    | PickupKind::LightAmp => {
                        // P_GivePower (p_inter.c:290–330): invisibility 2100 tics
                        // and MF_SHADOW on the body, suit 2100, visor 4200; the map
                        // is refused when already held, else set to 1.
                        let (power, tics) = match k {
                            PickupKind::Invisibility => (POWER_INVISIBILITY, INVISTICS),
                            PickupKind::RadSuit => (POWER_IRONFEET, IRONTICS),
                            PickupKind::ComputerMap => (POWER_ALLMAP, 1),
                            _ => (POWER_INFRARED, INFRATICS),
                        };
                        let powers = self
                            .player
                            .powers
                            .as_mut()
                            .ok_or(PlayerBodyError::Profile)?;
                        let before = powers.powers[power];
                        let accepted = power != POWER_ALLMAP || before == 0;
                        if accepted {
                            powers.powers[power] = tics;
                        }
                        if power == POWER_INVISIBILITY {
                            body.life.as_mut().ok_or(ActorError::InvalidType)?.flags |=
                                original_player::MF_SHADOW;
                            self.access
                                .get_mut(id)
                                .and_then(|a| a.life.as_mut())
                                .ok_or(StoreError::InvalidHandle)?
                                .flags |= original_player::MF_SHADOW;
                        }
                        observe(
                            events,
                            PickupEvent::Power(
                                u8::try_from(power).unwrap_or(u8::MAX),
                                accepted,
                                before,
                                powers.powers[power],
                            ),
                        );
                        sound = "sfx_getpow";
                        accepted
                    }
                    PickupKind::Rocket => ammo(w, 3, 1, events)?,
                    PickupKind::RocketBox => ammo(w, 3, 5, events)?,
                    PickupKind::Backpack => {
                        // p_inter.c:589–599: the first backpack doubles every
                        // maximum; then one clip load of each type, in order,
                        // each with its own zero-ammo switch; always taken.
                        let first = !w.backpack;
                        w.backpack = true;
                        observe(events, PickupEvent::Backpack(first));
                        for ty in 0..4 {
                            ammo(w, ty, 1, events)?;
                        }
                        true
                    }
                    PickupKind::Shotgun
                    | PickupKind::Chaingun
                    | PickupKind::Chainsaw
                    | PickupKind::RocketLauncher => {
                        // P_GiveWeapon (p_inter.c:168–222), single player: two
                        // clip loads of a found weapon (one if MF_DROPPED; the
                        // chainsaw and launcher pass `dropped` false, 617, 624),
                        // none for the chainsaw (am_noammo); then ownership and
                        // the pending switch.
                        let weapon = match k {
                            PickupKind::Shotgun => Weapon::Shotgun,
                            PickupKind::Chaingun => Weapon::Chaingun,
                            PickupKind::Chainsaw => Weapon::Chainsaw,
                            _ => Weapon::Rocket,
                        };
                        let before = w.owned;
                        let pending = w.pending;
                        let dropped = l.flags & MF_DROPPED != 0
                            && matches!(k, PickupKind::Shotgun | PickupKind::Chaingun);
                        let gave = match weapon.ammo() {
                            Some(kind) => ammo(w, kind, if dropped { 1 } else { 2 }, events)?,
                            None => false,
                        };
                        let fresh = before & (1 << weapon as u8) == 0;
                        if fresh {
                            w.owned |= 1 << weapon as u8;
                            w.pending = Some(weapon);
                        }
                        observe(
                            events,
                            PickupEvent::Weapon(
                                weapon,
                                gave || fresh,
                                before,
                                w.owned,
                                pending,
                                w.pending,
                            ),
                        );
                        gave || fresh
                    }
                };
                if accepted {
                    let state = self
                        .player
                        .pickups
                        .as_mut()
                        .ok_or(PlayerBodyError::Invariant)?;
                    if let Some(message) = message {
                        state.message = message;
                        observe(events, PickupEvent::Message(message));
                    }
                    if l.flags & MF_COUNTITEM != 0 {
                        if !self.player.policy.has_player_bonuses() {
                            return Err(PlayerBodyError::Invariant.into());
                        }
                        let before = state.item_count;
                        state.item_count =
                            before.checked_add(1).ok_or(PlayerBodyError::Invariant)?;
                        observe(events, PickupEvent::Counted(before, state.item_count));
                    }
                    if !self.access.mark(item_id)? {
                        return Err(StoreError::InvalidHandle.into());
                    }
                    spatial.unlink(item_id);
                    events.push(movement::MotionEvent::Contact(ContactEvent::Removed(
                        item_id,
                    )));
                    state.bonus_count = state
                        .bonus_count
                        .checked_add(BONUS_ADD)
                        .ok_or(PlayerBodyError::Invariant)?;
                    state.validate(self.player.policy, self.player.episode1)?;
                    observe(
                        events,
                        PickupEvent::Bonus(state.bonus_count, state.item_count),
                    );
                    events.push(movement::MotionEvent::Contact(ContactEvent::Sound(
                        lifecycle::source_sound(sound).ok_or(ActorError::InvalidType)?,
                    )));
                    outcome = PickupOutcome::Collected;
                }
            }
            observe(events, PickupEvent::Result(item_id, outcome));
        }
        Ok(())
    }
}
impl Simulation {
    /// One genuine player/Demon world with source ammo, health and weapon items.
    pub fn original_player_pickups_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            limit,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonPickupsV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// Source pickup observations; no mutable player/inventory alias is exposed.
    pub fn original_player_pickups(
        &self,
        _slot: PlayerSlot,
    ) -> Result<PlayerPickupState, PlayerBodyError> {
        self.original_player
            .as_ref()
            .and_then(|p| p.pickups)
            .ok_or(PlayerBodyError::Profile)
    }
    pub(crate) fn preflight_world_pickups(&self) -> Result<(), SimError> {
        if !self.player_policy().has_world_pickups() {
            return Ok(());
        }
        self.original_player
            .as_ref()
            .and_then(|p| p.pickups)
            .ok_or(PlayerBodyError::Invariant)?
            .validate(self.player_policy(), self.world.episode1)?;
        if self.player_policy().has_player_armor() {
            self.original_player_armor(PlayerSlot::One)?;
        }
        let mut count = 0;
        let mut counted = 0;
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            let a = *self.actor(id).ok_or(StoreError::InvalidHandle)?;
            if a.life()
                .is_some_and(|l| kind_for(self.player_policy(), l.actor_type).is_some())
            {
                count += 1;
                counted += usize::from(a.life().is_some_and(|l| l.flags & MF_COUNTITEM != 0));
                validate_item(self.room(), self.item_sectors(), a)?;
            }
        }
        if self.player_policy().has_player_bonuses() {
            let total = self
                .statistics
                .ok_or(PlayerBodyError::Invariant)?
                .total_items;
            let collected = self
                .original_player
                .as_ref()
                .and_then(|p| p.pickups)
                .ok_or(PlayerBodyError::Invariant)?
                .item_count;
            if !(0..=pickup_bound(self.player_policy(), self.world.episode1) as i32)
                .contains(&total)
                || collected > total
                || counted > (total - collected) as usize
                || (total > 0
                    && total as usize
                        != counted_map_things(
                            self.world.map().things(),
                            self.world.episode1.is_some(),
                            self.thing_skill_bit(),
                        ))
            {
                return Err(PlayerBodyError::Invariant.into());
            }
        }
        if count > world_item_bound(self.player_policy(), self.world.episode1) {
            return Err(MapSpawnError::WorkLimit.into());
        }
        Ok(())
    }
    /// Controlled actual dropped Clip/Shotgun/Chaingun spawn, before body/tic0.
    /// This is setup, not monster loot, respawn or dynamic item creation.
    pub fn spawn_dropped_pickup(
        &mut self,
        k: PickupKind,
        position: [Fixed; 2],
    ) -> Result<ActorId, SimError> {
        if !self.player_policy().has_world_pickups()
            || self.tic != 0
            || self.original_player.as_ref().is_none_or(|p| p.mo.is_some())
            || !k.can_drop()
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.preflight_world_pickups()?;
        let items = self
            .actors
            .order()
            .filter(|id| {
                self.actor(*id)
                    .and_then(Actor::life)
                    .is_some_and(|l| kind_for(self.player_policy(), l.actor_type).is_some())
            })
            .count();
        if items >= pickup_bound(self.player_policy(), self.world.episode1) {
            return Err(MapSpawnError::WorkLimit.into());
        }
        if !self.spawn_admitted(
            position,
            Fixed::from_bits(k.original_type().descriptor().radius),
            978,
            false,
        ) {
            return Err(MonsterError::Arithmetic.into());
        }
        self.spawn_fits(
            ActorTypeId(k.original_type().ordinal()),
            position,
            SpawnHeight::Floor,
        )?;
        self.reserve_actor_spawns(1)?;
        let id = lifecycle::LifeHost {
            specials: None,
            buttons: None,
            player: None,
            statistics: self.statistics.as_mut(),
            perception: None,
            access: &mut self.actors,
            registry: self.registry.as_deref().ok_or(ActorError::InvalidType)?,
            world: &self.world,
            sectors: crate::staging::Sectors::committed(&self.sectors),
            options: self.actor_options,
            rng: &mut self.rng,
            spatial: &mut self.spatial,
            bindings: &mut self.bindings,
            events: &mut self.pending,
        }
        .spawn(
            ActorTypeId(k.original_type().ordinal()),
            position,
            SpawnHeight::Floor,
        );
        self.actors
            .get_mut(id)
            .and_then(|a| a.life.as_mut())
            .ok_or(StoreError::InvalidHandle)?
            .flags |= MF_DROPPED;
        Ok(id)
    }
}

/// Live world-item bound: placed/controlled items ([`max_pickups`]) plus, under
/// PlayerMonstersV1 and later, at most one P_KillMobj drop per admitted roster
/// monster.
pub(crate) const fn max_world_items(policy: OriginalExecutionProfile) -> usize {
    if policy.has_monsters() {
        max_pickups(policy) + crate::MAX_PLAYER_MONSTERS
    } else {
        max_pickups(policy)
    }
}
/// DoomEd numbers of the COUNTITEM health/armor bonuses (info.c MT_MISC2/MT_MISC3).
pub(crate) const COUNTED_ITEM_DOOMED: [i16; 2] = [2014, 2015];
/// The same set under the Episode 1 policy: the bonuses plus the soulsphere,
/// invisibility, computer map and light amplification (info.c MT_MISC12, MT_INS,
/// MT_MISC15, MT_MISC16 carry `MF_COUNTITEM`).
pub(crate) const COUNTED_ITEM_DOOMED_EPISODE1: [i16; 6] = [2014, 2015, 2013, 2024, 2026, 2045];
/// DoomEd numbers of the admitted COUNTKILL monsters (info.c MT_SERGEANT,
/// MT_POSSESSED, MT_SHOTGUY, MT_CHAINGUY).
pub(crate) const COUNTED_KILL_DOOMED: [i16; 4] = [3002, 3004, 9, 65];
/// The same set plus MT_TROOP under `PlayerMonstersImpV1`.
pub(crate) const COUNTED_KILL_DOOMED_IMPS: [i16; 5] = [3001, 3002, 3004, 9, 65];
/// The single source placement filter for counted THINGs: p_mobj.c
/// P_SpawnMapThing skips `options & 16` (MTF_NOTSINGLE) in single player and
/// requires the Medium skill bit `options & 2`. Every caller (runtime
/// preflight and DSIM readers) counts through this one definition. Actor
/// admission itself stays in the existing batch validator.
pub(crate) fn counted_things(things: impl IntoIterator<Item = (i16, u16)>, kinds: &[i16]) -> usize {
    counted_things_with_bit(things, kinds, 2)
}
/// [`counted_things`] with the skill bit `bit` in place of the medium bit
/// (p_mobj.c:743–751): wire 24 and the DSIM40 reader pass the bit of the run's
/// skill (M4b decision I4, [`crate::episode1::skill_bit`]); every older caller
/// keeps 2 through [`counted_things`].
pub(crate) fn counted_things_with_bit(
    things: impl IntoIterator<Item = (i16, u16)>,
    kinds: &[i16],
    bit: u16,
) -> usize {
    things
        .into_iter()
        .filter(|(kind, options)| kinds.contains(kind) && options & 16 == 0 && options & bit != 0)
        .count()
}
/// The placed counted items of `things`, with the skill bit `bit` (2 for every
/// policy before wire 24).
pub(crate) fn counted_map_things(things: &[doom_map::Thing], episode1: bool, bit: u16) -> usize {
    counted_things_with_bit(
        things.iter().map(|t| (t.kind, t.options)),
        if episode1 {
            &COUNTED_ITEM_DOOMED_EPISODE1
        } else {
            &COUNTED_ITEM_DOOMED
        },
        bit,
    )
}
impl Simulation {
    /// The THING option bit the item total's cross-check counts with: the bit of
    /// the run's skill under wire 24 (M4b decision I4), the medium bit (2) for
    /// every older policy, which admits only the default options.
    fn thing_skill_bit(&self) -> u16 {
        if self.player_policy().has_hard_skill() {
            crate::episode1::skill_bit(self.actor_options.skill)
        } else {
            2
        }
    }
}
