/// The four LLM backend ATOMs. Each implements the same interface so the
/// router can call them uniformly. Actual HTTP/FFI calls are stubbed —
/// myth-nexus ExternalSource handles the transport layer.

use crate::capsule::{BackendId, InferenceCapsule, InferenceOutput, OutputType};

pub trait InferenceAtom: Send + Sync {
    fn backend_id(&self) -> BackendId;
    /// Returns true if this backend is reachable and ready.
    fn health_check(&self) -> bool;
    /// Returns true if this backend can produce the requested output type.
    fn supports(&self, output_type: &OutputType) -> bool;
    /// Execute inference. Returns Err if the backend fails.
    fn infer(&self, capsule: &InferenceCapsule) -> Result<InferenceOutput, String>;
}

// ─── Ollama (local-first) ────────────────────────────────────────────────────

pub struct OllamaAtom {
    /// Local Ollama endpoint, default http://localhost:11434
    pub endpoint: String,
    /// Model tag to use, e.g. "llama3", "mistral", "codellama"
    pub model: String,
}

impl Default for OllamaAtom {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".into(),
            model: "llama3".into(),
        }
    }
}

impl InferenceAtom for OllamaAtom {
    fn backend_id(&self) -> BackendId { BackendId::Ollama }

    fn health_check(&self) -> bool {
        // Probe /api/tags — stub returns false until myth-nexus transport wired
        false
    }

    fn supports(&self, _output_type: &OutputType) -> bool {
        // Ollama handles all output types — code models cover ShaderFragment
        true
    }

    fn infer(&self, capsule: &InferenceCapsule) -> Result<InferenceOutput, String> {
        // Real impl: POST /api/generate with model + prompt, stream response
        Err(format!("OllamaAtom: transport stub — wire myth-nexus ExternalSource first. model={} prompt_len={}", self.model, capsule.prompt.len()))
    }
}

// ─── Claude ─────────────────────────────────────────────────────────────────

pub struct ClaudeAtom {
    /// Model ID, e.g. "claude-sonnet-4-6"
    pub model: String,
}

impl Default for ClaudeAtom {
    fn default() -> Self {
        Self { model: "claude-sonnet-4-6".into() }
    }
}

impl InferenceAtom for ClaudeAtom {
    fn backend_id(&self) -> BackendId { BackendId::Claude }

    fn health_check(&self) -> bool {
        // Requires ANTHROPIC_API_KEY in myth-nexus env — stub
        std::env::var("ANTHROPIC_API_KEY").is_ok()
    }

    fn supports(&self, _output_type: &OutputType) -> bool { true }

    fn infer(&self, capsule: &InferenceCapsule) -> Result<InferenceOutput, String> {
        Err(format!("ClaudeAtom: transport stub — myth-nexus API bridge pending. model={}", self.model))
    }
}

// ─── Gemini ──────────────────────────────────────────────────────────────────

pub struct GeminiAtom {
    pub model: String,
}

impl Default for GeminiAtom {
    fn default() -> Self {
        Self { model: "gemini-2.0-flash".into() }
    }
}

impl InferenceAtom for GeminiAtom {
    fn backend_id(&self) -> BackendId { BackendId::Gemini }
    fn health_check(&self) -> bool { std::env::var("GEMINI_API_KEY").is_ok() }
    fn supports(&self, _output_type: &OutputType) -> bool { true }
    fn infer(&self, capsule: &InferenceCapsule) -> Result<InferenceOutput, String> {
        Err(format!("GeminiAtom: transport stub. model={} prompt_len={}", self.model, capsule.prompt.len()))
    }
}

// ─── OpenAI ──────────────────────────────────────────────────────────────────

pub struct OpenAIAtom {
    pub model: String,
}

impl Default for OpenAIAtom {
    fn default() -> Self {
        Self { model: "gpt-4o".into() }
    }
}

impl InferenceAtom for OpenAIAtom {
    fn backend_id(&self) -> BackendId { BackendId::OpenAI }
    fn health_check(&self) -> bool { std::env::var("OPENAI_API_KEY").is_ok() }
    fn supports(&self, _output_type: &OutputType) -> bool { true }
    fn infer(&self, capsule: &InferenceCapsule) -> Result<InferenceOutput, String> {
        Err(format!("OpenAIAtom: transport stub. model={}", self.model))
    }
}
