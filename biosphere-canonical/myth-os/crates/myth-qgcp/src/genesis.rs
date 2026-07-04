use serde::{Deserialize, Serialize};
use crate::{MythosModule, QgcpError, SealBlock, MAX_MYTHOS, TOTAL_CAPACITY};

/// The headless world-state Genesis Container.
///
/// WorldGenesis is the "CD" — pure simulation data with no media assets.
/// It is used by the headless server AND by client-side agents. Neither
/// needs audio samples or GLB references to reason about the world.
///
/// For the media layer (audio, images, models) see `MediaGenesis`.
/// For agent/character containers see `ActorGenesis`.
///
/// Lifecycle:  draft → active → sealed
/// Once sealed, no mutations are permitted. The seal carries a lineage hash
/// over the entire container so any drift is cryptographically detectable.
///
/// File extension: `.worldgenesis`
/// Short alias:    `WorldGen`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldGenesis {
    /// Unique ID for this genesis instance.
    pub genesis_id: String,

    /// Reverse-domain package ID e.g. `pkg.biospark.xyrona-prime`
    pub package_id: String,

    pub name: String,

    /// Domain classifier e.g. "world", "agent-memory", "project", "song"
    pub domain: String,

    pub description: Option<String>,

    /// `draft` | `active` | `sealed`
    pub lifecycle: String,

    pub sealed: bool,

    pub mythos: Vec<MythosModule>,

    /// Set when lifecycle reaches "sealed".
    pub seal: Option<SealBlock>,

    /// Unix timestamp of creation.
    pub created_at: i64,

    /// Schema version for forward-compatibility.
    pub schema_version: String,
}

