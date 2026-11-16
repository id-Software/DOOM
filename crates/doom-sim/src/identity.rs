//! Canonical Linux DOOM 1.10 metadata identities, separate from registry slots.
//!
//! Canonical identities cannot be used implicitly as registry-local candidates:
//! ```compile_fail
//! use doom_sim::{identity::OriginalActorType, ActorTypeId};
//! let slot: ActorTypeId = OriginalActorType::MT_SERGEANT;
//! ```
//!
//! Recognition never grants execution. Registry conversions compare one complete
//! record at its original ordinal; they do not certify downstream records or game
//! context. All existing profile admission checks still apply.
use crate::lifecycle::{SOURCE_ACTIONS, SOURCE_SPRITES, SOURCE_STATES, SOURCE_TYPES};
use crate::{ActionId, ActorAction, ActorRegistry, ActorTypeId, StateId};
use std::{error::Error, fmt};
mod names;

/// Rejected canonical value or a registry record that is not the original record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityError {
    /// Outside the original enumeration (including NUM* count sentinels).
    OutOfRange,
    /// Missing, replaced, renamed or altered record at the canonical ordinal.
    RegistryMismatch,
}
impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "original identity: {self:?}")
    }
}
impl Error for IdentityError {}

macro_rules! identity {
    ($ty:ident, $table:ident, $name:expr) => {
        const _: () = assert!($table.len() <= u16::MAX as usize);
        #[doc = $name]
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $ty(u16);
        impl $ty {
            /// Number of original entries; this value itself is not an identity.
            pub const COUNT: u16 = $table.len() as u16;
            /// Original zero-based enum ordinal, never a registry-local alias.
            pub const fn ordinal(self) -> u16 {
                self.0
            }
            /// All original identities in source order, without allocation.
            pub fn all() -> impl ExactSizeIterator<Item = Self> + DoubleEndedIterator {
                (0..Self::COUNT).map(Self)
            }
            /// Exact original symbolic name; case sensitive, with no aliases.
            pub fn from_name(name: &str) -> Option<Self> {
                Self::all().find(|id| id.name() == name)
            }
        }
        impl TryFrom<u16> for $ty {
            type Error = IdentityError;
            fn try_from(value: u16) -> Result<Self, Self::Error> {
                if value < Self::COUNT {
                    Ok(Self(value))
                } else {
                    Err(IdentityError::OutOfRange)
                }
            }
        }
        impl TryFrom<i32> for $ty {
            type Error = IdentityError;
            fn try_from(value: i32) -> Result<Self, Self::Error> {
                Self::try_from(u16::try_from(value).map_err(|_| IdentityError::OutOfRange)?)
            }
        }
        impl From<$ty> for u16 {
            fn from(value: $ty) -> Self {
                value.0
            }
        }
    };
}
identity!(
    OriginalActorType,
    SOURCE_TYPES,
    "Validated original mobjtype_t; metadata recognition only."
);
identity!(
    OriginalState,
    SOURCE_STATES,
    "Validated original statenum_t; S_NULL is a valid sentinel identity."
);
identity!(
    OriginalSprite,
    SOURCE_SPRITES,
    "Validated original spritenum_t; no image resource is loaded."
);

/// Eight original actor-state relationships. S_NULL means absent where applicable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OriginalActorStates {
    /// Raw spawn-state assignment, without action execution.
    pub spawn: OriginalState,
    /// Wake-up/pursuit sequence.
    pub see: OriginalState,
    /// Pain response.
    pub pain: OriginalState,
    /// Close-range attack sequence.
    pub melee: OriginalState,
    /// Ranged attack sequence.
    pub missile: OriginalState,
    /// Ordinary death sequence.
    pub death: OriginalState,
    /// Extreme death sequence.
    pub extreme_death: OriginalState,
    /// Resurrection sequence; representation does not admit resurrection.
    pub raise: OriginalState,
}

/// Original actor information in source units; metadata does not authorize behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OriginalActorDef {
    /// DoomEd number; -1 excludes ordinary map lookup.
    pub doomed_number: i32,
    /// All eight state relationships, retaining S_NULL sentinels.
    pub states: OriginalActorStates,
    /// Initial health.
    pub spawn_health: i32,
    /// Initial reaction time outside Nightmare.
    pub reaction_time: i32,
    /// Pain probability threshold.
    pub pain_chance: i32,
    /// See, attack, pain, death and active sound ordinals, as inert metadata.
    pub sounds: [u16; 5],
    /// Source speed; units depend on type (not universally fixed-point).
    pub speed: i32,
    /// Raw 16.16 radius.
    pub radius: i32,
    /// Raw 16.16 height.
    pub height: i32,
    /// Source mass.
    pub mass: i32,
    /// Source damage factor.
    pub damage: i32,
    /// Original MF_* bits; recognition does not enable unsupported flags.
    pub flags: u32,
}

