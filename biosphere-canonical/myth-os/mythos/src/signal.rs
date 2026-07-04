use crate::identity::MythId;
use serde::{Deserialize, Serialize};

/// System-level bus message. Routes between Core, Vault, and Genesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusSignal {
    pub origin: MythId,
    pub destination: SignalDestination,
    pub kind: SignalKind,
    pub priority: SignalPriority,
    pub payload: SignalPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalDestination {
    Core,
    Vault,
    Genesis,
    Broadcast,
    Specific(MythId),
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SignalPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalPayload {
    Empty,
    Bytes(Vec<u8>),
    Json(String),
}

impl BusSignal {
    pub fn tick(origin: MythId) -> Self {
        Self {
            origin,
            destination: SignalDestination::Broadcast,
            kind: SignalKind::Tick,
            priority: SignalPriority::Normal,
            payload: SignalPayload::Empty,
        }
    }

    pub fn shutdown(origin: MythId) -> Self {
        Self {
            origin,
            destination: SignalDestination::Broadcast,
            kind: SignalKind::Shutdown,
            priority: SignalPriority::Critical,
            payload: SignalPayload::Empty,
        }
    }
}
