use crate::atoms::{ClaudeAtom, GeminiAtom, InferenceAtom, OllamaAtom, OpenAIAtom};
use crate::capsule::{InferenceCapsule, InferenceOutput};

/// MOLECULE: InferenceRouter
///
/// Pre-wired ATOM graph: OllamaAtom → ClaudeAtom → GeminiAtom → OpenAIAtom
///
/// On each `route()` call it walks the chain, health-checks each ATOM,
/// verifies it supports the requested OutputType, and forwards to the first
/// live match. Falls through to error only if all four are down.
pub struct InferenceRouter {
    chain: Vec<Box<dyn InferenceAtom>>,
}

impl Default for InferenceRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl InferenceRouter {
    pub fn new() -> Self {
        Self {
            chain: vec![
                Box::new(OllamaAtom::default()),
                Box::new(ClaudeAtom::default()),
                Box::new(GeminiAtom::default()),
                Box::new(OpenAIAtom::default()),
            ],
        }
    }

    /// Override the Ollama model (e.g. "codellama" for ShaderFragment work).
    pub fn with_ollama_model(mut self, model: impl Into<String>) -> Self {
        if let Some(atom) = self.chain.get_mut(0) {
            // Downcast only works if we store concrete type — for now rebuild
            let _ = atom; // placeholder until trait supports model override
        }
        // Rebuild chain with new ollama model
        let model = model.into();
        self.chain[0] = Box::new(OllamaAtom { model, endpoint: "http://localhost:11434".into() });
        self
    }

    /// Route an InferenceCapsule through the chain.
    /// Returns the first successful InferenceOutput or an error listing all failures.
    pub fn route(&self, capsule: &InferenceCapsule) -> Result<InferenceOutput, RouterError> {
        let mut failures: Vec<(String, String)> = Vec::new();

        for atom in &self.chain {
            if !atom.supports(&capsule.output_type) {
                continue;
            }
            if !atom.health_check() {
                failures.push((atom.backend_id().to_string(), "health check failed".into()));
                continue;
            }
            match atom.infer(capsule) {
                Ok(output) => return Ok(output),
                Err(e)     => failures.push((atom.backend_id().to_string(), e)),
            }
        }

        Err(RouterError { failures })
    }
}

#[derive(Debug)]
pub struct RouterError {
    pub failures: Vec<(String, String)>,
}

impl std::fmt::Display for RouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InferenceRouter: all backends failed — ")?;
        for (backend, reason) in &self.failures {
            write!(f, "[{backend}: {reason}] ")?;
        }
        Ok(())
    }
}

impl std::error::Error for RouterError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capsule::OutputType;

    #[test]
    fn router_falls_through_all_stubs_to_error() {
        let router = InferenceRouter::new();
        let capsule = InferenceCapsule {
            output_type: OutputType::NarrativeText,
            prompt: "Describe this mountain range in one sentence.".into(),
            system: None,
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        // All backends are stubs, so we expect a RouterError listing all four
        let result = router.route(&capsule);
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Should have attempted at least the cloud backends (Ollama skips health)
        assert!(!err.failures.is_empty());
    }

    #[test]
    fn output_types_are_distinct() {
        assert_ne!(OutputType::NarrativeText, OutputType::ShaderFragment);
        assert_ne!(OutputType::StructuredJson, OutputType::Decision);
    }
}
