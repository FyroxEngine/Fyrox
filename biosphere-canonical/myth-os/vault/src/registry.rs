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
use mythos::{
    capsule::Capsule,
    identity::MythId,
};
use std::{path::Path, sync::Arc};
use tracing::info;

/// Top-level facade: boots all 16 Vault atoms and exposes the unified API.
pub struct VaultRegistry {
    pub storage: Arc<BlobStorage>,
    pub namespace: Arc<NamespaceRegistrar>,
    pub fingerprinter: Fingerprinter,
    pub dedup: DedupEngine,
    pub meta_ingestor: MetadataIngestor,
    pub hierarchy: HierarchyMapper,
    pub versioning: VersioningController,
    pub relationships: RelationshipWeaver,
    pub security: SecurityGate,
    pub loader: LazyLoader,
    pub cache: CacheOrchestrator,
    pub portal: PortalTranslator,
    pub auditor: IntegrityAuditor,
    pub cold: ColdStorageManager,
    pub purge: PurgeSequencer,
    pub audit_log: AuditLogger,
    vault_id: MythId,
}

impl VaultRegistry {
    pub fn open(root: impl AsRef<Path>) -> VaultResult<Self> {
        let root = root.as_ref();
        let hot_root = root.join("hot");
        let cold_root = root.join("cold");
        let log_root = root.join("log");
        std::fs::create_dir_all(&log_root)?;

        let storage = Arc::new(BlobStorage::open(&hot_root)?);
        let namespace = Arc::new(NamespaceRegistrar::default());

        info!("VaultRegistry online at {}", root.display());

        Ok(Self {
            loader: LazyLoader::new(Arc::clone(&storage)),
            auditor: IntegrityAuditor::new(Arc::clone(&storage)),
            purge: PurgeSequencer::new(Arc::clone(&storage), Arc::clone(&namespace)),
            cold: ColdStorageManager::new(&cold_root)?,
            audit_log: AuditLogger::open(&log_root)
                .map_err(|e| VaultError::Io(e))?,
            storage,
            namespace,
            fingerprinter: Fingerprinter,
            dedup: DedupEngine::default(),
            meta_ingestor: MetadataIngestor,
            hierarchy: HierarchyMapper::default(),
            versioning: VersioningController::default(),
            relationships: RelationshipWeaver::default(),
            security: SecurityGate::default(),
            cache: CacheOrchestrator::default(),
            portal: PortalTranslator,
            vault_id: MythId::new(),
        })
    }

    /// Ingest a capsule: fingerprint → dedup check → write → register.
    pub fn ingest(&self, capsule: &Capsule) -> VaultResult<MythId> {
        let fp = Fingerprinter::hash(&capsule.payload);
        let (canonical_id, is_dup) = self.dedup.register(&fp, capsule.id.clone());

        if !is_dup {
            self.storage.write(&canonical_id, &capsule.payload)?;
            self.namespace
                .register(&canonical_id, self.storage_path(&canonical_id));
        }

        self.audit_log
            .record(&self.vault_id, &canonical_id, "INGEST");
        self.cache.insert(&canonical_id, capsule.payload.clone());

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

    /// List the string IDs of all persisted capsules in hot storage.
    pub fn list_ids(&self) -> Vec<String> {
        self.storage.list_ids()
    }

    fn storage_path(&self, id: &MythId) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("{}.page", id.as_str()))
    }
}
