// VAULT-ATOM-13: Integrity Auditor — background checksum verification
use crate::atoms::{blob_storage::BlobStorage, fingerprinter::Fingerprinter};
use crate::error::{VaultError, VaultResult};
use mythos::identity::{Blake3Hash, MythId};
use std::sync::Arc;

pub struct IntegrityAuditor {
    storage: Arc<BlobStorage>,
}

impl IntegrityAuditor {
    pub fn new(storage: Arc<BlobStorage>) -> Self {
        Self { storage }
    }

    pub fn verify(&self, id: &MythId, expected: &Blake3Hash) -> VaultResult<()> {
        let data = self.storage.read(id)?;
        if Fingerprinter::verify(&data, expected) {
            Ok(())
        } else {
            let actual = Fingerprinter::hash(&data);
            Err(VaultError::IntegrityViolation {
                id: id.as_str(),
                expected: expected.to_string(),
                actual: actual.to_string(),
            })
        }
    }
}
