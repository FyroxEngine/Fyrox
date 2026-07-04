// VaultRegistry — the unified facade over all 16 Vault atoms.
//
// The Registry is intentionally Capsule-agnostic. It speaks raw bytes and
// MythIds. Higher layers (myth-quill) wrap the Registry and work with the
// full typed Capsule. This keeps the Vault reusable by any crate that needs
// content-addressable persistent storage, not just the narrative engine.
//
// Call order for ingest:
//   fingerprint → dedup check → write to hot storage → register namespace
//   → audit log → cache
//
// Call order for fetch:
//   cache check → lazy load from storage → cache prime

use crate::atoms::{
    audit_log::AuditLogger,
    blob_storage::BlobStorage,
    cache::CacheOrchestrator,
    cold_storage::ColdStorageManager,
    dedup_engine::DedupEngine,
    fingerprinter::Fingerprinter,
    hierarchy_mapper::HierarchyMapper,
    integrity_auditor::IntegrityAuditor,
    lazy_loader::LazyLoader,
    metadata_ingestor::MetadataIngestor,
    namespace::NamespaceRegistrar,
    portal::PortalTranslator,
    purge::PurgeSequencer,
    relationship_weaver::RelationshipWeaver,
    security_gate::SecurityGate,
    versioning::VersioningController,
};
use crate::error::{VaultError, VaultResult};
use myth_wire::MythId;
use std::{path::Path, sync::Arc};
use tracing::info;

// ─── Vault Profile ────────────────────────────────────────────────────────────

/// Declares which plugin Departments this Vault accepts.
/// Set once at creation, stored in `vault.profile.json`, immutable after that.
/// The Plugin Registry filters its certified manifest by this profile at boot.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum VaultProfile {
    /// 3D world simulation — WorldConstruction + EntitySystems
    GenesisWorld,
    /// Story and dialogue authoring — NarrativeSystems + EntitySystems
    NarrativeStudio,
    /// Music and sound design — Composer + PipelineSystems
    AudioStudio,
    /// Video / film editing pipeline — NarrativeSystems + PipelineSystems
    FilmEdit,
    /// User-defined mix of departments
    Custom(Vec<PluginDepartment>),
    /// No restrictions — loads all certified plugins (admin / dev use only)
    Universal,
}

/// The department tags that plugins declare and Vaults filter on.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum PluginDepartment {
    WorldConstruction,
    EntitySystems,
    NarrativeSystems,
    PipelineSystems,
    /// Accepted by all VaultProfiles (logic, math, routing utilities)
    Universal,
}

impl VaultProfile {
    /// Returns true if a plugin with this department should be loaded by this vault.
    pub fn accepts(&self, dept: &PluginDepartment) -> bool {
        if *dept == PluginDepartment::Universal {
            return true;
        }
        match self {
            Self::GenesisWorld    => matches!(dept, PluginDepartment::WorldConstruction | PluginDepartment::EntitySystems),
            Self::NarrativeStudio => matches!(dept, PluginDepartment::NarrativeSystems  | PluginDepartment::EntitySystems),
            Self::AudioStudio     => matches!(dept, PluginDepartment::NarrativeSystems  | PluginDepartment::PipelineSystems),
            Self::FilmEdit        => matches!(dept, PluginDepartment::NarrativeSystems  | PluginDepartment::PipelineSystems),
            Self::Custom(depts)   => depts.contains(dept),
            Self::Universal       => true,
        }
    }

