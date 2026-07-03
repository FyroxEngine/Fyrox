use crate::{
    alignment::{FunctionalRole, ThreeWayAlignment, validate_alignment},
    bdna::{BDnaRecord, BDnaError},
    capacity::{
        self, CapacityError, CapacityMetadata, ContainerLevel, Lifecycle, SealType, MAX_CHILDREN,
    },
    domain::Domain,
    heraldry::{CrestName, HeraldryError, SymbolicType, validate_heraldry},
    wire::{WirePort, WireType},
};
use serde::{Deserialize, Serialize};
use std::fmt;

mod bdna_bits_serde {
    use serde::{Deserializer, Serializer, de::{SeqAccess, Visitor}, ser::SerializeSeq};

    pub fn serialize<S: Serializer>(bits: &[bool; 64], ser: S) -> Result<S::Ok, S::Error> {
        let mut seq = ser.serialize_seq(Some(64))?;
        for b in bits { seq.serialize_element(b)?; }
        seq.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[bool; 64], D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = [bool; 64];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "array of 64 booleans")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<[bool; 64], A::Error> {
                let mut bits = [false; 64];
                for slot in &mut bits { *slot = seq.next_element()?.unwrap_or(false); }
                Ok(bits)
            }
        }
        de.deserialize_seq(V)
    }
}

// ── Genesis Container (Level 0 — Seal) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisContainer {
    pub id: String,
    pub name: String,
    /// Free-form world type label ("2D", "3D", "Audio", "Screenplay", etc.)
    pub world_type: String,
    pub domain: Domain,
    pub seal_type: SealType,
    pub lifecycle: Lifecycle,
    pub mythos: Vec<MythosContainer>,
    pub capacity: CapacityMetadata,
    pub sealed_at: Option<u64>,
    pub bdna: BDnaRecord,
    pub resonance_hz: f64,
    /// Harmonic ratio to parent seal (required for Lesser Seals).
    pub harmonic_ratio: Option<f64>,
    pub parent_seal_id: Option<String>,
    pub bound_elements: Vec<String>,
}

// ── Mythos Container (Level 1 — Crest) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythosContainer {
    pub id: String,
    pub name: String,
    pub parent_genesis_id: String,
    pub crest: CrestName,
    pub containers: Vec<Container>,
    pub lifecycle: Lifecycle,
    pub capacity: CapacityMetadata,
    pub bdna: BDnaRecord,
    pub resonance_hz: f64,
    pub bound_elements: Vec<String>,
}

// ── Container (Level 2 — Glyph / Device / Emblem) ──────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub parent_mythos_id: String,
    pub capsules: Vec<Capsule>,
    pub heraldry: ContainerHeraldry,
    pub lifecycle: Lifecycle,
    pub capacity: CapacityMetadata,
    pub bdna: BDnaRecord,
    /// Primary wire type output for this container's capsules.
    pub wire_out: Option<WireType>,
    pub resonance_hz: f64,
    pub bound_elements: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerHeraldry {
    Glyph,
    Device,
    Emblem,
}

impl ContainerHeraldry {
    pub fn to_symbolic_type(&self) -> SymbolicType {
        match self {
            ContainerHeraldry::Glyph => SymbolicType::Glyph,
            ContainerHeraldry::Device => SymbolicType::Device,
            ContainerHeraldry::Emblem => SymbolicType::Emblem,
        }
    }
}

// ── Capsule (Level 3 — Atomic Unit) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capsule {
    pub id: String,
    pub name: String,
    pub parent_container_id: String,
    /// Symbolic type at birth — immutable.
    pub heraldic_birth: CapsuleHeraldry,
    /// Symbolic type after ascent — may differ from heraldic_birth.
    pub heraldic_current: CapsuleHeraldry,
    pub wire_type: WireType,
    pub bdna: BDnaRecord,
    /// 64-bit binary DNA strand — compile-time guaranteed length.
    #[serde(with = "bdna_bits_serde")]
    pub bdna_bits: [bool; 64],
    /// Required for sealed Genesis Containers.
    pub lineage_hash: String,
    pub payload: serde_json::Value,
    pub ports: Vec<WirePort>,
    pub tags: Vec<String>,
    pub created_at: u64,
    pub resonance_hz: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapsuleHeraldry {
    Trait,
    Mark,
    Token,
    Sigil,
}