/// Original immutable state descriptor. A callback symbol is metadata, not dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OriginalStateDef {
    /// Canonical sprite identity.
    pub sprite: OriginalSprite,
    /// Frame including original fullbright bits.
    pub frame: u32,
    /// -1 infinite, 0 immediate entry, positive countdown; raw spawn differs.
    pub tics: i32,
    /// Original function symbol, or None for a NULL pointer. No original action ordinal exists.
    pub action: Option<&'static str>,
    /// Original next reference; infinite states need not follow it.
    pub next: OriginalState,
    /// Preserved miscellaneous fields.
    pub misc: [i32; 2],
}
/// Checked conversion of a verified source value; a failure stops the build.
const fn source_u16(v: i32, what: &str) -> u16 {
    if v < 0 || v > u16::MAX as i32 {
        panic!("{}", what);
    }
    v as u16
}
const fn source_state(v: i32) -> OriginalState {
    let v = source_u16(v, "verified original state reference");
    assert!(
        (v as usize) < SOURCE_STATES.len(),
        "verified original state reference"
    );
    OriginalState(v)
}
/// `OriginalActorType::descriptor` of every type, decoded at compile time from
/// `SOURCE_TYPES` field by field exactly as the former per-call decoding did.
static ACTOR_DESCRIPTORS: [OriginalActorDef; SOURCE_TYPES.len()] = {
    const fn decode(p: &[i32; 23]) -> OriginalActorDef {
        assert!(p[21] >= 0, "verified original flags");
        OriginalActorDef {
            doomed_number: p[0],
            states: OriginalActorStates {
                spawn: source_state(p[1]),
                see: source_state(p[3]),
                pain: source_state(p[7]),
                melee: source_state(p[10]),
                missile: source_state(p[11]),
                death: source_state(p[12]),
                extreme_death: source_state(p[13]),
                raise: source_state(p[22]),
            },
            spawn_health: p[2],
            reaction_time: p[5],
            pain_chance: p[8],
            sounds: [
                source_u16(p[4], "verified original sound"),
                source_u16(p[6], "verified original sound"),
                source_u16(p[9], "verified original sound"),
                source_u16(p[14], "verified original sound"),
                source_u16(p[20], "verified original sound"),
            ],
            speed: p[15],
            radius: p[16],
            height: p[17],
            mass: p[18],
            damage: p[19],
            flags: p[21] as u32,
        }
    }
    let mut table = [decode(&SOURCE_TYPES[0].1); SOURCE_TYPES.len()];
    let mut i = 1;
    while i < SOURCE_TYPES.len() {
        table[i] = decode(&SOURCE_TYPES[i].1);
        i += 1;
    }
    table
};
/// `OriginalState::descriptor` of every state, decoded at compile time from
/// `SOURCE_STATES` field by field exactly as the former per-call decoding did.
static STATE_DESCRIPTORS: [OriginalStateDef; SOURCE_STATES.len()] = {
    const fn decode(v: &[i32; 7]) -> OriginalStateDef {
        let sprite = source_u16(v[0], "verified original sprite reference");
        assert!(
            (sprite as usize) < SOURCE_SPRITES.len(),
            "verified original sprite reference"
        );
        assert!(v[1] >= 0, "verified original frame bits");
        OriginalStateDef {
            sprite: OriginalSprite(sprite),
            frame: v[1] as u32,
            tics: v[2],
            action: if v[3] == 0 {
                None
            } else {
                assert!(v[3] > 0, "verified action reference");
                Some(SOURCE_ACTIONS[v[3] as usize])
            },
            next: source_state(v[4]),
            misc: [v[5], v[6]],
        }
    }
    let mut table = [decode(&SOURCE_STATES[0].1); SOURCE_STATES.len()];
    let mut i = 1;
    while i < SOURCE_STATES.len() {
        table[i] = decode(&SOURCE_STATES[i].1);
        i += 1;
    }
    table
};
impl OriginalActorType {
    /// Original MT_* symbol.
    pub fn name(self) -> &'static str {
        SOURCE_TYPES[usize::from(self.0)].0
    }
    /// All 23 original info.h fields, in declaration order and source units.
    /// This borrowed metadata has no registry allocation or execution capability.
    pub fn properties(self) -> &'static [i32; 23] {
        &SOURCE_TYPES[usize::from(self.0)].1
    }
    /// Named state references; every value is validated original metadata.
    pub fn states(self) -> OriginalActorStates {
        ACTOR_DESCRIPTORS[usize::from(self.0)].states
    }
    /// Named original actor information; all fields retain source units.
    ///
    /// Read from a table decoded once, at compile time, from the same source
    /// properties, so a reference that fails verification is a build error.
    pub fn descriptor(self) -> OriginalActorDef {
        ACTOR_DESCRIPTORS[usize::from(self.0)]
    }
    /// Resolve one exact record in an owned registry. Execution admission is separate.
    pub fn resolve(self, registry: &ActorRegistry) -> Result<ActorTypeId, IdentityError> {
        let id = ActorTypeId(self.0);
        match registry.actor_type(id) {
            Some(t) if t.name == self.name() && &t.properties == self.properties() => Ok(id),
            _ => Err(IdentityError::RegistryMismatch),
        }
    }
}
impl OriginalState {
    /// Original S_* symbol.
    pub fn name(self) -> &'static str {
        SOURCE_STATES[usize::from(self.0)].0
    }
    /// Exact original descriptor. Private static source data is independently checked against C.
    ///
    /// Read from a table decoded once, at compile time, from the same source record.
    pub fn descriptor(self) -> OriginalStateDef {
        STATE_DESCRIPTORS[usize::from(self.0)]
    }
    /// Resolve one complete state record, including next reference and callback.
    /// This does not certify the records reached by that next reference.
    pub fn resolve(self, registry: &ActorRegistry) -> Result<StateId, IdentityError> {
        let id = StateId(self.0);
        let d = self.descriptor();
        let v = SOURCE_STATES[usize::from(self.0)].1;
        let action = if v[3] == 0 {
            ActorAction::None
        } else {
            ActorAction::Original(ActionId(
                u16::try_from(v[3]).expect("verified action identity"),
            ))
        };
        match registry.state(id) {
            Some(s)
                if s.name == self.name()
                    && s.sprite == d.sprite.ordinal()
                    && s.frame == d.frame
                    && s.tics == d.tics
                    && s.action == action
                    && s.next.0 == d.next.ordinal()
                    && s.misc == d.misc =>
            {
                Ok(id)
            }
            _ => Err(IdentityError::RegistryMismatch),
        }
    }
}
impl OriginalSprite {
    /// Original SPR_* symbol (resource names are the suffix after SPR_).
    pub fn name(self) -> &'static str {
        SOURCE_SPRITES[usize::from(self.0)]
    }
    /// Original four-character sprite resource name; metadata only.
    pub fn resource_name(self) -> &'static str {
        &self.name()[4..]
    }
}
impl ActorRegistry {
    /// Explicitly identify one original actor record; native replacements reject.
    pub fn original_actor_type(&self, id: ActorTypeId) -> Result<OriginalActorType, IdentityError> {
        let original = OriginalActorType::try_from(id.0)?;
        original.resolve(self)?;
        Ok(original)
    }
    /// Explicitly identify one original state record, without granting execution.
    pub fn original_state(&self, id: StateId) -> Result<OriginalState, IdentityError> {
        let original = OriginalState::try_from(id.0)?;
        original.resolve(self)?;
        Ok(original)
    }
}

/// `source_u16` is a `const fn` run only at compile time on the static tables, whose verified
/// data never trips its range check; these unit tests call it at run time. Below the public
/// API because no public path passes it an out-of-range value.
#[cfg(test)]
mod source_u16_tests {
    use super::source_u16;

    /// Pins the lower bound (`v < 0`) of the check.
    #[test]
    #[should_panic(expected = "range probe")]
    fn rejects_minus_one() {
        source_u16(-1, "range probe");
    }

    /// Pins the upper bound (`v > u16::MAX`) of the check.
    #[test]
    #[should_panic(expected = "range probe")]
    fn rejects_65536() {
        source_u16(65536, "range probe");
    }

    /// Pins both inclusive ends: 0 and 65535 are in range and returned unchanged.
    #[test]
    fn accepts_the_inclusive_ends() {
        assert_eq!(source_u16(0, "range probe"), 0);
        assert_eq!(source_u16(65535, "range probe"), 65535);
    }
}
