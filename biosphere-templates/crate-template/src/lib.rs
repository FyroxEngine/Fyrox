// BioSpark Quantum Genesis — Crate Template
//
// RENAME_ME: Replace this module name and all RENAME_ME / DESCRIBE_* placeholders.
//
// This crate is a pure data/logic crate. It must NOT depend on the Fyrox renderer
// or editor. If you need Fyrox types (Handle, Pool, math), use fyrox-core only.
//
// Standard module layout — delete what you don't need, add what you do:

pub mod config;     // Serializable configuration types
pub mod domain;     // Core domain types (enums, structs, rules)
pub mod error;      // Error types (use thiserror or manual Display impls)
pub mod ops;        // Business logic operations (mutation, validation)

// Re-export the most-used types at crate root so callers write:
//   use fyrox_biosphere_RENAME_ME::MyType;
// instead of digging through modules.
pub use domain::ExampleType;
pub use error::ModuleError;