impl CapsuleHeraldry {
    pub fn to_symbolic_type(&self) -> SymbolicType {
        match self {
            CapsuleHeraldry::Trait => SymbolicType::Trait,
            CapsuleHeraldry::Mark => SymbolicType::Mark,
            CapsuleHeraldry::Token => SymbolicType::Token,
            CapsuleHeraldry::Sigil => SymbolicType::Sigil,
        }
    }

    /// Heraldry ascent order: Mark → Trait → Token → Sigil
    pub fn ascent_rank(&self) -> u8 {
        match self {
            CapsuleHeraldry::Mark => 0,
            CapsuleHeraldry::Trait => 1,
            CapsuleHeraldry::Token => 2,
            CapsuleHeraldry::Sigil => 3,
        }
    }

    pub fn can_ascend_to(&self, target: &CapsuleHeraldry) -> bool {
        target.ascent_rank() > self.ascent_rank()
    }
}

// ── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ContainerError {
    Capacity(CapacityError),
    Heraldry(HeraldryError),
    Alignment(Vec<crate::alignment::AlignmentError>),
    BDna(BDnaError),
    Lifecycle(String),
}

impl fmt::Display for ContainerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerError::Capacity(e) => write!(f, "Capacity: {e}"),
            ContainerError::Heraldry(e) => write!(f, "Heraldry: {e}"),
            ContainerError::Alignment(errors) => {
                write!(f, "Alignment errors: ")?;
                for e in errors {
                    write!(f, "{e}; ")?;
                }
                Ok(())
            }
            ContainerError::BDna(e) => write!(f, "B-DNA: {e}"),
            ContainerError::Lifecycle(msg) => write!(f, "Lifecycle: {msg}"),
        }
    }
}

impl std::error::Error for ContainerError {}

// ── GenesisContainer impl ────────────────────────────────────────────────────

impl GenesisContainer {
    pub fn new(name: String, domain: Domain, created_by: Option<String>) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let bdna = BDnaRecord::new_original(&name, created_by);
        Self {
            id,
            name,
            world_type: String::new(),
            domain,
            seal_type: SealType::Greater,
            lifecycle: Lifecycle::Seeding,
            mythos: Vec::new(),
            capacity: CapacityMetadata::default_fixed(),
            sealed_at: None,
            bdna,
            resonance_hz: 440.0,
            harmonic_ratio: None,
            parent_seal_id: None,
            bound_elements: Vec::new(),
        }
    }

    pub fn add_mythos(
        &mut self,
        name: String,
        crest: CrestName,
        created_by: Option<String>,
    ) -> Result<&MythosContainer, ContainerError> {
        if !self.lifecycle.is_mutable() {
            return Err(ContainerError::Capacity(CapacityError::SealedContainer {
                id: self.id.clone(),
            }));
        }
        capacity::check_capacity(self.mythos.len(), ContainerLevel::Genesis)
            .map_err(ContainerError::Capacity)?;

        let id = uuid::Uuid::new_v4().to_string();
        let bdna = BDnaRecord::new_original(&name, created_by);
        let mythos = MythosContainer {
            id,
            name,
            parent_genesis_id: self.id.clone(),
            crest,
            containers: Vec::new(),
            lifecycle: Lifecycle::Seeding,
            capacity: CapacityMetadata::default_fixed(),
            bdna,
            resonance_hz: 440.0,
            bound_elements: Vec::new(),
        };
        self.mythos.push(mythos);
        self.capacity.child_count = self.mythos.len();
        Ok(self.mythos.last().unwrap())
    }

    pub fn seal(&mut self) -> Result<(), ContainerError> {
        if !self.lifecycle.can_seal() {
            return Err(ContainerError::Lifecycle(format!(
                "Cannot seal from {} state (must be Active)",
                self.lifecycle
            )));
        }
        self.lifecycle = Lifecycle::Sealed;
        self.bdna.sealed = true;
        self.capacity.sealed_octave = Some(self.capacity.current_octave);
        self.sealed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
        Ok(())
    }

    pub fn activate(&mut self) -> Result<(), ContainerError> {
        if self.lifecycle != Lifecycle::Seeding {
            return Err(ContainerError::Lifecycle(format!(
                "Cannot activate from {} state (must be Seeding)",
                self.lifecycle
            )));
        }
        self.lifecycle = Lifecycle::Active;
        Ok(())
    }

    pub fn symbolic_type(&self) -> SymbolicType {
        match self.seal_type {
            SealType::Greater => SymbolicType::GreaterSeal,
            SealType::Lesser => SymbolicType::LesserSeal,
        }
    }

    pub fn validate(&self) -> Vec<ContainerError> {
        let mut errors = Vec::new();

        let alignment = ThreeWayAlignment {
            structural: ContainerLevel::Genesis,
            functional: FunctionalRole::Engine,
            symbolic: self.symbolic_type(),
        };
        if let Err(e) = validate_alignment(&alignment) {
            errors.push(ContainerError::Alignment(e));
        }

        if self.mythos.len() > MAX_CHILDREN {
            errors.push(ContainerError::Capacity(CapacityError::Overflow {
                level: ContainerLevel::Genesis,
                current_count: self.mythos.len(),
            }));
        }

        for mythos in &self.mythos {
            errors.extend(mythos.validate());
        }

        errors
    }

    pub fn total_capsule_count(&self) -> usize {
        self.mythos.iter().map(|m| m.total_capsule_count()).sum()
    }
}

