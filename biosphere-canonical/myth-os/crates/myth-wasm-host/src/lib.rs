//! WASM Host — loads certified .wasm plugins via Wasmtime and bridges
//! them to the MythPlugin trait interface.
//!
//! # Why this exists
//!
//! Core instruments (myth-atlas, etc.) are compiled into the binary.
//! User plugins are .wasm files loaded at runtime. This crate is the
//! bridge: it embeds a Wasmtime engine, enforces a call timeout so a
//! hung plugin can't freeze the simulation, and exposes the loaded
//! plugin as a Box<dyn MythPlugin> that the rest of myth-os treats
//! identically to a compiled-in instrument.
//!
//! # Security model
//!
//! - Only loads plugins whose hash matches a certified PluginRegistry entry
//! - WASM runs in a capability-restricted Wasmtime store (no filesystem, no network)
//! - process() calls are wrapped with a fuel/timeout limit
//! - A panicking plugin kills its Store, not the host process

pub mod host;
pub mod sandbox;
pub mod error;
pub mod abi;

pub use host::WasmPluginHost;
pub use error::WasmHostError;
