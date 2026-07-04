// myth-wire — The wire type protocol for the BioSpark / myth-os ecosystem.
//
// This crate is the zero-renderer-dependency foundation that every other crate
// in the workspace imports. It defines:
//
//   WireType   — the 17 canonical signal types
//   WirePacket — a typed, serialisable message envelope
//   BDna       — deterministic identity (64 bool positions)
//   MythId     — a UUID-based entity identifier
//   ChannelId  — a Theater channel identifier
//   Blake3Hash — a BLAKE3 content fingerprint
//
// Dependency rule: serde, bincode, uuid, blake3, thiserror only.
// If you are tempted to add bevy, egui, tokio, or any renderer here — stop.

pub mod bdna;
pub mod ids;
pub mod packet;
pub mod sigil;
pub mod wire_type;

pub use bdna::{BDna, BDnaError};
pub use ids::{Blake3Hash, ChannelId, MythId};
pub use packet::WirePacket;
pub use sigil::Sigil;
pub use wire_type::WireType;
