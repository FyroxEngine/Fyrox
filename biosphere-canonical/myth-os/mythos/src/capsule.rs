use crate::identity::{Blake3Hash, MythId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The universal data container transported between Vault, Core, and Genesis.
/// Every asset — actor, mesh, audio, script — is a Capsule at rest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capsule {
    pub id: MythId,
    pub fingerprint: Option<Blake3Hash>,
    pub kind: CapsuleKind,
    pub schema_version: u32,
    pub created_at: DateTime<Utc>,
    pub payload: Vec<u8>,
    pub metadata: CapsuleMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CapsuleKind {
    Actor,
    Terrain,
    Audio,
    Script,
    Blueprint,
    Signal,
    /// A sealed Vault — a Capsule at the Genesis hierarchy level.
    /// Every Vault IS a Capsule; this kind makes that relationship explicit.
    Vault,
    Raw,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapsuleMeta {
    pub name: String,
    pub tags: Vec<String>,
    pub origin_vault: Option<MythId>,
    pub parent: Option<MythId>,
}

impl Capsule {
    pub fn new(kind: CapsuleKind, payload: Vec<u8>, name: impl Into<String>) -> Self {
        Self {
            id: MythId::new(),
            fingerprint: None,
            kind,
            schema_version: 1,
            created_at: Utc::now(),
            payload,
            metadata: CapsuleMeta {
                name: name.into(),
                ..Default::default()
            },
        }
    }

    pub fn byte_len(&self) -> usize {
        self.payload.len()
    }
}
