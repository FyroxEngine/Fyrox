use crate::{
    alignment::{FunctionalRole, ThreeWayAlignment, validate_alignment},
    bdna::{BDnaRecord, BDnaError},
    capacity::{
        self, CapacityError, ContainerLevel, Lifecycle, MAX_CHILDREN,
    },
    domain::Domain,
    heraldry::{CrestName, HeraldryError, SymbolicType, validate_heraldry},
    wire::WirePort,
};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisContainer {
    pub id: String,
    pub name: String,
    pub domain: Domain,
    pub lifecycle: Lifecycle,
    pub is_lesser_seal: bool,
    pub parent_group_id: Option<String>,
    pub mythos: Vec<MythosContainer>,
    pub sealed_at: Option<u64>,
    pub bdna: BDnaRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythosContainer {
    pub id: String,
    pub name: String,
    pub parent_genesis_id: String,
    pub crest: CrestName,
    pub containers: Vec<Container>,
    pub lifecycle: Lifecycle,
    pub bdna: BDnaRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub parent_mythos_id: String,
    pub capsules: Vec<Capsule>,
    pub heraldry: ContainerHeraldry,
    pub lifecycle: Lifecycle,
    pub bdna: BDnaRecord,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capsule {
    pub id: String,
    pub name: String,
    pub parent_container_id: String,
    pub heraldry: CapsuleHeraldry,
    pub bdna: BDnaRecord,
    pub payload: serde_json::Value,
    pub ports: Vec<WirePort>,
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
}

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

impl GenesisContainer {
    pub fn new(name: String, domain: Domain, created_by: Option<String>) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let bdna = BDnaRecord::new_original(&name, created_by);
        Self {
            id,
            name,
            domain,
            lifecycle: Lifecycle::Seeding,
            is_lesser_seal: false,
            parent_group_id: None,
            mythos: Vec::new(),
            sealed_at: None,
            bdna,
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
            bdna,
        };
        self.mythos.push(mythos);
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
        if self.is_lesser_seal {
            SymbolicType::LesserSeal
        } else {
            SymbolicType::GreaterSeal
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
            bdna,
        };
        self.containers.push(container);
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

impl Container {
    pub fn add_capsule(
        &mut self,
        name: String,
        heraldry: CapsuleHeraldry,
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

        let id = uuid::Uuid::new_v4().to_string();
        let bdna = BDnaRecord::new_original(&name, created_by);
        let capsule = Capsule {
            id,
            name,
            parent_container_id: self.id.clone(),
            heraldry,
            bdna,
            payload,
            ports: Vec::new(),
        };
        self.capsules.push(capsule);
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

impl Capsule {
    pub fn validate(&self) -> Vec<ContainerError> {
        let mut errors = Vec::new();

        let alignment = ThreeWayAlignment {
            structural: ContainerLevel::Capsule,
            functional: FunctionalRole::Entity,
            symbolic: self.heraldry.to_symbolic_type(),
        };
        if let Err(e) = validate_alignment(&alignment) {
            errors.push(ContainerError::Alignment(e));
        }

        if self.bdna.signature.is_empty() {
            errors.push(ContainerError::BDna(BDnaError::MissingProvenance {
                entity_name: self.name.clone(),
            }));
        }

        errors
    }
}

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
        assert!(!genesis.is_lesser_seal);
        assert!(genesis.mythos.is_empty());
        assert_eq!(genesis.total_capsule_count(), 0);
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
                serde_json::json!({"species": "human"}),
                None,
            )
            .unwrap();

        assert_eq!(genesis.total_capsule_count(), 1);

        let errors = genesis.validate();
        assert!(errors.is_empty(), "Validation errors: {errors:?}");
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
    }
}
