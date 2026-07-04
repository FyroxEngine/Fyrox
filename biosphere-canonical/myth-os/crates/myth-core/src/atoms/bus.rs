// CORE-ATOM-02: Bus Router — zero-copy async broadcast message routing.
//
// Every atom that needs to emit or receive internal signals subscribes here.
// The bus is a tokio broadcast channel — all subscribers see every message.
// Use SignalDestination to filter at the subscriber level.

use crate::signal::{BusSignal, SignalPriority};
use tokio::sync::broadcast;
use tracing::{debug, warn};

pub struct BusRouter {
    tx: broadcast::Sender<BusSignal>,
}

impl BusRouter {
    pub fn new(capacity: usize) -> (Self, broadcast::Receiver<BusSignal>) {
        let (tx, rx) = broadcast::channel(capacity);
        (Self { tx }, rx)
    }

    /// Subscribe a new receiver to all bus signals.
    pub fn subscribe(&self) -> broadcast::Receiver<BusSignal> {
        self.tx.subscribe()
    }

    /// Route a signal to all subscribers.
    pub fn route(&self, signal: BusSignal) {
        if signal.priority == SignalPriority::Critical {
            debug!(kind = ?signal.kind, dest = ?signal.destination, "CRITICAL signal routed");
        }
        if let Err(e) = self.tx.send(signal) {
            warn!("Bus route failed (no receivers): {:?}", e);
        }
    }

    /// Clone the sender so atoms can emit signals without holding a BusRouter ref.
    pub fn sender(&self) -> broadcast::Sender<BusSignal> {
        self.tx.clone()
    }
}
