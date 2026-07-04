use serde::{Deserialize, Serialize};
use myth_wire::BDna;
use crate::{ActorContainer, MythosModule, QgcpError, SealBlock, MAX_MYTHOS};

pub const MAX_ACTOR_CONTAINERS: usize = 16;

/// An agent or character Genesis Container.
///
/// ActorGenesis defines a single autonomous actor — their archetype,
/// behavioral drives, faction memberships, memory, and heraldry.
/// It uses the same 16-Mythos hierarchy as WorldGenesis so the same
/// tooling reads and writes both.
///
/// Actors carry a reference to the WorldGenesis they were born in
/// (`home_world_id`). They can be transplanted to other worlds but
/// always know their origin.
///
/// File extension: `.actorgenesis`
/// Short alias:    `ActorGen`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorGenesis {
    pub genesis_id: String,

    /// The WorldGenesis where this actor was created.
    pub home_world_id: String,

    /// Human-readable actor name.
    pub actor_name: String,

    /// Actor archetype identifier — maps to a capsule in the world's
    /// Mythos 05 (Modeling / Animus) that defines the base template.
    pub archetype_id: String,

    /// Deterministic identity derived from name + world + archetype.
    pub bdna_signature: BDna,

    /// The heraldry symbol this actor carries — their routing identity.
    /// Can be a Glyph (if they are a plugin-level agent) or a Brand
    /// (a unique personal mark). Deception mechanics can alter this at runtime.
    pub heraldry_symbol: String,

    /// Intelligence tier (0 = simple reflex, 16 = full reasoning).
    pub intelligence_tier: u8,

    /// Same 16-slot Mythos structure as WorldGenesis.
    /// For actors the slots carry: memory, behavior, drives, relationships,
    /// inventory, skills, narrative arcs, etc.
    pub mythos: Vec<MythosModule>,

    /// Actor capability containers — up to 16 groups of Cells.
    /// Each ActorContainer holds up to 16 Cells (Sigil-routed action units).
    /// This is the CELL layer: what the actor can DO (cast, erupt, pick locks, etc.).
    #[serde(default)]
    pub actor_containers: Vec<ActorContainer>,

    /// `draft` | `active` | `sealed`
    pub lifecycle: String,

    pub sealed: bool,

    pub seal: Option<SealBlock>,

    pub created_at: i64,

    pub schema_version: String,
}

impl ActorGenesis {
    pub fn new(
        home_world_id: impl Into<String>,
        actor_name: impl Into<String>,
        archetype_id: impl Into<String>,
    ) -> Self {
        let name: String = actor_name.into();
        let world: String = home_world_id.into();
        let seed = format!("{}{}", world, name);
        let bdna = BDna::from_seed(seed.as_bytes());

        Self {
            genesis_id: format!("actor_{}", uuid::Uuid::new_v4().simple()),
            home_world_id: world,
            actor_name: name,
            archetype_id: archetype_id.into(),
            bdna_signature: bdna,
            heraldry_symbol: String::new(),
            intelligence_tier: 4,
            mythos: Vec::new(),
            actor_containers: Vec::new(),
            lifecycle: "draft".into(),
            sealed: false,
            seal: None,
            created_at: chrono::Utc::now().timestamp(),
            schema_version: "qgcp-v1.0".into(),
        }
    }

    pub fn with_heraldry(mut self, symbol: impl Into<String>) -> Self {
        self.heraldry_symbol = symbol.into();
        self
    }

    pub fn with_intelligence(mut self, tier: u8) -> Self {
        self.intelligence_tier = tier.min(16);
        self
    }

    pub fn add_actor_container(&mut self, container: ActorContainer) -> Result<(), QgcpError> {
        if self.sealed {
            return Err(QgcpError::Sealed);
        }
        if self.actor_containers.len() >= MAX_ACTOR_CONTAINERS {
            return Err(QgcpError::MythosOverflow(self.actor_containers.len() + 1));
        }
        self.actor_containers.push(container);
        Ok(())
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

    pub fn seal(&mut self, sealed_by: impl Into<String>) -> Result<&SealBlock, QgcpError> {
        if self.sealed {
            return Err(QgcpError::Sealed);
        }
        let content = serde_json::to_string(&self.mythos)?;
        let hash = blake3::hash(content.as_bytes());
        let lineage_hash = hex::encode(hash.as_bytes());

        self.lifecycle = "sealed".into();
        self.sealed = true;
        self.seal = Some(SealBlock::new(lineage_hash, sealed_by));
        Ok(self.seal.as_ref().unwrap())
    }

    pub fn to_json(&self) -> Result<String, QgcpError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(json: &str) -> Result<Self, QgcpError> {
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_genesis_id_prefixed() {
        let a = ActorGenesis::new("world_kasmir", "Synod Mapper 01", "archetype_chroma_seraph");
        assert!(a.genesis_id.starts_with("actor_"));
    }

    #[test]
    fn bdna_deterministic_from_world_and_name() {
        let a1 = ActorGenesis::new("world_kasmir", "Synod Mapper 01", "archetype_chroma_seraph");
        let a2 = ActorGenesis::new("world_kasmir", "Synod Mapper 01", "archetype_chroma_seraph");
        assert_eq!(a1.bdna_signature, a2.bdna_signature);
    }

    #[test]
    fn intelligence_tier_capped_at_16() {
        let a = ActorGenesis::new("world_kasmir", "Elder Mind", "archetype_vitrion").with_intelligence(99);
        assert_eq!(a.intelligence_tier, 16);
    }
}
