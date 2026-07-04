use std::path::Path;
use crate::error::RegistryError;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PluginStatus {
    Certified,
    Revoked,
    Suspended,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginEntry {
    pub id:           String,
    pub name:         String,
    pub version:      String,
    pub heraldry:     String,
    pub hash:         String,
    pub wasm_path:    String,
    pub status:       PluginStatus,
    pub certified_at: String,
    pub author:       String,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PluginManifest {
    pub entries: Vec<PluginEntry>,
}

impl PluginManifest {
    pub fn load_or_create(path: impl AsRef<Path>) -> Result<Self, RegistryError> {
        let path = path.as_ref();
        if path.exists() {
            let text = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&text)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), RegistryError> {
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }

    pub fn add(&mut self, entry: PluginEntry) {
        self.entries.push(entry);
    }

    pub fn find_by_id(&self, id: &str) -> Option<&PluginEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn find_by_hash(&self, hash: &str) -> Option<&PluginEntry> {
        self.entries.iter().find(|e| e.hash == hash)
    }

    pub fn set_status(&mut self, id: &str, status: PluginStatus) -> Result<(), RegistryError> {
        self.entries.iter_mut()
            .find(|e| e.id == id)
            .map(|e| e.status = status)
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))
    }

    pub fn entries_with_status(&self, status: PluginStatus) -> Vec<&PluginEntry> {
        self.entries.iter().filter(|e| e.status == status).collect()
    }
}
