// VAULT-ATOM-15: Purge Sequencer — controlled permanent deletion.
//
// Purge removes the page file from disk and deregisters the ID from the
// namespace. This is irreversible. The caller is responsible for verifying
// that the dedup ref count has hit zero before calling purge.

use crate::atoms::{blob_storage::BlobStorage, namespace::NamespaceRegistrar};
use crate::error::VaultResult;
use myth_wire::MythId;
use std::sync::Arc;
use tracing::info;

pub struct PurgeSequencer {
    storage: Arc<BlobStorage>,
    namespace: Arc<NamespaceRegistrar>,
}

impl PurgeSequencer {
    pub fn new(storage: Arc<BlobStorage>, namespace: Arc<NamespaceRegistrar>) -> Self {
        Self { storage, namespace }
    }

    /// Permanently delete a capsule from hot storage and remove its namespace entry.
    pub fn purge(&self, id: &MythId) -> VaultResult<()> {
        info!(capsule = %id, "Purging capsule");
        self.storage.delete(id)?;
        self.namespace.deregister(id);
        Ok(())
    }
}