impl WorldGenesis {
    pub fn new(
        package_id: impl Into<String>,
        name: impl Into<String>,
        domain: impl Into<String>,
    ) -> Self {
        Self {
            genesis_id: format!("world_{}", uuid::Uuid::new_v4().simple()),
            package_id: package_id.into(),
            name: name.into(),
            domain: domain.into(),
            description: None,
            lifecycle: "draft".into(),
            sealed: false,
            mythos: Vec::new(),
            seal: None,
            created_at: chrono::Utc::now().timestamp(),
            schema_version: "qgcp-v1.0".into(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn add_mythos(&mut self, module: MythosModule) -> Result<(), QgcpError> {
        if self.sealed {
            return Err(QgcpError::Sealed);
        }
        if self.mythos.len() >= MAX_MYTHOS {
            return Err(QgcpError::MythosOverflow(self.mythos.len() + 1));
        }
        self.mythos.push(module);
        Ok(())
    }

    /// Total number of capsules across all mythos and containers.
    pub fn capsule_count(&self) -> usize {
        self.mythos.iter()
            .flat_map(|m| &m.containers)
            .map(|c| c.capsules.len())
            .sum()
    }

    /// Remaining capsule capacity before hitting the 4,096 limit.
    pub fn remaining_capacity(&self) -> usize {
        TOTAL_CAPACITY.saturating_sub(self.capsule_count())
    }

    /// Validate the 16x16x16 capacity law across the entire hierarchy.
    pub fn validate_capacity(&self) -> Result<(), QgcpError> {
        if self.mythos.len() > MAX_MYTHOS {
            return Err(QgcpError::MythosOverflow(self.mythos.len()));
        }
        for m in &self.mythos {
            if m.containers.len() > crate::MAX_CONTAINERS {
                return Err(QgcpError::ContainerOverflow(m.containers.len()));
            }
            for c in &m.containers {
                if c.capsules.len() > crate::MAX_CAPSULES {
                    return Err(QgcpError::CapsuleOverflow(c.capsules.len()));
                }
            }
        }
        Ok(())
    }

    /// Seal the container. Computes a lineage hash over the current content
    /// and locks it. Any subsequent mutation attempts return QgcpError::Sealed.
    pub fn seal(&mut self, sealed_by: impl Into<String>) -> Result<&SealBlock, QgcpError> {
        if self.sealed {
            return Err(QgcpError::Sealed);
        }
        self.validate_capacity()?;
        if !self.verify_integrity() {
            return Err(QgcpError::LineageMismatch {
                expected: "all capsules valid".into(),
                actual: "one or more capsules failed integrity check".into(),
            });
        }

        // Hash the serialized content before adding the seal.
        let content = serde_json::to_string(&self.mythos)?;
        let hash = blake3::hash(content.as_bytes());
        let lineage_hash = hex::encode(hash.as_bytes());

        self.lifecycle = "sealed".into();
        self.sealed = true;
        self.seal = Some(SealBlock::new(lineage_hash, sealed_by));

        Ok(self.seal.as_ref().unwrap())
    }

    /// Verify the seal is still valid (content hasn't changed since sealing).
    pub fn verify_seal(&self) -> bool {
        let Some(ref seal) = self.seal else { return false };
        let Ok(content) = serde_json::to_string(&self.mythos) else { return false };
        let hash = blake3::hash(content.as_bytes());
        let current = hex::encode(hash.as_bytes());
        current == seal.lineage_hash
    }

    /// Verify all capsule lineage hashes are intact.
    pub fn verify_integrity(&self) -> bool {
        self.mythos.iter().all(|m| m.verify_integrity())
    }

    /// Serialize to pretty JSON.
    pub fn to_json(&self) -> Result<String, QgcpError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, QgcpError> {
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use myth_wire::WireType;
    use crate::{Capsule, Container, MythosModule};

    fn make_genesis() -> WorldGenesis {
        WorldGenesis::new("pkg.test", "Test World", "world")
    }

    #[test]
    fn starts_unsealed() {
        let g = make_genesis();
        assert!(!g.sealed);
        assert_eq!(g.lifecycle, "draft");
        assert_eq!(g.capsule_count(), 0);
    }

    #[test]
    fn genesis_id_prefixed_world() {
        let g = make_genesis();
        assert!(g.genesis_id.starts_with("world_"));
    }

    #[test]
    fn capacity_law_enforced() {
        let mut g = make_genesis();
        for i in 0..=MAX_MYTHOS {
            let m = MythosModule::new(format!("MYTH-{:02}", i), format!("Module {}", i), WireType::Data);
            if i < MAX_MYTHOS {
                assert!(g.add_mythos(m).is_ok());
            } else {
                assert!(matches!(g.add_mythos(m), Err(QgcpError::MythosOverflow(_))));
            }
        }
    }

    #[test]
    fn seal_and_verify() {
        let mut g = make_genesis();
        let mut m = MythosModule::new("MYTH-01", "Lore", WireType::Narrative);
        let mut c = Container::new("CONT-01-01", "Origins", WireType::Narrative);
        let cap = Capsule::new(
            "Creation Myth",
            WireType::Narrative,
            serde_json::json!({ "text": "In the beginning..." }),
            vec!["lore".into()],
            None,
        );
        c.add_capsule(cap).unwrap();
        m.add_container(c).unwrap();
        g.add_mythos(m).unwrap();

        g.seal("local-architect").unwrap();
        assert!(g.sealed);
        assert!(g.verify_seal());
        assert_eq!(g.capsule_count(), 1);
    }

    #[test]
    fn mutation_after_seal_fails() {
        let mut g = make_genesis();
        g.seal("local-architect").unwrap();
        let m = MythosModule::new("MYTH-01", "Late Addition", WireType::Data);
        assert!(matches!(g.add_mythos(m), Err(QgcpError::Sealed)));
    }

    #[test]
    fn json_round_trip() {
        let g = make_genesis();
        let json = g.to_json().unwrap();
        let back = WorldGenesis::from_json(&json).unwrap();
        assert_eq!(g.genesis_id, back.genesis_id);
        assert_eq!(g.name, back.name);
    }
}
