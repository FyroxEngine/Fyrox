// CORE-ATOM-13: Health Monitor — continuous node heartbeat diagnostics.
//
// Every node that wants to be considered "alive" must call record_beat()
// on every tick (or at least once per STALE_THRESHOLD). stale_nodes()
// returns IDs that have gone quiet — Core uses this to trigger failsafe.

use myth_wire::MythId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::warn;

const STALE_THRESHOLD: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct Heartbeat {
    pub last_seen: Instant,
    pub tick_count: u64,
}

#[derive(Default)]
pub struct HealthMonitor {
    beats: Arc<RwLock<HashMap<String, Heartbeat>>>,
}

impl HealthMonitor {
    pub fn record_beat(&self, id: &MythId) {
        let mut beats = self.beats.write().unwrap();
        let entry = beats.entry(id.as_str()).or_insert(Heartbeat {
            last_seen: Instant::now(),
            tick_count: 0,
        });
        entry.last_seen = Instant::now();
        entry.tick_count += 1;
    }

    /// Returns IDs of nodes that haven't checked in within STALE_THRESHOLD.
    pub fn stale_nodes(&self) -> Vec<String> {
        let beats = self.beats.read().unwrap();
        let now = Instant::now();
        beats
            .iter()
            .filter(|(_, b)| now.duration_since(b.last_seen) > STALE_THRESHOLD)
            .map(|(id, _)| {
                warn!(node = %id, "Node is stale");
                id.clone()
            })
            .collect()
    }

    pub fn beat_count(&self, id: &MythId) -> u64 {
        self.beats
            .read()
            .unwrap()
            .get(&id.as_str())
            .map(|b| b.tick_count)
            .unwrap_or(0)
    }
}
