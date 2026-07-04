// myth-qgcp — Quantum Genesis Container Protocol
//
// Defines the canonical sealed container formats for the BioSpark ecosystem.
// All Genesis types share the same 16x16x16 hierarchy:
//   Genesis → 16 Mythos → 16 Containers → 16 Capsules = 4,096 capsule slots
//
// Genesis Family:
//   WorldGenesis  (.worldgenesis) — headless world state (server + client agents)
//   MediaGenesis  (.mediagenesis) — binary asset bundle (client Theater only)
//   ActorGenesis  (.actorgenesis) — agent/character container
//   UIGenesis     (.uigenesis)    — UI layout slot topology
//
// Short aliases (same type, less typing in implementations):
//   WorldGen  =  WorldGenesis
//   MediaGen  =  MediaGenesis
//   ActorGen  =  ActorGenesis
//   UIGen     =  UIGenesis
//
// A world ships as a directory:
//   worlds/kasmir-delta/
//       kasmir-delta.worldgenesis   ← simulation state
//       kasmir-delta.mediagenesis   ← assets and samples
//       meta.toml                   ← name, bDNA, checksums
//
// Dependency rule: myth-wire, serde, blake3, chrono, uuid only.
// No renderer deps. No tokio. No Bevy.

pub mod actor;
pub mod actor_container;
pub mod capsule;
pub mod cell;
pub mod container;
pub mod error;
pub mod genesis;
pub mod media;
pub mod mythos;
pub mod seal;
pub mod ui;

pub use actor::ActorGenesis;
pub use actor_container::{ActorContainer, MAX_CELLS};
pub use capsule::Capsule;
pub use cell::Cell;
pub use container::Container;
pub use error::QgcpError;
pub use genesis::WorldGenesis;
pub use media::{AssetRef, MediaGenesis, MediaType};
pub use mythos::MythosModule;
pub use seal::SealBlock;
pub use ui::{LayoutRegion, SlotDefinition, UIGenesis};

// Short aliases — same types, less typing in implementations.
pub type WorldGen  = WorldGenesis;
pub type MediaGen  = MediaGenesis;
pub type ActorGen  = ActorGenesis;
pub type UIGen     = UIGenesis;

/// The 16x16x16 Capacity Law — enforced on all Genesis types.
/// One Genesis → 16 Mythos → 16 Containers → 16 Capsules = 4,096 capsule slots.
pub const MAX_MYTHOS:     usize = 16;
pub const MAX_CONTAINERS: usize = 16;
pub const MAX_CAPSULES:   usize = 16;
pub const TOTAL_CAPACITY: usize = MAX_MYTHOS * MAX_CONTAINERS * MAX_CAPSULES;
