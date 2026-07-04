// VAULT-ATOM-08: Namespace Registrar — canonical ID → storage address map.
//
// Maintains the mapping from MythId to the on-disk path of its page file.
// Needed for any atom that needs to locate a page without going through
// BlobStorage directly (e.g., ColdStorageManager before moving a file).

use myth_wire::MythId;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct NamespaceRegistrar {
    registry: Arc<RwLock<HashMap<String, PathBuf>>>,
}

impl NamespaceRegistrar {
    pub fn register(&self, id: &MythId, path: PathBuf) {
        self.registry.write().unwrap().insert(id.as_str(), path);
    }

    pub fn resolve(&self, id: &MythId) -> Option<PathBuf> {
        self.registry.read().unwrap().get(&id.as_str()).cloned()
    }

    pub fn deregister(&self, id: &MythId) {
        self.registry.write().unwrap().remove(&id.as_str());
    }
}
