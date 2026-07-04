// VAULT-ATOM-10: Lazy Loader — demand-paging asset retrieval.
//
// Tracks which capsules are currently "pinned" in active memory.
// Pinning retrieves the bytes from BlobStorage and marks the ID as active.
// Unpinning evicts the mmap page, freeing OS memory.

use crate::atoms::blob_storage::BlobStorage;
use crate::error::VaultResult;
use myth_wire::MythId;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

pub struct LazyLoader {
    pinned: Arc<RwLock<HashSet<String>>>,
    storage: Arc<BlobStorage>,
}

impl LazyLoader {
    pub fn new(storage: Arc<BlobStorage>) -> Self {
        Self {
            pinned: Arc::new(RwLock::new(HashSet::new())),
            storage,
        }
    }

    /// Load a capsule into active memory and mark it as pinned.
    pub fn pin(&self, id: &MythId) -> VaultResult<Vec<u8>> {
        self.pinned.write().unwrap().insert(id.as_str());
        self.storage.read(id)
    }

    /// Release a pinned capsule and evict its mmap page.
    pub fn unpin(&self, id: &MythId) {
        self.pinned.write().unwrap().remove(&id.as_str());
        self.storage.evict(id);
    }

    pub fn is_pinned(&self, id: &MythId) -> bool {
        self.pinned.read().unwrap().contains(&id.as_str())
    }

    pub fn pinned_count(&self) -> usize {
        self.pinned.read().unwrap().len()
    }
}
