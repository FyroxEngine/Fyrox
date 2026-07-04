// mythos::network — Wire-typed message envelope for cross-vault communication.
//
// This module defines the seam between Genesis instances and the transport
// layer (Phase 9: gRPC / WebSocket). The transport is not implemented here —
// only the message types. Any system that produces or consumes inter-vault
// messages uses these types so the Phase 9 transport slots in without
// touching business logic.
//
// Design note (from ADR critique): the network protocol seam must be defined
// before world state grows too large to reshape. These types establish that
// seam. All 17 wire types from the Quantum taxonomy are represented.

use serde::{Deserialize, Serialize};

use crate::identity::MythId;
use crate::soul::{MigrationReason, SocialBond, SoulMigration};

// ── Wire type taxonomy ────────────────────────────────────────────────────────

/// The 17 canonical signal wire types of the Quantum Ecosystem.
/// Mirrors the string codes used in the rack UI ("SPA", "BHV", …) but as a
/// typed enum for use in network messages and event routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WireType {
    Data,       // DAT — typed JSON payloads, universal fallback
    Control,    // CTL — boolean, gate, trigger signals
    Audio,      // AUD — waveform / sample streams
    Narrative,  // NAR — story / text / lore payloads
    Temporal,   // TMP — time, tick, clock signals
    Agent,      // AGT — agent instruction / state streams
    Visual,     // VIS — image / render / shader streams
    Spatial,    // SPA — 3D / voxel / coordinate data
    Behavioral, // BHV — emotion / drive / decision signals
    Social,     // SOC — relationship / faction / reputation data
    Energy,     // ENR — power / resource flow values
    Identity,   // IDN — B-DNA / lineage / covenant data
    Event,      // EVT — COSMIC bus events
    Asset,      // AST — file / binary / media references
    Meta,       // MET — schema / type / structure definitions
    Logic,      // LGC — boolean expression / rule streams
    Resonance,  // RES — resonance field values (gravity of the graph)
}

impl WireType {
    /// 3-character code used in the rack UI and Scribe binary streams.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Data      => "DAT",
            Self::Control   => "CTL",
            Self::Audio     => "AUD",
            Self::Narrative => "NAR",
            Self::Temporal  => "TMP",
            Self::Agent     => "AGT",
            Self::Visual    => "VIS",
            Self::Spatial   => "SPA",
            Self::Behavioral => "BHV",
            Self::Social    => "SOC",
            Self::Energy    => "ENR",
            Self::Identity  => "IDN",
            Self::Event     => "EVT",
            Self::Asset     => "AST",
            Self::Meta      => "MET",
            Self::Logic     => "LGC",
            Self::Resonance => "RES",
        }
    }

    /// Resolve a 3-char code string back to a WireType.
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "DAT" => Some(Self::Data),
            "CTL" => Some(Self::Control),
            "AUD" => Some(Self::Audio),
            "NAR" => Some(Self::Narrative),
            "TMP" => Some(Self::Temporal),
            "AGT" => Some(Self::Agent),
            "VIS" => Some(Self::Visual),
            "SPA" => Some(Self::Spatial),
            "BHV" => Some(Self::Behavioral),
            "SOC" => Some(Self::Social),
            "ENR" => Some(Self::Energy),
            "IDN" => Some(Self::Identity),
            "EVT" => Some(Self::Event),
            "AST" => Some(Self::Asset),
            "MET" => Some(Self::Meta),
            "LGC" => Some(Self::Logic),
            "RES" => Some(Self::Resonance),
            _     => None,
        }
    }
}

// ── Network message envelope ──────────────────────────────────────────────────

/// A wire-typed message exchanged between Genesis instances, Library, and
/// the Vault transport layer.
///
/// Every variant is fully serialisable. The Phase 9 transport wraps these
/// in a length-prefixed binary frame (header: wire_type code + payload_len u32).
/// Until Phase 9 the messages are used in-process via Bevy EventWriter/Reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    // ── Soul transport (IDN wire) ─────────────────────────────────────────

    /// A soul has arrived at this vault after migration.
    SoulArrived(SoulMigration),

    /// A soul is departing this vault — notify the destination.
    SoulDeparting {
        soul_id:    MythId,
        dest_vault: MythId,
        reason:     MigrationReason,
    },

    // ── World state sync (SPA + TMP wire) ────────────────────────────────

    /// Periodic snapshot of world-level state for a connected client.
    /// Sent every N ticks (configurable); not every frame.
    WorldStateSnapshot {
        tick:         u64,
        vault_id:     MythId,
        soul_count:   usize,
        /// Current base resonance frequency of the world in Hz.
        resonance_hz: f64,
        timestamp:    u64,
    },

    // ── Stimulus broadcast (BHV wire) ─────────────────────────────────────

    /// A stimulus event that may build resonance memory in the target soul.
    /// Broadcast when an actor interacts with a voxel or another actor.
    StimulusBroadcast {
        source_id: MythId,
        target_id: MythId,
        intensity: f32,
        valence:   f32,
        /// Resonance frequency of the emitting source (e.g. 800.0 for fire).
        source_hz: Option<f32>,
    },

    // ── Social graph (SOC wire) ───────────────────────────────────────────

    /// A social bond was created or updated between two souls.
    BondUpdated(SocialBond),

    // ── Canon events (RES wire) ───────────────────────────────────────────

    /// An event stream was audited by the Quantum Quill and stamped as canon.
    /// The blake3_hash is the departure fingerprint of the event payload.
    CanonEvent {
        tick:            u64,
        entity_id:       MythId,
        wire_type:       WireType,
        blake3_hash:     String,
        narrative_label: String,
    },

    // ── Resonance navigation (RES wire) ──────────────────────────────────

    /// An actor has entered the proximity threshold of a resonance target and
    /// snapped to its absolute voxel coordinate (the "Reality Snap").
    /// Notifies connected vaults that the actor is now at this position.
    RealitySnap {
        soul_id:      MythId,
        target_hz:    f32,
        snapped_x:    f32,
        snapped_y:    f32,
        snapped_z:    f32,
        tick:         u64,
    },

    // ── Heartbeat (CTL wire) ──────────────────────────────────────────────

    Ping { vault_id: MythId, tick: u64 },
    Pong { vault_id: MythId, tick: u64 },
}

impl NetworkMessage {
    /// The wire type this message travels on.
    pub fn wire_type(&self) -> WireType {
        match self {
            Self::SoulArrived(_)
            | Self::SoulDeparting { .. }    => WireType::Identity,
            Self::WorldStateSnapshot { .. } => WireType::Spatial,
            Self::StimulusBroadcast { .. }  => WireType::Behavioral,
            Self::BondUpdated(_)            => WireType::Social,
            Self::CanonEvent { .. }
            | Self::RealitySnap { .. }      => WireType::Resonance,
            Self::Ping { .. }
            | Self::Pong { .. }             => WireType::Control,
        }
    }

    /// True for messages that cross vault boundaries (require Phase 9 transport).
    pub fn is_cross_vault(&self) -> bool {
        matches!(
            self,
            Self::SoulArrived(_)
            | Self::SoulDeparting { .. }
            | Self::CanonEvent { .. }
            | Self::RealitySnap { .. }
        )
    }
}
