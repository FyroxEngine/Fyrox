// myth-core — BioSpark runtime core.
//
// This crate is the Single Source of Truth (SSoT) that all Quantum modules
// refer to for runtime primitives: clock, bus, health monitoring, node
// lifecycle, interrupts, and audit logging.
//
// Dependency rule: myth-wire + tokio + serde + tracing + anyhow only.
// No renderer deps. No Bevy. No egui. No audio.
//
// The bin/ target (myth-core) is a thin wrapper that boots the atoms
// and listens for Ctrl-C. The library is what other crates import.

pub mod atoms;
pub mod signal;

pub use signal::{BusSignal, SignalDestination, SignalKind, SignalPayload, SignalPriority};
