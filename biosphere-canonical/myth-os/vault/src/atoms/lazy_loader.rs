// VAULT-ATOM-10: Lazy-Loader — demand-paging asset retrieval
use crate::atoms::blob_storage::BlobStorage;
use crate::error::VaultResult;
use mythos::identity::MythId;
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

    /// Pin a capsule into active memory on demand.
    pub fn pin(&self, id: &MythId) -> VaultResult<Vec<u8>> {
        self.pinned.write().unwrap().insert(id.as_str());
        self.storage.read(id)
    }

    pub fn unpin(&self, id: &MythId) {
        self.pinned.write().unwrap().remove(&id.as_str());
        self.storage.evict(id);
    }

    pub fn is_pinned(&self, id: &MythId) -> bool {
        self.pinned.read().unwrap().contains(&id.as_str())
    }
}
