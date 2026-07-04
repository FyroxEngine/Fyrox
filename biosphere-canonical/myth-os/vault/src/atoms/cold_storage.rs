// VAULT-ATOM-14: Cold Storage Manager — compress and offload stale capsules
use crate::error::VaultResult;
use mythos::identity::MythId;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ColdStorageManager {
    archive_root: PathBuf,
}

impl ColdStorageManager {
    pub fn new(archive_root: impl AsRef<Path>) -> VaultResult<Self> {
        let archive_root = archive_root.as_ref().to_path_buf();
        fs::create_dir_all(&archive_root)?;
        Ok(Self { archive_root })
    }

    /// Move a raw page into cold storage (rename, no compression yet — zstd to follow).
    pub fn offload(&self, id: &MythId, hot_path: &Path) -> VaultResult<PathBuf> {
        let cold_path = self.archive_root.join(format!("{}.cold", id.as_str()));
        fs::rename(hot_path, &cold_path)?;
        Ok(cold_path)
    }

    pub fn cold_path(&self, id: &MythId) -> PathBuf {
        self.archive_root.join(format!("{}.cold", id.as_str()))
    }
}
