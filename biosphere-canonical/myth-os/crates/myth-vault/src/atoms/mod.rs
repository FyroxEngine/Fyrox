pub mod audit_log;          // VAULT-16 — append-only ndjson audit trail
pub mod blob_storage;       // VAULT-01 — mmap-backed binary page store
pub mod cache;              // VAULT-11 — LRU cache for hot capsules
pub mod cold_storage;       // VAULT-14 — offload stale capsules to archive
pub mod dedup_engine;       // VAULT-04 — fingerprint-based deduplication
pub mod fingerprinter;      // VAULT-02 — BLAKE3 content fingerprinting
pub mod hierarchy_mapper;   // VAULT-06 — parent-child relationship tree
pub mod integrity_auditor;  // VAULT-13 — checksum verification
pub mod lazy_loader;        // VAULT-10 — demand-paging retrieval
pub mod metadata_ingestor;  // VAULT-03 — JSON metadata extraction
pub mod namespace;          // VAULT-08 — ID → storage path registry
pub mod portal;             // VAULT-12 — cross-vault serialization
pub mod purge;              // VAULT-15 — controlled capsule deletion
pub mod relationship_weaver; // VAULT-05 — graph linkages between capsules
pub mod security_gate;      // VAULT-09 — token-based access validation
pub mod versioning;         // VAULT-07 — delta-compressed history
