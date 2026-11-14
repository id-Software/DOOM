//! Typed original action identity for the canonical dispatch family.
//!
//! `ActionId` indexes the original action table. Dispatch for the admitted Demon,
//! former-human, Imp and player-weapon callbacks matches this enum, which is
//! resolved once at compile time from the source names, so action dispatch
//! compares no strings. Admission keeps its own explicit per-profile tables: naming a
//! variant here grants nothing.
use crate::{ActionId, lifecycle::SOURCE_ACTIONS};

macro_rules! original_actions {
    ($($variant:ident = $name:literal,)+) => {
        /// One admitted original callback.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub(crate) enum OriginalAction {
            $($variant,)+
        }
        const NAMED: &[(OriginalAction, &str)] = &[$((OriginalAction::$variant, $name),)+];
    };
}
original_actions! {
    Look = "A_Look",
    Chase = "A_Chase",
    FaceTarget = "A_FaceTarget",
    SargAttack = "A_SargAttack",
    PosAttack = "A_PosAttack",
    SPosAttack = "A_SPosAttack",
    CPosAttack = "A_CPosAttack",
    CPosRefire = "A_CPosRefire",
    TroopAttack = "A_TroopAttack",
    Pain = "A_Pain",
    Scream = "A_Scream",
    XScream = "A_XScream",
    Fall = "A_Fall",
    PlayerScream = "A_PlayerScream",
    Raise = "A_Raise",
    Lower = "A_Lower",
    WeaponReady = "A_WeaponReady",
    ReFire = "A_ReFire",
    FirePistol = "A_FirePistol",
    FireShotgun = "A_FireShotgun",
    FireCGun = "A_FireCGun",
    Punch = "A_Punch",
    Light0 = "A_Light0",
    Light1 = "A_Light1",
    Light2 = "A_Light2",
    Explode = "A_Explode",
    Saw = "A_Saw",
    FireMissile = "A_FireMissile",
    GunFlash = "A_GunFlash",
    BruisAttack = "A_BruisAttack",
    BossDeath = "A_BossDeath",
}
const fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}
const ACTION_COUNT: usize = SOURCE_ACTIONS.len();
/// Compile-time table; a name absent from the source table fails the build.
const BY_ORDINAL: [Option<OriginalAction>; ACTION_COUNT] = {
    let mut table = [None; ACTION_COUNT];
    let mut named = 0;
    while named < NAMED.len() {
        let mut ordinal = 0;
        while !same(SOURCE_ACTIONS[ordinal], NAMED[named].1) {
            ordinal += 1;
        }
        table[ordinal] = Some(NAMED[named].0);
        named += 1;
    }
    table
};
impl OriginalAction {
    /// Identity of an original action, or None outside the canonical family.
    pub(crate) fn of(id: ActionId) -> Option<Self> {
        BY_ORDINAL.get(usize::from(id.0)).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActorAction, OriginalExecutionProfile, OriginalExecutionRegistry, StateId,
        identity::{OriginalActorType, OriginalState},
        original_registry,
    };

    #[test]
    fn typed_identity_matches_the_original_action_table() {
        for (ordinal, name) in SOURCE_ACTIONS.iter().enumerate() {
            let typed = OriginalAction::of(ActionId(ordinal as u16));
            let listed = NAMED.iter().find(|(_, n)| n == name).map(|(a, _)| *a);
            assert_eq!(typed, listed, "{name}");
        }
        assert_eq!(OriginalAction::of(ActionId(u16::MAX)), None);
    }

    #[test]
    fn every_action_of_the_canonical_family_has_a_typed_identity() {
        let profile = OriginalExecutionProfile::PlayerMonstersImpV1;
        let registry = OriginalExecutionRegistry::new(profile).unwrap();
        let mut seen = 0;
        for ty in [
            OriginalActorType::MT_PLAYER,
            OriginalActorType::MT_SERGEANT,
            OriginalActorType::MT_POSSESSED,
            OriginalActorType::MT_SHOTGUY,
            OriginalActorType::MT_CHAINGUY,
            OriginalActorType::MT_TROOP,
            OriginalActorType::MT_TROOPSHOT,
        ] {
            for state in OriginalState::all() {
                let id = StateId(state.ordinal());
                if original_registry::state_role_for(profile, crate::ActorTypeId(ty.ordinal()), id)
                    .is_err()
                {
                    continue;
                }
                let definition = registry.registry().state(id).unwrap();
                if let ActorAction::Original(action) = definition.action {
                    assert!(OriginalAction::of(action).is_some(), "{state:?}");
                    seen += 1;
                }
            }
        }
        for weapon in [
            crate::Weapon::Fist,
            crate::Weapon::Pistol,
            crate::Weapon::Shotgun,
            crate::Weapon::Chaingun,
        ] {
            for state in OriginalState::all() {
                if !crate::player_weapons::weapon_state(weapon, state) {
                    continue;
                }
                let id = StateId(state.ordinal());
                if let ActorAction::Original(action) = registry.registry().state(id).unwrap().action
                {
                    assert!(OriginalAction::of(action).is_some(), "{state:?}");
                    seen += 1;
                }
            }
        }
        assert!(seen > 0);
    }
}
