use serde::{Deserialize, Serialize};
use std::fmt;

pub const MAX_CHILDREN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContainerLevel {
    Genesis,
    Mythos,
    Container,
    Capsule,
}

impl ContainerLevel {
    pub fn depth(&self) -> u8 {
        match self {
            ContainerLevel::Genesis => 1,
            ContainerLevel::Mythos => 2,
            ContainerLevel::Container => 3,
            ContainerLevel::Capsule => 4,
        }
    }

    pub fn child_level(&self) -> Option<ContainerLevel> {
        match self {
            ContainerLevel::Genesis => Some(ContainerLevel::Mythos),
            ContainerLevel::Mythos => Some(ContainerLevel::Container),
            ContainerLevel::Container => Some(ContainerLevel::Capsule),
            ContainerLevel::Capsule => None,
        }
    }

    pub fn parent_level(&self) -> Option<ContainerLevel> {
        match self {
            ContainerLevel::Genesis => None,
            ContainerLevel::Mythos => Some(ContainerLevel::Genesis),
            ContainerLevel::Container => Some(ContainerLevel::Mythos),
            ContainerLevel::Capsule => Some(ContainerLevel::Container),
        }
    }

    pub fn max_total_descendants(&self) -> usize {
        match self {
            ContainerLevel::Genesis => 16 * 16 * 16,  // 4,096
            ContainerLevel::Mythos => 16 * 16,         // 256
            ContainerLevel::Container => 16,
            ContainerLevel::Capsule => 0,
        }
    }
}

impl fmt::Display for ContainerLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerLevel::Genesis => write!(f, "Genesis"),
            ContainerLevel::Mythos => write!(f, "Mythos"),
            ContainerLevel::Container => write!(f, "Container"),
            ContainerLevel::Capsule => write!(f, "Capsule"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lifecycle {
    Seeding,
    Active,
    Sealed,
    Archived,
    Deprecated,
}

impl Lifecycle {
    pub fn is_mutable(&self) -> bool {
        matches!(self, Lifecycle::Seeding | Lifecycle::Active)
    }

    pub fn can_seal(&self) -> bool {
        matches!(self, Lifecycle::Active)
    }

    pub fn is_sealed(&self) -> bool {
        matches!(self, Lifecycle::Sealed)
    }
}

impl fmt::Display for Lifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lifecycle::Seeding => write!(f, "Seeding"),
            Lifecycle::Active => write!(f, "Active"),
            Lifecycle::Sealed => write!(f, "Sealed"),
            Lifecycle::Archived => write!(f, "Archived"),
            Lifecycle::Deprecated => write!(f, "Deprecated"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapacityError {
    Overflow {
        level: ContainerLevel,
        current_count: usize,
    },
    SealedContainer {
        id: String,
    },
    InvalidChildLevel {
        parent: ContainerLevel,
        attempted_child: ContainerLevel,
    },
}

impl fmt::Display for CapacityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapacityError::Overflow { level, current_count } => {
                write!(
                    f,
                    "Capacity overflow at {level} level: {current_count}/{MAX_CHILDREN} \
                     (must split into a new sibling, cannot overflow)"
                )
            }
            CapacityError::SealedContainer { id } => {
                write!(f, "Container '{id}' is sealed — hierarchy is frozen")
            }
            CapacityError::InvalidChildLevel { parent, attempted_child } => {
                write!(
                    f,
                    "Cannot add {attempted_child} as child of {parent} \
                     (expected {:?})",
                    parent.child_level()
                )
            }
        }
    }
}

impl std::error::Error for CapacityError {}

pub fn check_capacity(current_children: usize, level: ContainerLevel) -> Result<(), CapacityError> {
    if current_children >= MAX_CHILDREN {
        Err(CapacityError::Overflow {
            level,
            current_count: current_children,
        })
    } else {
        Ok(())
    }
}

pub fn check_child_level(
    parent: ContainerLevel,
    child: ContainerLevel,
) -> Result<(), CapacityError> {
    match parent.child_level() {
        Some(expected) if expected == child => Ok(()),
        _ => Err(CapacityError::InvalidChildLevel {
            parent,
            attempted_child: child,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_law_enforced() {
        assert!(check_capacity(0, ContainerLevel::Genesis).is_ok());
        assert!(check_capacity(15, ContainerLevel::Genesis).is_ok());
        assert!(check_capacity(16, ContainerLevel::Genesis).is_err());
        assert!(check_capacity(100, ContainerLevel::Mythos).is_err());
    }

    #[test]
    fn child_level_validation() {
        assert!(check_child_level(ContainerLevel::Genesis, ContainerLevel::Mythos).is_ok());
        assert!(check_child_level(ContainerLevel::Mythos, ContainerLevel::Container).is_ok());
        assert!(check_child_level(ContainerLevel::Container, ContainerLevel::Capsule).is_ok());
        assert!(check_child_level(ContainerLevel::Capsule, ContainerLevel::Capsule).is_err());
        assert!(check_child_level(ContainerLevel::Genesis, ContainerLevel::Capsule).is_err());
    }

    #[test]
    fn lifecycle_transitions() {
        assert!(Lifecycle::Seeding.is_mutable());
        assert!(Lifecycle::Active.is_mutable());
        assert!(!Lifecycle::Sealed.is_mutable());
        assert!(Lifecycle::Active.can_seal());
        assert!(!Lifecycle::Seeding.can_seal());
    }

    #[test]
    fn max_descendants() {
        assert_eq!(ContainerLevel::Genesis.max_total_descendants(), 4096);
        assert_eq!(ContainerLevel::Mythos.max_total_descendants(), 256);
        assert_eq!(ContainerLevel::Container.max_total_descendants(), 16);
        assert_eq!(ContainerLevel::Capsule.max_total_descendants(), 0);
    }
}
