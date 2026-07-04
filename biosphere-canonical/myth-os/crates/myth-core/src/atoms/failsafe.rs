// CORE-ATOM-15: Emergency Failsafe — microsecond isolation of crashed modules.
//
// When HealthMonitor detects a stale node, Core isolates it here.
// Isolated nodes are removed from routing — the Theater will not route
// WirePackets to them until Core explicitly releases the isolation.
// Think of this as the circuit breaker for the entire module graph.

use myth_wire::MythId;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use tracing::error;

#[derive(Default)]
pub struct EmergencyFailsafe {
    isolated: Arc<RwLock<HashSet<String>>>,
}

impl EmergencyFailsafe {
    /// Isolate a node. Emits an error-level log. Idempotent.
    pub fn isolate(&self, id: &MythId) {
        error!(node = %id, "FAILSAFE: isolating node");
        self.isolated.write().unwrap().insert(id.as_str());
    }

    /// Release a previously isolated node back into the routing graph.
    pub fn release(&self, id: &MythId) {
        self.isolated.write().unwrap().remove(&id.as_str());
    }

    pub fn is_isolated(&self, id: &MythId) -> bool {
        self.isolated.read().unwrap().contains(&id.as_str())
    }

    pub fn isolated_count(&self) -> usize {
        self.isolated.read().unwrap().len()
    }
}
