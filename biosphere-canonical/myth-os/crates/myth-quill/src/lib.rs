// myth-quill — Quantum Quill, the original narrative engine.
//
// The entire Quantum Ecosystem was built on top of Quill. It defines:
//
//   Capsule           — the atomic unit (64-bit BDna, wire type, heraldry)
//   Container         — level 2 of the hierarchy
//   MythosContainer   — level 1 of the hierarchy
//   GenesisContainer  — level 0 — the sealed universe
//   HeraldricPosition — positional identity in the Heraldric Order
//   LifecycleState    — seeding → active → sealed → archived → deprecated
//   CapacityMetadata  — the Octave Capacity Law tracking
//
// Dependency rule: myth-wire + serde + chrono + thiserror only.
// No renderer deps. No tokio. No Bevy. No egui.

pub mod capsule;
pub mod container;
pub mod genesis;
pub mod heraldry;
pub mod lifecycle;

pub use capsule::Capsule;
pub use container::{Container, MythosContainer};
pub use genesis::{GenesisContainer, SealType, SeedQuestion};
pub use heraldry::HeraldricPosition;
pub use lifecycle::{CapacityMetadata, GrowthMode, LifecycleState};
