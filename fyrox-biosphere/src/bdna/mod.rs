use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BDnaRecord {
    pub signature: String,
    pub generation: u32,
    pub parent_ids: Vec<String>,
    pub root_id: Option<String>,
    pub covenant: Option<Covenant>,
    pub metadata: BDnaMetadata,
    pub sealed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BDnaMetadata {
    pub forked_from: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Covenant {
    pub bindings: Vec<CovenantBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CovenantBinding {
    pub scope: CovenantScope,
    pub enforceable: bool,
    pub rule: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CovenantScope {
    Local,
    Inherited,
    CrossContext,
}

impl fmt::Display for CovenantScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CovenantScope::Local => write!(f, "Local"),
            CovenantScope::Inherited => write!(f, "Inherited"),
            CovenantScope::CrossContext => write!(f, "Cross-Context"),
        }
    }
}

impl BDnaRecord {
    pub fn generate_signature(name: &str, parent_ids: &[String], generation: u32) -> String {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        for pid in parent_ids {
            hasher.update(pid.as_bytes());
        }
        hasher.update(generation.to_le_bytes());
        hasher.update(uuid::Uuid::new_v4().as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn new_original(name: &str, created_by: Option<String>) -> Self {
        let signature = Self::generate_signature(name, &[], 0);
        Self {
            signature,
            generation: 0,
            parent_ids: Vec::new(),
            root_id: None,
            covenant: None,
            metadata: BDnaMetadata {
                forked_from: None,
                created_by,
            },
            sealed: false,
        }
    }

    pub fn fork(&self, name: &str, created_by: Option<String>) -> Self {
        let new_generation = self.generation + 1;
        let parent_ids = vec![self.signature.clone()];
        let root_id = self.root_id.clone().unwrap_or_else(|| self.signature.clone());
        let signature = Self::generate_signature(name, &parent_ids, new_generation);

        let inherited_bindings: Vec<CovenantBinding> = self
            .covenant
            .as_ref()
            .map(|c| {
                c.bindings
                    .iter()
                    .filter(|b| matches!(b.scope, CovenantScope::Inherited | CovenantScope::CrossContext))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        let covenant = if inherited_bindings.is_empty() {
            None
        } else {
            Some(Covenant {
                bindings: inherited_bindings,
            })
        };

        Self {
            signature,
            generation: new_generation,
            parent_ids,
            root_id: Some(root_id),
            covenant,
            metadata: BDnaMetadata {
                forked_from: Some(self.signature.clone()),
                created_by,
            },
            sealed: false,
        }
    }

    pub fn add_covenant_binding(&mut self, binding: CovenantBinding) {
        let covenant = self.covenant.get_or_insert(Covenant {
            bindings: Vec::new(),
        });
        covenant.bindings.push(binding);
    }

    pub fn inherited_bindings(&self) -> Vec<&CovenantBinding> {
        self.covenant
            .as_ref()
            .map(|c| {
                c.bindings
                    .iter()
                    .filter(|b| matches!(b.scope, CovenantScope::Inherited | CovenantScope::CrossContext))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn enforceable_bindings(&self) -> Vec<&CovenantBinding> {
        self.covenant
            .as_ref()
            .map(|c| c.bindings.iter().filter(|b| b.enforceable).collect())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BDnaError {
    MissingProvenance { entity_name: String },
    SealedMutation { signature: String },
}

impl fmt::Display for BDnaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BDnaError::MissingProvenance { entity_name } => {
                write!(
                    f,
                    "Entity '{entity_name}' has no B-DNA — nothing without provenance \
                     enters a sealed Genesis"
                )
            }
            BDnaError::SealedMutation { signature } => {
                write!(f, "Cannot mutate sealed B-DNA record '{signature}'")
            }
        }
    }
}

impl std::error::Error for BDnaError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_entity() {
        let bdna = BDnaRecord::new_original("TestActor", Some("system".into()));
        assert_eq!(bdna.generation, 0);
        assert!(bdna.parent_ids.is_empty());
        assert!(bdna.root_id.is_none());
        assert!(!bdna.signature.is_empty());
        assert!(!bdna.sealed);
    }

    #[test]
    fn forking_increments_generation() {
        let original = BDnaRecord::new_original("Parent", None);
        let fork = original.fork("Child", None);
        assert_eq!(fork.generation, 1);
        assert_eq!(fork.parent_ids, vec![original.signature.clone()]);
        assert_eq!(fork.root_id, Some(original.signature.clone()));
    }

    #[test]
    fn covenant_inheritance_on_fork() {
        let mut original = BDnaRecord::new_original("Bound", None);
        original.add_covenant_binding(CovenantBinding {
            scope: CovenantScope::Local,
            enforceable: true,
            rule: "Local only rule".into(),
        });
        original.add_covenant_binding(CovenantBinding {
            scope: CovenantScope::Inherited,
            enforceable: true,
            rule: "Cannot betray the Crown".into(),
        });
        original.add_covenant_binding(CovenantBinding {
            scope: CovenantScope::CrossContext,
            enforceable: false,
            rule: "Narrative: always seek truth".into(),
        });

        let fork = original.fork("Offspring", None);
        let covenant = fork.covenant.unwrap();
        assert_eq!(covenant.bindings.len(), 2);
        assert!(covenant.bindings.iter().any(|b| b.rule == "Cannot betray the Crown"));
        assert!(covenant.bindings.iter().any(|b| b.rule == "Narrative: always seek truth"));
        assert!(!covenant.bindings.iter().any(|b| b.rule == "Local only rule"));
    }

    #[test]
    fn unique_signatures() {
        let a = BDnaRecord::new_original("A", None);
        let b = BDnaRecord::new_original("B", None);
        assert_ne!(a.signature, b.signature);
    }

    #[test]
    fn multi_generation_fork() {
        let gen0 = BDnaRecord::new_original("Gen0", None);
        let gen1 = gen0.fork("Gen1", None);
        let gen2 = gen1.fork("Gen2", None);
        assert_eq!(gen2.generation, 2);
        assert_eq!(gen2.parent_ids, vec![gen1.signature.clone()]);
        assert_eq!(gen2.root_id, Some(gen0.signature.clone()));
    }
}
