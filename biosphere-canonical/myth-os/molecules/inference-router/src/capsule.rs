/// What the downstream ATOM needs back from inference.
/// This is the type-check the router uses to pick a capable backend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum OutputType {
    /// Plain prose — dialogue, lore, descriptions. Any backend works.
    NarrativeText,
    /// A GLSL or WGSL shader fragment. Needs a code-capable model.
    ShaderFragment,
    /// A JSON blob — structured world data, entity state, quest params.
    StructuredJson,
    /// A myth-os MOLECULE definition (ATOM wiring spec in JSON).
    MoleculeSpec,
    /// A dialogue tree in myth-quill format.
    DialogueTree,
    /// A raw decision — yes/no or an enum variant. Cheapest path.
    Decision,
}

/// The Capsule that flows INTO an LLM ATOM.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InferenceCapsule {
    /// What kind of output this Capsule needs.
    pub output_type: OutputType,
    /// The prompt or context to send to the model.
    pub prompt: String,
    /// Optional system/role context (world lore, persona, constraints).
    pub system: Option<String>,
    /// Hard token ceiling. None = backend default.
    pub max_tokens: Option<u32>,
    /// Temperature 0.0–1.0. None = backend default.
    pub temperature: Option<f32>,
}

/// The Capsule that flows OUT of an LLM ATOM.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InferenceOutput {
    /// Which backend actually handled this request.
    pub backend: BackendId,
    pub output_type: OutputType,
    /// Raw text from the model. Downstream ATOMs parse to their expected type.
    pub content: String,
    /// Tokens consumed — used by resource-management ATOMs.
    pub tokens_used: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum BackendId {
    Ollama,
    Claude,
    Gemini,
    OpenAI,
}

impl std::fmt::Display for BackendId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ollama  => write!(f, "Ollama"),
            Self::Claude  => write!(f, "Claude"),
            Self::Gemini  => write!(f, "Gemini"),
            Self::OpenAI  => write!(f, "OpenAI"),
        }
    }
}