// ── MythosContainer impl ─────────────────────────────────────────────────────

impl MythosContainer {
    pub fn add_container(
        &mut self,
        name: String,
        heraldry: ContainerHeraldry,
        created_by: Option<String>,
    ) -> Result<&Container, ContainerError> {
        if !self.lifecycle.is_mutable() {
            return Err(ContainerError::Capacity(CapacityError::SealedContainer {
                id: self.id.clone(),
            }));
        }
        capacity::check_capacity(self.containers.len(), ContainerLevel::Mythos)
            .map_err(ContainerError::Capacity)?;

        validate_heraldry(ContainerLevel::Container, &heraldry.to_symbolic_type())
            .map_err(ContainerError::Heraldry)?;

        let id = uuid::Uuid::new_v4().to_string();
        let bdna = BDnaRecord::new_original(&name, created_by);
        let container = Container {
            id,
            name,
            parent_mythos_id: self.id.clone(),
            capsules: Vec::new(),
            heraldry,
            lifecycle: Lifecycle::Seeding,
            capacity: CapacityMetadata::default_fixed(),
            bdna,
            wire_out: None,
            resonance_hz: 440.0,
            bound_elements: Vec::new(),
        };
        self.containers.push(container);
        self.capacity.child_count = self.containers.len();
        Ok(self.containers.last().unwrap())
    }

    pub fn validate(&self) -> Vec<ContainerError> {
        let mut errors = Vec::new();

        let alignment = ThreeWayAlignment {
            structural: ContainerLevel::Mythos,
            functional: FunctionalRole::MajorSystem,
            symbolic: SymbolicType::Crest(self.crest.clone()),
        };
        if let Err(e) = validate_alignment(&alignment) {
            errors.push(ContainerError::Alignment(e));
        }

        if self.containers.len() > MAX_CHILDREN {
            errors.push(ContainerError::Capacity(CapacityError::Overflow {
                level: ContainerLevel::Mythos,
                current_count: self.containers.len(),
            }));
        }

        for container in &self.containers {
            errors.extend(container.validate());
        }

        errors
    }

    pub fn total_capsule_count(&self) -> usize {
        self.containers.iter().map(|c| c.capsules.len()).sum()
    }
}

// ── Container impl ───────────────────────────────────────────────────────────

impl Container {
    pub fn add_capsule(
        &mut self,
        name: String,
        heraldry: CapsuleHeraldry,
        wire_type: WireType,
        payload: serde_json::Value,
        created_by: Option<String>,
    ) -> Result<&Capsule, ContainerError> {
        if !self.lifecycle.is_mutable() {
            return Err(ContainerError::Capacity(CapacityError::SealedContainer {
                id: self.id.clone(),
            }));
        }
        capacity::check_capacity(self.capsules.len(), ContainerLevel::Container)
            .map_err(ContainerError::Capacity)?;

        validate_heraldry(ContainerLevel::Capsule, &heraldry.to_symbolic_type())
            .map_err(ContainerError::Heraldry)?;

        let id = format!("0x{:016X}", uuid::Uuid::new_v4().as_u128() as u64);
        let lineage_hash = format!("0x{:016X}", uuid::Uuid::new_v4().as_u128() as u64);
        let bdna = BDnaRecord::new_original(&name, created_by);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let capsule = Capsule {
            id,
            name: name.clone(),
            parent_container_id: self.id.clone(),
            heraldic_birth: heraldry.clone(),
            heraldic_current: heraldry,
            wire_type,
            bdna,
            bdna_bits: [false; 64],
            lineage_hash,
            payload,
            ports: Vec::new(),
            tags: vec![wire_type.abbreviation().to_lowercase()],
            created_at: now,
            resonance_hz: 440.0,
        };
        self.capsules.push(capsule);
        self.capacity.child_count = self.capsules.len();
        Ok(self.capsules.last().unwrap())
    }

