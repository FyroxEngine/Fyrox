// VAULT-ATOM-04: Deduplication Engine
use mythos::identity::{Blake3Hash, MythId};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default)]
pub struct DedupEngine {
    // fingerprint -> canonical ID + ref count
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

    /// Decrement ref count; returns true if the capsule should be purged.
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
