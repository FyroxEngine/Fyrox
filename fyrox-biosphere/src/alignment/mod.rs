use crate::capacity::ContainerLevel;
use crate::heraldry::SymbolicType;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionalRole {
    Engine,
    MajorSystem,
    Addon,
    Entity,
}

impl FunctionalRole {
    pub fn expected_level(&self) -> ContainerLevel {
        match self {
            FunctionalRole::Engine => ContainerLevel::Genesis,
            FunctionalRole::MajorSystem => ContainerLevel::Mythos,
            FunctionalRole::Addon => ContainerLevel::Container,
            FunctionalRole::Entity => ContainerLevel::Capsule,
        }
    }
}

impl fmt::Display for FunctionalRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionalRole::Engine => write!(f, "Engine"),
            FunctionalRole::MajorSystem => write!(f, "Major System"),
            FunctionalRole::Addon => write!(f, "Addon/Feature"),
            FunctionalRole::Entity => write!(f, "Entity"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreeWayAlignment {
    pub structural: ContainerLevel,
    pub functional: FunctionalRole,
    pub symbolic: SymbolicType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignmentError {
    StructuralFunctionalMismatch {
        structural: ContainerLevel,
        functional: FunctionalRole,
    },
    StructuralSymbolicMismatch {
        structural: ContainerLevel,
        symbolic: String,
        symbolic_level: ContainerLevel,
    },
    FunctionalSymbolicMismatch {
        functional: FunctionalRole,
        symbolic: String,
    },
}

impl fmt::Display for AlignmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlignmentError::StructuralFunctionalMismatch {
                structural,
                functional,
            } => {
                write!(
                    f,
                    "Structural-Functional misalignment: container is {structural} \
                     but functional role is {functional} (expected level {:?})",
                    functional.expected_level()
                )
            }
            AlignmentError::StructuralSymbolicMismatch {
                structural,
                symbolic,
                symbolic_level,
            } => {
                write!(
                    f,
                    "Structural-Symbolic misalignment: container is {structural} \
                     but symbolic type {symbolic} belongs to level {symbolic_level}"
                )
            }
            AlignmentError::FunctionalSymbolicMismatch {
                functional,
                symbolic,
            } => {
                write!(
                    f,
                    "Functional-Symbolic misalignment: role is {functional} \
                     but symbolic type {symbolic} does not match"
                )
            }
        }
    }
}

impl std::error::Error for AlignmentError {}

pub fn validate_alignment(alignment: &ThreeWayAlignment) -> Result<(), Vec<AlignmentError>> {
    let mut errors = Vec::new();

    // Structural ↔ Functional
    if alignment.structural != alignment.functional.expected_level() {
        errors.push(AlignmentError::StructuralFunctionalMismatch {
            structural: alignment.structural,
            functional: alignment.functional.clone(),
        });
    }

    // Structural ↔ Symbolic
    let symbolic_level = alignment.symbolic.level();
    if alignment.structural != symbolic_level {
        errors.push(AlignmentError::StructuralSymbolicMismatch {
            structural: alignment.structural,
            symbolic: alignment.symbolic.to_string(),
            symbolic_level,
        });
    }

    // Functional ↔ Symbolic
    if alignment.functional.expected_level() != symbolic_level {
        errors.push(AlignmentError::FunctionalSymbolicMismatch {
            functional: alignment.functional.clone(),
            symbolic: alignment.symbolic.to_string(),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heraldry::CrestName;

    #[test]
    fn valid_alignment_capsule() {
        let alignment = ThreeWayAlignment {
            structural: ContainerLevel::Capsule,
            functional: FunctionalRole::Entity,
            symbolic: SymbolicType::Sigil,
        };
        assert!(validate_alignment(&alignment).is_ok());
    }

    #[test]
    fn valid_alignment_genesis() {
        let alignment = ThreeWayAlignment {
            structural: ContainerLevel::Genesis,
            functional: FunctionalRole::Engine,
            symbolic: SymbolicType::GreaterSeal,
        };
        assert!(validate_alignment(&alignment).is_ok());
    }

    #[test]
    fn valid_alignment_mythos() {
        let alignment = ThreeWayAlignment {
            structural: ContainerLevel::Mythos,
            functional: FunctionalRole::MajorSystem,
            symbolic: SymbolicType::Crest(CrestName::Vault),
        };
        assert!(validate_alignment(&alignment).is_ok());
    }

    #[test]
    fn misaligned_structural_functional() {
        let alignment = ThreeWayAlignment {
            structural: ContainerLevel::Genesis,
            functional: FunctionalRole::Entity,
            symbolic: SymbolicType::GreaterSeal,
        };
        let errors = validate_alignment(&alignment).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, AlignmentError::StructuralFunctionalMismatch { .. })));
    }

    #[test]
    fn fully_misaligned() {
        let alignment = ThreeWayAlignment {
            structural: ContainerLevel::Genesis,
            functional: FunctionalRole::Entity,
            symbolic: SymbolicType::Sigil,
        };
        let errors = validate_alignment(&alignment).unwrap_err();
        assert!(errors.len() >= 2);
    }
}
