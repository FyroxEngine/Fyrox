/// The 17 canonical signal wire types of the Quantum Ecosystem.
///
/// Wire types are the ONLY legal interface between modules. A module publishes
/// packets onto a wire type. The Theater routes by type. Adapters subscribe to
/// the types they render. Modules never know who is listening.
///
/// The 3-character codes ("SPA", "BHV", …) are used in the rack UI, Scribe
/// binary streams, and JSON manifests. The enum is used everywhere in Rust code.
///
/// **The 17 types are closed. Do not add new ones without a deliberate decision
/// documented in an ADR. Extend `DAT` payload schemas instead.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum WireType {
    Data      = 0,  // DAT — typed payloads, universal fallback
    Control   = 1,  // CTL — boolean, gate, trigger signals
    Audio     = 2,  // AUD — waveform / sample streams
    Narrative = 3,  // NAR — story / text / lore payloads
    Temporal  = 4,  // TMP — time, tick, clock signals
    Agent     = 5,  // AGT — agent instruction / state streams
    Visual    = 6,  // VIS — image / render / shader streams
    Spatial   = 7,  // SPA — 3D / voxel / coordinate data
    Behavioral = 8, // BHV — emotion / drive / decision signals
    Social    = 9,  // SOC — relationship / faction / reputation data
    Energy    = 10, // ENR — power / resource flow values
    Identity  = 11, // IDN — B-DNA / lineage / covenant data
    Event     = 12, // EVT — COSMIC bus events
    Asset     = 13, // AST — file / binary / media references
    Meta      = 14, // MET — schema / type / structure definitions
    Logic     = 15, // LGC — boolean expression / rule streams
    Resonance = 16, // RES — resonance field values (gravity of the graph)
}

impl WireType {
    /// All 17 wire types in canonical order.
    pub const ALL: [WireType; 17] = [
        Self::Data, Self::Control, Self::Audio, Self::Narrative, Self::Temporal,
        Self::Agent, Self::Visual, Self::Spatial, Self::Behavioral, Self::Social,
        Self::Energy, Self::Identity, Self::Event, Self::Asset, Self::Meta,
        Self::Logic, Self::Resonance,
    ];

    /// 3-character code used in the rack UI, Scribe streams, and JSON manifests.
    pub fn code(self) -> &'static str {
        match self {
            Self::Data       => "DAT",
            Self::Control    => "CTL",
            Self::Audio      => "AUD",
            Self::Narrative  => "NAR",
            Self::Temporal   => "TMP",
            Self::Agent      => "AGT",
            Self::Visual     => "VIS",
            Self::Spatial    => "SPA",
            Self::Behavioral => "BHV",
            Self::Social     => "SOC",
            Self::Energy     => "ENR",
            Self::Identity   => "IDN",
            Self::Event      => "EVT",
            Self::Asset      => "AST",
            Self::Meta       => "MET",
            Self::Logic      => "LGC",
            Self::Resonance  => "RES",
        }
    }

    /// Resolve a 3-char code back to a WireType. Returns None for unknown codes.
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "DAT" => Some(Self::Data),
            "CTL" => Some(Self::Control),
            "AUD" => Some(Self::Audio),
            "NAR" => Some(Self::Narrative),
            "TMP" => Some(Self::Temporal),
            "AGT" => Some(Self::Agent),
            "VIS" => Some(Self::Visual),
            "SPA" => Some(Self::Spatial),
            "BHV" => Some(Self::Behavioral),
            "SOC" => Some(Self::Social),
            "ENR" => Some(Self::Energy),
            "IDN" => Some(Self::Identity),
            "EVT" => Some(Self::Event),
            "AST" => Some(Self::Asset),
            "MET" => Some(Self::Meta),
            "LGC" => Some(Self::Logic),
            "RES" => Some(Self::Resonance),
            _     => None,
        }
    }

    /// Human-readable name for display.
    pub fn label(self) -> &'static str {
        match self {
            Self::Data       => "Data",
            Self::Control    => "Control",
            Self::Audio      => "Audio",
            Self::Narrative  => "Narrative",
            Self::Temporal   => "Temporal",
            Self::Agent      => "Agent",
            Self::Visual     => "Visual",
            Self::Spatial    => "Spatial",
            Self::Behavioral => "Behavioral",
            Self::Social     => "Social",
            Self::Energy     => "Energy",
            Self::Identity   => "Identity",
            Self::Event      => "Event",
            Self::Asset      => "Asset",
            Self::Meta       => "Meta",
            Self::Logic      => "Logic",
            Self::Resonance  => "Resonance",
        }
    }
}

impl std::fmt::Display for WireType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_17_variants_have_codes() {
        assert_eq!(WireType::ALL.len(), 17);
        for wt in WireType::ALL {
            let code = wt.code();
            assert_eq!(code.len(), 3, "code {code:?} should be 3 chars");
            assert_eq!(WireType::from_code(code), Some(wt), "round-trip failed for {code}");
        }
    }

    #[test]
    fn unknown_code_returns_none() {
        assert_eq!(WireType::from_code("XYZ"), None);
        assert_eq!(WireType::from_code(""), None);
    }

    #[test]
    fn serializes_with_bincode() {
        for wt in WireType::ALL {
            let bytes = bincode::serialize(&wt).unwrap();
            let back: WireType = bincode::deserialize(&bytes).unwrap();
            assert_eq!(wt, back);
        }
    }
}
