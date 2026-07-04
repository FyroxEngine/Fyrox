// VAULT-ATOM-04: Deduplication Engine — fingerprint-keyed reference counting.
//
// When the same bytes are ingested twice, DedupEngine returns the canonical
// MythId from the first ingest and increments the reference count.
// When a capsule is released and the ref count hits zero, the caller should
// delete the underlying page.

use myth_wire::{Blake3Hash, MythId};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default)]
pub struct DedupEngine {
    /// fingerprint → (canonical_id, ref_count)
    index: Arc<RwLock<HashMap<[u8; 32], (MythId, u32)>>>,
}

impl DedupEngine {
    /// Register a fingerprint. Returns the canonical ID and whether it was a duplicate.
    pub fn register(&self, fp: &Blake3Hash, id: MythId) -> (MythId, bool) {
        let mut index = self.index.write().unwrap();
        if let Some((canonical, count)) = index.get_mut(&fp.0) {
            *count += 1;
            (canonical.clone(), true)
        } else {
            index.insert(fp.0, (id.clone(), 1));
            (id, false)
        }
    }

    /// Decrement ref count. Returns true if the capsule should be purged (count hit zero).
    pub fn release(&self, fp: &Blake3Hash) -> bool {
        let mut index = self.index.write().unwrap();
        if let Some((_, count)) = index.get_mut(&fp.0) {
            *count -= 1;
            if *count == 0 {
                index.remove(&fp.0);
                return true;
            }
        }
        false
    }

    pub fn ref_count(&self, fp: &Blake3Hash) -> u32 {
        self.index
            .read()
            .unwrap()
            .get(&fp.0)
            .map(|(_, c)| *c)
            .unwrap_or(0)
    }
}
