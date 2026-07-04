//! MOLECULE: InferenceRouter
//!
//! A pre-wired ATOM sub-graph that routes Capsule inference requests to the
//! best available LLM backend. Priority order is always local-first:
//!
//!   Ollama (local) → Claude → Gemini → OpenAI → Error
//!
//! Each backend is an ATOM. The router probes health, picks the first live
//! backend whose capability matches the Capsule's `output_type`, and forwards.
//!
//! Drop this MOLECULE into any ATOM graph where you need inference without
//! caring which model handles it.

pub mod atoms;
pub mod router;
pub mod capsule;

pub use router::InferenceRouter;
pub use capsule::{InferenceCapsule, InferenceOutput, OutputType};
