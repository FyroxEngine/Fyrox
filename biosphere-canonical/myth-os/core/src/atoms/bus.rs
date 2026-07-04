// CORE-ATOM-02: Bus Router — zero-copy async message routing
use mythos::signal::{BusSignal, SignalDestination, SignalPriority};
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

    pub fn subscribe(&self) -> broadcast::Receiver<BusSignal> {
        self.tx.subscribe()
    }

    pub fn route(&self, signal: BusSignal) {
        if signal.priority == SignalPriority::Critical {
            debug!(kind = ?signal.kind, dest = ?signal.destination, "CRITICAL signal routed");
        }
        if let Err(e) = self.tx.send(signal) {
            warn!("Bus route failed (no receivers): {:?}", e);
        }
    }

    pub fn sender(&self) -> broadcast::Sender<BusSignal> {
        self.tx.clone()
    }
}
