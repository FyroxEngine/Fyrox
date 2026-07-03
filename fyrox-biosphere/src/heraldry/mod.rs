use crate::capacity::ContainerLevel;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolicType {
    // Level 1 — Genesis
    GreaterSeal,
    LesserSeal,

    // Level 2 — Mythos
    Crest(CrestName),

    // Level 3 — Container
    Glyph,
    Device,
    Emblem,

    // Level 4 — Capsule
    Trait,
    Mark,
    Token,
    Sigil,
}

impl SymbolicType {
    pub fn level(&self) -> ContainerLevel {
        match self {
            SymbolicType::GreaterSeal | SymbolicType::LesserSeal => ContainerLevel::Genesis,
            SymbolicType::Crest(_) => ContainerLevel::Mythos,
            SymbolicType::Glyph | SymbolicType::Device | SymbolicType::Emblem => {
                ContainerLevel::Container
            }
            SymbolicType::Trait
            | SymbolicType::Mark
            | SymbolicType::Token
            | SymbolicType::Sigil => ContainerLevel::Capsule,
        }
    }

    pub fn authority(&self) -> Authority {
        match self.level() {
            ContainerLevel::Genesis => Authority::Cosmic,
            ContainerLevel::Mythos => Authority::Dynastic,
            ContainerLevel::Container => Authority::Modular,
            ContainerLevel::Capsule => Authority::Granular,
        }
    }

    pub fn permanence(&self) -> Option<Permanence> {
        match self {
            SymbolicType::Trait => Some(Permanence::SemiPermanent),
            SymbolicType::Mark => Some(Permanence::Variable),
            SymbolicType::Token => Some(Permanence::Temporary),
            SymbolicType::Sigil => Some(Permanence::SemiPermanent),
            _ => None,
        }
    }

    pub fn is_seal(&self) -> bool {
        matches!(self, SymbolicType::GreaterSeal | SymbolicType::LesserSeal)
    }

    pub fn is_crest(&self) -> bool {
        matches!(self, SymbolicType::Crest(_))
    }
}

impl fmt::Display for SymbolicType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolicType::GreaterSeal => write!(f, "Greater Seal"),
            SymbolicType::LesserSeal => write!(f, "Lesser Seal"),
            SymbolicType::Crest(name) => write!(f, "Crest ({name})"),
            SymbolicType::Glyph => write!(f, "Glyph"),
            SymbolicType::Device => write!(f, "Device"),
            SymbolicType::Emblem => write!(f, "Emblem"),
            SymbolicType::Trait => write!(f, "Trait"),
            SymbolicType::Mark => write!(f, "Mark"),
            SymbolicType::Token => write!(f, "Token"),
            SymbolicType::Sigil => write!(f, "Sigil"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CrestName {
    Core,
    Atlas,
    Vault,
    Mythos,
    Codex,
    Loom,
    Composer,
    Forge,
    Order,
    Mind,
    Soul,
    Custom(String),
}

impl CrestName {
    pub fn is_known(&self) -> bool {
        !matches!(self, CrestName::Custom(_))
    }

    pub const KNOWN_CRESTS: &[&str] = &[
        "Core", "Atlas", "Vault", "Mythos", "Codex", "Loom", "Composer", "Forge", "Order", "Mind",
        "Soul",
    ];
}

impl fmt::Display for CrestName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CrestName::Core => write!(f, "Core"),
            CrestName::Atlas => write!(f, "Atlas"),
            CrestName::Vault => write!(f, "Vault"),
            CrestName::Mythos => write!(f, "Mythos"),
            CrestName::Codex => write!(f, "Codex"),
            CrestName::Loom => write!(f, "Loom"),
            CrestName::Composer => write!(f, "Composer"),
            CrestName::Forge => write!(f, "Forge"),
            CrestName::Order => write!(f, "Order"),
            CrestName::Mind => write!(f, "Mind"),
            CrestName::Soul => write!(f, "Soul"),
            CrestName::Custom(name) => write!(f, "{name}"),
        }
    }
}