    pub fn validate(&self) -> Vec<ContainerError> {
        let mut errors = Vec::new();

        let alignment = ThreeWayAlignment {
            structural: ContainerLevel::Container,
            functional: FunctionalRole::Addon,
            symbolic: self.heraldry.to_symbolic_type(),
        };
        if let Err(e) = validate_alignment(&alignment) {
            errors.push(ContainerError::Alignment(e));
        }

        if self.capsules.len() > MAX_CHILDREN {
            errors.push(ContainerError::Capacity(CapacityError::Overflow {
                level: ContainerLevel::Container,
                current_count: self.capsules.len(),
            }));
        }

        for capsule in &self.capsules {
            errors.extend(capsule.validate());
        }

        errors
    }
}

// ── Capsule impl ─────────────────────────────────────────────────────────────

impl Capsule {
    pub fn ascend_to(&mut self, target: CapsuleHeraldry) -> Result<(), ContainerError> {
        if !self.heraldic_current.can_ascend_to(&target) {
            return Err(ContainerError::Heraldry(
                crate::heraldry::HeraldryError::LevelMismatch {
                    expected_level: ContainerLevel::Capsule,
                    symbolic_type: format!("{:?}", target),
                    symbolic_level: ContainerLevel::Capsule,
                },
            ));
        }
        self.heraldic_current = target;
        Ok(())
    }

    pub fn validate(&self) -> Vec<ContainerError> {
        let mut errors = Vec::new();

        let alignment = ThreeWayAlignment {
            structural: ContainerLevel::Capsule,
            functional: FunctionalRole::Entity,
            symbolic: self.heraldic_current.to_symbolic_type(),
        };
        if let Err(e) = validate_alignment(&alignment) {
            errors.push(ContainerError::Alignment(e));
        }

        if self.bdna.signature.is_empty() {
            errors.push(ContainerError::BDna(BDnaError::MissingProvenance {
                entity_name: self.name.clone(),
            }));
        }

        if self.lineage_hash.is_empty() {
            errors.push(ContainerError::BDna(BDnaError::MissingProvenance {
                entity_name: format!("{} (lineage_hash)", self.name),
            }));
        }

        errors
    }
}

// ── Serialization helpers ────────────────────────────────────────────────────

pub const QGENESIS_EXTENSION: &str = "qgenesis";
pub const QGCP_EXTENSION: &str = "qgcp";

pub fn serialize_qgenesis(genesis: &GenesisContainer) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec_pretty(genesis)
}

