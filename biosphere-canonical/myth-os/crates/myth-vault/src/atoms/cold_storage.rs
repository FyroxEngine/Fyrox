// VAULT-ATOM-14: Cold Storage Manager — archive offload for stale capsules.
//
// Moves capsule page files from the hot storage root to the cold archive root.
// No compression yet — that's a zstd layer to be added later.
// The cold path is deterministic: {archive_root}/{id}.cold

use crate::error::VaultResult;
use myth_wire::MythId;
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

    /// Move a hot page into cold storage. The hot path is removed.
    pub fn offload(&self, id: &MythId, hot_path: &Path) -> VaultResult<PathBuf> {
        let cold_path = self.cold_path(id);
        fs::rename(hot_path, &cold_path)?;
        Ok(cold_path)
    }

    pub fn cold_path(&self, id: &MythId) -> PathBuf {
        self.archive_root.join(format!("{}.cold", id.as_str()))
    }

    pub fn is_cold(&self, id: &MythId) -> bool {
        self.cold_path(id).exists()
    }
}
