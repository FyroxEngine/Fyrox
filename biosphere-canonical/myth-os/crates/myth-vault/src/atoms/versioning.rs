// VAULT-ATOM-07: Versioning Controller — delta-compressed state history.
//
// Maintains an ordered sequence of deltas per capsule ID. Each commit
// appends a new Version with a monotonically increasing seq number.
// Full reconstruction requires replaying all deltas from seq 0.
// (Compression and full snapshots are future work — placeholder for now.)

use myth_wire::MythId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct Version {
    pub seq: u64,
    pub delta: Vec<u8>,
    pub parent_seq: Option<u64>,
}

#[derive(Default)]
pub struct VersioningController {
    history: Arc<RwLock<HashMap<String, Vec<Version>>>>,
}

impl VersioningController {
    /// Append a new delta for `id`. Returns the new seq number.
    pub fn commit(&self, id: &MythId, delta: Vec<u8>) -> u64 {
        let mut history = self.history.write().unwrap();
        let versions = history.entry(id.as_str()).or_default();
        let parent_seq = versions.last().map(|v| v.seq);
        let seq = versions.len() as u64;
        versions.push(Version { seq, delta, parent_seq });
        seq
    }

    pub fn version_count(&self, id: &MythId) -> usize {
        self.history
            .read()
            .unwrap()
            .get(&id.as_str())
            .map(|v| v.len())
            .unwrap_or(0)
    }
}
