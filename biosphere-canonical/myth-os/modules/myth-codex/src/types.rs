pub const CRATE_NAME: &str = "myth-codex";
pub const CREST: &str = "Codex";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MemoryType { Episodic, Semantic, Procedural, Emotional, Collective }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryRecord {
    pub memory_id: String,
    pub memory_type: MemoryType,
    pub subject_id: String,
    pub content: serde_json::Value,
    pub emotional_weight: f32,
    pub confidence: f32,
    pub created_at: f64,
    pub last_accessed: f64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeDomain {
    pub domain_id: String,
    pub label: String,
    pub parent_domain: Option<String>,
    pub required_intelligence: u8,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodexConfig {
    pub max_memories_per_actor: u32,
    pub memory_decay_rate: f32,
    pub emotional_decay_modifier: f32,
    pub collective_memory: bool,
    pub knowledge_domains: Vec<KnowledgeDomain>,
    pub history_enabled: bool,
    pub history_max_records: u32,
    pub search_depth: u8,
}

impl Default for CodexConfig {
    fn default() -> Self {
        Self {
            max_memories_per_actor: 256,
            memory_decay_rate: 0.0001,
            emotional_decay_modifier: 0.5,
            collective_memory: true,
            knowledge_domains: vec![],
            history_enabled: true,
            history_max_records: 10000,
            search_depth: 3,
        }
    }
}