pub fn deserialize_qgenesis(data: &[u8]) -> Result<GenesisContainer, serde_json::Error> {
    serde_json::from_slice(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_genesis_container() {
        let genesis = GenesisContainer::new(
            "Test World".into(),
            Domain::Narrative,
            Some("system".into()),
        );
        assert_eq!(genesis.lifecycle, Lifecycle::Seeding);
        assert_eq!(genesis.seal_type, SealType::Greater);
        assert!(genesis.mythos.is_empty());
        assert_eq!(genesis.total_capsule_count(), 0);
        assert_eq!(genesis.resonance_hz, 440.0);
    }

    #[test]
    fn add_mythos_to_genesis() {
        let mut genesis = GenesisContainer::new("World".into(), Domain::Agent, None);
        let result = genesis.add_mythos("Vault Crest".into(), CrestName::Vault, None);
        assert!(result.is_ok());
        assert_eq!(genesis.mythos.len(), 1);
        assert_eq!(genesis.mythos[0].crest, CrestName::Vault);
    }

    #[test]
    fn capacity_enforced_at_genesis() {
        let mut genesis = GenesisContainer::new("Full World".into(), Domain::Narrative, None);
        for i in 0..16 {
            assert!(genesis
                .add_mythos(format!("Mythos {i}"), CrestName::Core, None)
                .is_ok());
        }
        assert!(genesis
            .add_mythos("Overflow".into(), CrestName::Core, None)
            .is_err());
    }

    #[test]
    fn full_hierarchy() {
        let mut genesis = GenesisContainer::new("World".into(), Domain::Agent, None);
        genesis
            .add_mythos("Systems".into(), CrestName::Core, None)
            .unwrap();

        genesis.mythos[0]
            .add_container("Features".into(), ContainerHeraldry::Device, None)
            .unwrap();

        genesis.mythos[0].containers[0]
            .add_capsule(
                "Actor Alpha".into(),
                CapsuleHeraldry::Sigil,
                WireType::Identity,
                serde_json::json!({"species": "human"}),
                None,
            )
            .unwrap();

        assert_eq!(genesis.total_capsule_count(), 1);

        let errors = genesis.validate();
        assert!(errors.is_empty(), "Validation errors: {errors:?}");
    }

    #[test]
    fn capsule_has_lineage_hash() {
        let mut genesis = GenesisContainer::new("World".into(), Domain::Agent, None);
        genesis.add_mythos("M".into(), CrestName::Core, None).unwrap();
        genesis.mythos[0].add_container("C".into(), ContainerHeraldry::Device, None).unwrap();
        genesis.mythos[0].containers[0]
            .add_capsule("Cap".into(), CapsuleHeraldry::Trait, WireType::Data, serde_json::Value::Null, None)
            .unwrap();

        let cap = &genesis.mythos[0].containers[0].capsules[0];
        assert!(!cap.lineage_hash.is_empty());
        assert_eq!(cap.bdna_bits.len(), 64);
        assert_eq!(cap.heraldic_birth, CapsuleHeraldry::Trait);
        assert_eq!(cap.heraldic_current, CapsuleHeraldry::Trait);
    }

    #[test]
    fn capsule_heraldry_ascent() {
        let mut cap = Capsule {
            id: "0x0000000000000001".into(),
            name: "Test".into(),
            parent_container_id: "c1".into(),
            heraldic_birth: CapsuleHeraldry::Mark,
            heraldic_current: CapsuleHeraldry::Mark,
            wire_type: WireType::Data,
            bdna: BDnaRecord::new_original("Test", None),
            bdna_bits: [false; 64],
            lineage_hash: "0x0000000000000002".into(),
            payload: serde_json::Value::Null,
            ports: vec![],
            tags: vec![],
            created_at: 0,
            resonance_hz: 440.0,
        };
        assert!(cap.ascend_to(CapsuleHeraldry::Sigil).is_ok());
        assert_eq!(cap.heraldic_current, CapsuleHeraldry::Sigil);
        assert_eq!(cap.heraldic_birth, CapsuleHeraldry::Mark);
        // Cannot descend
        assert!(cap.ascend_to(CapsuleHeraldry::Mark).is_err());
    }

    #[test]
    fn serialization_roundtrip() {
        let mut genesis = GenesisContainer::new("Roundtrip".into(), Domain::Music, None);
        genesis
            .add_mythos("Album Series".into(), CrestName::Composer, None)
            .unwrap();

        let data = serialize_qgenesis(&genesis).unwrap();
        let restored = deserialize_qgenesis(&data).unwrap();
        assert_eq!(restored.name, "Roundtrip");
        assert_eq!(restored.mythos.len(), 1);
        assert_eq!(restored.mythos[0].crest, CrestName::Composer);
    }

    #[test]
    fn sealed_container_rejects_mutations() {
        let mut genesis = GenesisContainer::new("Sealable".into(), Domain::Narrative, None);
        genesis.activate().unwrap();
        genesis.seal().unwrap();
        assert!(genesis
            .add_mythos("New".into(), CrestName::Core, None)
            .is_err());
    }

    #[test]
    fn lifecycle_transitions() {
        let mut genesis = GenesisContainer::new("Lifecycle".into(), Domain::Software, None);
        assert_eq!(genesis.lifecycle, Lifecycle::Seeding);
        assert!(genesis.seal().is_err());
        genesis.activate().unwrap();
        assert_eq!(genesis.lifecycle, Lifecycle::Active);
        genesis.seal().unwrap();
        assert_eq!(genesis.lifecycle, Lifecycle::Sealed);
        assert!(genesis.bdna.sealed);
        assert!(genesis.capacity.sealed_octave.is_some());
    }
}
