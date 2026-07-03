use serde::{Deserialize, Serialize};
use std::fmt;

/// Default octave — 2^4 = 16 children.
pub const DEFAULT_OCTAVE: u8 = 4;
pub const MAX_CHILDREN: usize = 16;

/// Capacity is 2^octave. Octave 4 = 16 (the canonical default).
pub fn octave_capacity(octave: u8) -> usize {
    1usize << octave
}

/// Fixed: capacity locked at `declared_octave` forever (VHD-style).
/// Dynamic: starts at octave 1, expands up to `estimated_ceiling_octave` (VHDX-style).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GrowthMode {
    Fixed,
    Dynamic,
}

/// Greater = root seal, exactly one per metaverse.
/// Lesser = all others; must declare a `harmonic_ratio` to parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SealType {
    Greater,
    Lesser,
}

impl fmt::Display for SealType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SealType::Greater => write!(f, "Greater Seal"),
            SealType::Lesser => write!(f, "Lesser Seal"),
        }
    }
}

/// Tracks live capacity state for any container level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityMetadata {
    pub growth_mode: GrowthMode,
    pub declared_octave: u8,
    pub estimated_ceiling_octave: u8,
    pub current_octave: u8,
    pub sealed_octave: Option<u8>,
    pub child_count: usize,
}

impl CapacityMetadata {
    pub fn fixed(octave: u8) -> Self {
        Self {
            growth_mode: GrowthMode::Fixed,
            declared_octave: octave,
            estimated_ceiling_octave: octave,
            current_octave: octave,
            sealed_octave: None,
            child_count: 0,
        }
    }

    pub fn default_fixed() -> Self {
        Self::fixed(DEFAULT_OCTAVE)
    }

    pub fn max_children_current(&self) -> usize {
        octave_capacity(self.current_octave)
    }

    pub fn capacity_utilization(&self) -> f32 {
        let max = self.max_children_current();
        if max == 0 { return 1.0; }
        self.child_count as f32 / max as f32
    }

    pub fn can_add_child(&self) -> bool {
        self.child_count < self.max_children_current()
    }
}

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
