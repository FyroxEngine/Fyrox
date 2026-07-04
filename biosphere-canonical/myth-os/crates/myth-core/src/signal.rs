// Core system signals — the internal bus protocol for myth-core.
//
// BusSignal is NOT a WirePacket. WirePackets cross module boundaries via the
// Theater. BusSignals route internally within the Core process — they carry
// lifecycle events (tick, spawn, shutdown) between Core atoms on the broadcast bus.

use myth_wire::MythId;
use serde::{Deserialize, Serialize};

/// System-level bus message. Routes between Core atoms on the internal broadcast bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusSignal {
    pub origin: MythId,
    pub destination: SignalDestination,
    pub kind: SignalKind,
    pub priority: SignalPriority,
    pub payload: SignalPayload,
}

/// Where a bus signal should be delivered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalDestination {
    Core,
    Vault,
    Genesis,
    Broadcast,
    Specific(MythId),
}

/// The type of system event being signaled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalKind {
    Tick,
    Spawn,
    Despawn,
    Migrate,
    Snapshot,
    Shutdown,
    Heartbeat,
    Custom(String),
}

/// Priority determines interrupt queue ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SignalPriority {
    Low      = 0,
    Normal   = 1,
    High     = 2,
    Critical = 3,
}

/// Optional payload attached to a bus signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalPayload {
    Empty,
    Bytes(Vec<u8>),
    Json(String),
}

impl BusSignal {
    /// A normal-priority clock tick broadcast.
    pub fn tick(origin: MythId) -> Self {
        Self {
            origin,
            destination: SignalDestination::Broadcast,
            kind: SignalKind::Tick,
            priority: SignalPriority::Normal,
            payload: SignalPayload::Empty,
        }
    }

    /// A critical-priority shutdown broadcast.
    pub fn shutdown(origin: MythId) -> Self {
        Self {
            origin,
            destination: SignalDestination::Broadcast,
            kind: SignalKind::Shutdown,
            priority: SignalPriority::Critical,
            payload: SignalPayload::Empty,
        }
    }

    /// A normal-priority heartbeat from a known node.
    pub fn heartbeat(origin: MythId) -> Self {
        Self {
            origin,
            destination: SignalDestination::Core,
            kind: SignalKind::Heartbeat,
            priority: SignalPriority::Low,
            payload: SignalPayload::Empty,
        }
    }
}