impl std::str::FromStr for CrestName {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "core" => CrestName::Core,
            "atlas" => CrestName::Atlas,
            "vault" => CrestName::Vault,
            "mythos" => CrestName::Mythos,
            "codex" => CrestName::Codex,
            "loom" => CrestName::Loom,
            "composer" => CrestName::Composer,
            "forge" => CrestName::Forge,
            "order" => CrestName::Order,
            "mind" => CrestName::Mind,
            "soul" => CrestName::Soul,
            _ => CrestName::Custom(s.to_string()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Authority {
    Cosmic,
    Dynastic,
    Modular,
    Granular,
}

impl fmt::Display for Authority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Authority::Cosmic => write!(f, "Cosmic — cannot be overridden"),
            Authority::Dynastic => write!(f, "Dynastic — defines system identity"),
            Authority::Modular => write!(f, "Modular — composable capability"),
            Authority::Granular => write!(f, "Granular — atomic narrative content"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permanence {
    SemiPermanent,
    Variable,
    Temporary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeraldryError {
    LevelMismatch {
        expected_level: ContainerLevel,
        symbolic_type: String,
        symbolic_level: ContainerLevel,
    },
    TooManyCustomCrests {
        current_custom: usize,
        max_custom: usize,
    },
}

impl fmt::Display for HeraldryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeraldryError::LevelMismatch {
                expected_level,
                symbolic_type,
                symbolic_level,
            } => {
                write!(
                    f,
                    "Heraldry mismatch: {symbolic_type} is level {symbolic_level} \
                     but container is level {expected_level}"
                )
            }
            HeraldryError::TooManyCustomCrests {
                current_custom,
                max_custom,
            } => {
                write!(
                    f,
                    "Too many custom Crests: {current_custom}/{max_custom} \
                     (11 known + up to 5 custom = 16 max)"
                )
            }
        }
    }
}

impl std::error::Error for HeraldryError {}

pub const MAX_CUSTOM_CRESTS: usize = 5;
pub const KNOWN_CREST_COUNT: usize = 11;

pub fn validate_heraldry(
    container_level: ContainerLevel,
    symbolic_type: &SymbolicType,
) -> Result<(), HeraldryError> {
    let symbolic_level = symbolic_type.level();
    if container_level != symbolic_level {
        return Err(HeraldryError::LevelMismatch {
            expected_level: container_level,
            symbolic_type: symbolic_type.to_string(),
            symbolic_level,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_is_genesis_level() {
        assert_eq!(SymbolicType::GreaterSeal.level(), ContainerLevel::Genesis);
        assert_eq!(SymbolicType::LesserSeal.level(), ContainerLevel::Genesis);
    }

    #[test]
    fn crest_is_mythos_level() {
        assert_eq!(
            SymbolicType::Crest(CrestName::Vault).level(),
            ContainerLevel::Mythos
        );
    }

    #[test]
    fn glyph_device_emblem_are_container_level() {
        assert_eq!(SymbolicType::Glyph.level(), ContainerLevel::Container);
        assert_eq!(SymbolicType::Device.level(), ContainerLevel::Container);
        assert_eq!(SymbolicType::Emblem.level(), ContainerLevel::Container);
    }

    #[test]
    fn capsule_types() {
        assert_eq!(SymbolicType::Trait.level(), ContainerLevel::Capsule);
        assert_eq!(SymbolicType::Mark.level(), ContainerLevel::Capsule);
        assert_eq!(SymbolicType::Token.level(), ContainerLevel::Capsule);
        assert_eq!(SymbolicType::Sigil.level(), ContainerLevel::Capsule);
    }

    #[test]
    fn permanence_mapping() {
        assert_eq!(
            SymbolicType::Trait.permanence(),
            Some(Permanence::SemiPermanent)
        );
        assert_eq!(SymbolicType::Mark.permanence(), Some(Permanence::Variable));
        assert_eq!(
            SymbolicType::Token.permanence(),
            Some(Permanence::Temporary)
        );
        assert_eq!(
            SymbolicType::Sigil.permanence(),
            Some(Permanence::SemiPermanent)
        );
        assert_eq!(SymbolicType::Glyph.permanence(), None);
    }

    #[test]
    fn heraldry_validation() {
        assert!(validate_heraldry(ContainerLevel::Genesis, &SymbolicType::GreaterSeal).is_ok());
        assert!(validate_heraldry(
            ContainerLevel::Mythos,
            &SymbolicType::Crest(CrestName::Core)
        )
        .is_ok());
        assert!(validate_heraldry(ContainerLevel::Container, &SymbolicType::Device).is_ok());
        assert!(validate_heraldry(ContainerLevel::Capsule, &SymbolicType::Sigil).is_ok());

        // Misalignment
        assert!(validate_heraldry(ContainerLevel::Genesis, &SymbolicType::Glyph).is_err());
        assert!(validate_heraldry(ContainerLevel::Capsule, &SymbolicType::GreaterSeal).is_err());
    }

    #[test]
    fn crest_parsing() {
        assert_eq!("vault".parse::<CrestName>().unwrap(), CrestName::Vault);
        assert_eq!("Core".parse::<CrestName>().unwrap(), CrestName::Core);
        assert_eq!(
            "MyCustom".parse::<CrestName>().unwrap(),
            CrestName::Custom("MyCustom".to_string())
        );
    }
}
