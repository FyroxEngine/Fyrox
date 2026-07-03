use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WireType {
    Data = 1,
    Control = 2,
    Audio = 3,
    Narrative = 4,
    Temporal = 5,
    Agent = 6,
    Visual = 7,
    Spatial = 8,
    Behavioral = 9,
    Social = 10,
    Energy = 11,
    Identity = 12,
    Event = 13,
    Asset = 14,
    Meta = 15,
    Logic = 16,
}

impl WireType {
    pub fn abbreviation(&self) -> &'static str {
        match self {
            WireType::Data => "DAT",
            WireType::Control => "CTL",
            WireType::Audio => "AUD",
            WireType::Narrative => "NAR",
            WireType::Temporal => "TMP",
            WireType::Agent => "AGT",
            WireType::Visual => "VIS",
            WireType::Spatial => "SPA",
            WireType::Behavioral => "BHV",
            WireType::Social => "SOC",
            WireType::Energy => "ENR",
            WireType::Identity => "IDN",
            WireType::Event => "EVT",
            WireType::Asset => "AST",
            WireType::Meta => "MET",
            WireType::Logic => "LGC",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            WireType::Data => "Typed JSON/typed payloads",
            WireType::Control => "Boolean, gate, trigger signals",
            WireType::Audio => "Waveform / sample streams",
            WireType::Narrative => "Story / text / lore payloads",
            WireType::Temporal => "Time, tick, clock signals",
            WireType::Agent => "Agent instruction / state streams",
            WireType::Visual => "Image / render / shader streams",
            WireType::Spatial => "3D / voxel / coordinate data",
            WireType::Behavioral => "Emotion / drive / decision signals",
            WireType::Social => "Relationship / faction / reputation data",
            WireType::Energy => "Power / resource flow values",
            WireType::Identity => "B-DNA / lineage / covenant data",
            WireType::Event => "Cosmic bus events",
            WireType::Asset => "File / binary / media references",
            WireType::Meta => "Schema / type / structure definitions",
            WireType::Logic => "Boolean expression / rule streams",
        }
    }

    pub fn is_universal_fallback(&self) -> bool {
        matches!(self, WireType::Data)
    }

    pub const ALL: [WireType; 16] = [
        WireType::Data,
        WireType::Control,
        WireType::Audio,
        WireType::Narrative,
        WireType::Temporal,
        WireType::Agent,
        WireType::Visual,
        WireType::Spatial,
        WireType::Behavioral,
        WireType::Social,
        WireType::Energy,
        WireType::Identity,
        WireType::Event,
        WireType::Asset,
        WireType::Meta,
        WireType::Logic,
    ];
}

impl fmt::Display for WireType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.abbreviation(), self.description())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WirePort {
    pub name: String,
    pub wire_type: WireType,
    pub direction: PortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireConnection {
    pub source_entity_id: String,
    pub source_port: String,
    pub target_entity_id: String,
    pub target_port: String,
    pub wire_type: WireType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    TypeMismatch {
        source_type: WireType,
        target_type: WireType,
    },
    PortNotFound {
        entity_id: String,
        port_name: String,
    },
    DirectionMismatch {
        port_name: String,
        expected: PortDirection,
        actual: PortDirection,
    },
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireError::TypeMismatch {
                source_type,
                target_type,
            } => {
                write!(
                    f,
                    "Wire type mismatch: source is {} but target expects {} \
                     (only DAT is a universal fallback)",
                    source_type.abbreviation(),
                    target_type.abbreviation()
                )
            }
            WireError::PortNotFound {
                entity_id,
                port_name,
            } => {
                write!(f, "Port '{port_name}' not found on entity '{entity_id}'")
            }
            WireError::DirectionMismatch {
                port_name,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Port '{port_name}' direction mismatch: expected {expected:?}, got {actual:?}"
                )
            }
        }
    }
}

impl std::error::Error for WireError {}

pub fn check_wire_compatibility(
    source_type: WireType,
    target_type: WireType,
) -> Result<(), WireError> {
    if source_type == target_type || source_type.is_universal_fallback() {
        Ok(())
    } else {
        Err(WireError::TypeMismatch {
            source_type,
            target_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_16_wire_types() {
        assert_eq!(WireType::ALL.len(), 16);
    }

    #[test]
    fn matching_types_compatible() {
        assert!(check_wire_compatibility(WireType::Audio, WireType::Audio).is_ok());
        assert!(check_wire_compatibility(WireType::Narrative, WireType::Narrative).is_ok());
    }

    #[test]
    fn data_is_universal_fallback() {
        for wt in &WireType::ALL {
            assert!(check_wire_compatibility(WireType::Data, *wt).is_ok());
        }
    }

    #[test]
    fn mismatched_types_rejected() {
        assert!(check_wire_compatibility(WireType::Audio, WireType::Visual).is_err());
        assert!(check_wire_compatibility(WireType::Narrative, WireType::Spatial).is_err());
    }

    #[test]
    fn non_data_not_universal() {
        assert!(check_wire_compatibility(WireType::Audio, WireType::Data).is_err());
    }

    #[test]
    fn abbreviations_unique() {
        let abbrevs: Vec<&str> = WireType::ALL.iter().map(|w| w.abbreviation()).collect();
        let unique: std::collections::HashSet<&str> = abbrevs.iter().copied().collect();
        assert_eq!(abbrevs.len(), unique.len());
    }
}