    /// Load the profile from `<vault_root>/vault.profile.json`.
    /// Returns `Universal` if the file doesn't exist (backwards compat / dev vaults).
    pub fn load(vault_root: &Path) -> Self {
        let path = vault_root.join("vault.profile.json");
        if !path.exists() {
            return Self::Universal;
        }
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Self::Universal)
    }

    /// Persist the profile to `<vault_root>/vault.profile.json`.
    pub fn save(&self, vault_root: &Path) -> VaultResult<()> {
        let path = vault_root.join("vault.profile.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

pub struct VaultRegistry {
    pub profile:       VaultProfile,
    pub storage:       Arc<BlobStorage>,
    pub namespace:     Arc<NamespaceRegistrar>,
    pub fingerprinter: Fingerprinter,
    pub dedup:         DedupEngine,
    pub meta_ingestor: MetadataIngestor,
    pub hierarchy:     HierarchyMapper,
    pub versioning:    VersioningController,
    pub relationships: RelationshipWeaver,
    pub security:      SecurityGate,
    pub loader:        LazyLoader,
    pub cache:         CacheOrchestrator,
    pub portal:        PortalTranslator,
    pub auditor:       IntegrityAuditor,
    pub cold:          ColdStorageManager,
    pub purge:         PurgeSequencer,
    pub audit_log:     AuditLogger,
    vault_id:          MythId,
}

impl VaultRegistry {
    /// Open (or create) a Vault at the given root directory.
    /// Subdirectories `hot/`, `cold/`, and `log/` are created automatically.
    /// Open a Vault with an explicit profile (used when creating a new Vault).
    pub fn open_with_profile(root: impl AsRef<Path>, profile: VaultProfile) -> VaultResult<Self> {
        let root = root.as_ref();
        profile.save(root)?;
        Self::open(root)
    }

    pub fn open(root: impl AsRef<Path>) -> VaultResult<Self> {
        let root = root.as_ref();
        let profile   = VaultProfile::load(root);
        let hot_root  = root.join("hot");
        let cold_root = root.join("cold");
        let log_root  = root.join("log");
        std::fs::create_dir_all(&log_root)?;

        let storage   = Arc::new(BlobStorage::open(&hot_root)?);
        let namespace = Arc::new(NamespaceRegistrar::default());

        info!("VaultRegistry online at {}", root.display());

        Ok(Self {
            profile,
            loader:       LazyLoader::new(Arc::clone(&storage)),
            auditor:      IntegrityAuditor::new(Arc::clone(&storage)),
            purge:        PurgeSequencer::new(Arc::clone(&storage), Arc::clone(&namespace)),
            cold:         ColdStorageManager::new(&cold_root)?,
            audit_log:    AuditLogger::open(&log_root)
                              .map_err(VaultError::Io)?,
            storage,
            namespace,
            fingerprinter: Fingerprinter,
            dedup:         DedupEngine::default(),
            meta_ingestor: MetadataIngestor,
            hierarchy:     HierarchyMapper::default(),
            versioning:    VersioningController::default(),
            relationships: RelationshipWeaver::default(),
            security:      SecurityGate::default(),
            cache:         CacheOrchestrator::default(),
            portal:        PortalTranslator,
            vault_id:      MythId::new(),
        })
    }

    /// Ingest raw bytes identified by `id`.
    ///
    /// Pipeline: fingerprint → dedup check → write → namespace register
    ///           → audit → cache.
    ///
    /// Returns the canonical MythId (may differ from `id` if a duplicate
    /// was detected and an earlier canonical ID already existed).
    pub fn ingest(&self, id: MythId, payload: &[u8]) -> VaultResult<MythId> {
        let fp = Fingerprinter::hash(payload);
        let (canonical_id, is_dup) = self.dedup.register(&fp, id);

        if !is_dup {
            self.storage.write(&canonical_id, payload)?;
            self.namespace
                .register(&canonical_id, self.hot_page_path(&canonical_id));
        }

        self.audit_log.record(&self.vault_id, &canonical_id, "INGEST");
        self.cache.insert(&canonical_id, payload.to_vec());

        Ok(canonical_id)
    }

    /// Retrieve a capsule's raw bytes (cache → hot storage).
    pub fn fetch(&self, id: &MythId) -> VaultResult<Vec<u8>> {
        if let Some(cached) = self.cache.get(id) {
            return Ok(cached);
        }
        let data = self.loader.pin(id)?;
        self.cache.insert(id, data.clone());
        Ok(data)
    }

    /// List all capsule IDs currently persisted in hot storage.
    pub fn list_ids(&self) -> Vec<String> {
        self.storage.list_ids()
    }

    fn hot_page_path(&self, id: &MythId) -> std::path::PathBuf {
        // Mirrors BlobStorage::page_path — kept consistent via convention.
        std::path::PathBuf::from(format!("data/vault/hot/{}.page", id.as_str()))
    }
}
