pub const CRATE_NAME: &str = "myth-quill";
pub const CREST: &str = "Quill";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum NarrativeVoice { First, Second, Third, Omniscient, Unreliable }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TextFormat { PlainText, Markdown, Json, Html }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DialogueLine {
    pub line_id: String,
    pub speaker_id: String,
    pub text: String,
    pub emotion_hint: Option<String>,
    pub next_line_id: Option<String>,
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DialogueTree {
    pub tree_id: String,
    pub label: String,
    pub entry_line_id: String,
    pub lines: Vec<DialogueLine>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoreEntry {
    pub lore_id: String,
    pub title: String,
    pub body: String,
    pub format: TextFormat,
    pub tags: Vec<String>,
    pub unlocked_by: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuillConfig {
    pub narrative_voice: NarrativeVoice,
    pub default_format: TextFormat,
    pub language_code: String,
    pub lore_entries: Vec<LoreEntry>,
    pub dialogue_trees: Vec<DialogueTree>,
    pub auto_generate_names: bool,
    pub name_seed: u64,
    pub profanity_filter: bool,
    pub max_line_length: u32,
    pub enable_procedural_lore: bool,
}

impl Default for QuillConfig {
    fn default() -> Self {
        Self {
            narrative_voice: NarrativeVoice::Third,
            default_format: TextFormat::Markdown,
            language_code: "en".into(),
            lore_entries: vec![],
            dialogue_trees: vec![],
            auto_generate_names: true,
            name_seed: 0,
            profanity_filter: false,
            max_line_length: 280,
            enable_procedural_lore: false,
        }
    }
}
