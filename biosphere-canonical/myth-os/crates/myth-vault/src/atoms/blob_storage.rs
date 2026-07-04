// VAULT-ATOM-01: Blob Storage — memory-mapped binary page store.
//
// Each capsule's raw bytes are written to a `.page` file and kept
// memory-mapped for zero-copy reads. Pages are evicted from the mmap
// cache on demand but never deleted from disk unless purge() is called.

use crate::error::{VaultError, VaultResult};
use memmap2::MmapMut;
use myth_wire::MythId;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

pub struct BlobStorage {
    root: PathBuf,
    /// Live mmap handles keyed by capsule ID — kept open for zero-copy reads.
    pages: Arc<RwLock<HashMap<String, MmapMut>>>,
}

impl BlobStorage {
    pub fn open(root: impl AsRef<Path>) -> VaultResult<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            pages: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Write raw bytes for a capsule to disk and memory-map the result.
    pub fn write(&self, id: &MythId, data: &[u8]) -> VaultResult<()> {
        let path = self.page_path(id);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        file.set_len(data.len() as u64)?;
        let mut mmap = unsafe { MmapMut::map_mut(&file)? };
        mmap.copy_from_slice(data);
        mmap.flush()?;
        self.pages.write().unwrap().insert(id.as_str(), mmap);
        Ok(())
    }

    /// Read a capsule's bytes. Returns a copy so the lock is not held by callers.
    pub fn read(&self, id: &MythId) -> VaultResult<Vec<u8>> {
        // Fast path: still mapped.
        {
            let pages = self.pages.read().unwrap();
            if let Some(page) = pages.get(&id.as_str()) {
                return Ok(page.to_vec());
            }
        }
        // Cold path: re-map from disk.
        let path = self.page_path(id);
        if !path.exists() {
            return Err(VaultError::NotFound { id: id.as_str() });
        }
        let file = OpenOptions::new().read(true).write(true).open(&path)?;
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        let data = mmap.to_vec();
        self.pages.write().unwrap().insert(id.as_str(), mmap);
        Ok(data)
    }

    /// Evict the mmap page from memory without deleting the file.
    pub fn evict(&self, id: &MythId) {
        self.pages.write().unwrap().remove(&id.as_str());
    }

    /// Permanently delete the page from disk and memory.
    pub fn delete(&self, id: &MythId) -> VaultResult<()> {
        self.pages.write().unwrap().remove(&id.as_str());
        let path = self.page_path(id);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn exists(&self, id: &MythId) -> bool {
        self.page_path(id).exists()
    }

    /// List all capsule IDs currently persisted in this storage root.
    pub fn list_ids(&self) -> Vec<String> {
        fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let path = e.path();
                if path.extension()? == "page" {
                    Some(path.file_stem()?.to_str()?.to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    fn page_path(&self, id: &MythId) -> PathBuf {
        self.root.join(format!("{}.page", id.as_str()))
    }
}
