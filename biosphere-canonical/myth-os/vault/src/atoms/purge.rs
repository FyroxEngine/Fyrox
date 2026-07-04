// VAULT-ATOM-15: Purge Sequencer — controlled deletion of decommissioned assets
use crate::atoms::{blob_storage::BlobStorage, namespace::NamespaceRegistrar};
use crate::error::VaultResult;
use mythos::identity::MythId;
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

    pub fn purge(&self, id: &MythId) -> VaultResult<()> {
        info!(capsule = %id, "Purging capsule");
        self.storage.delete(id)?;
        self.namespace.deregister(id);
        Ok(())
    }
}
