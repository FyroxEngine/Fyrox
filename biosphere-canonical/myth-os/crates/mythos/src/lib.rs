// mythos — BioSpark Agent Workforce types.
//
// Layer 1 crate: zero renderer deps, zero async runtime.
// All agent simulation types live here. Simulation behavior (tick logic)
// lives in modules/forge which depends on this crate.
//
// Dependency rule: myth-wire + serde + bincode ONLY.

pub mod agents;

pub use agents::{
    Agent, AgentCouncil, AgentId, AgentRole, Department, EmergenceReport,
    EmotionArray, LifecyclePhase, NeuralRack, Race, SoulWeight,
};
