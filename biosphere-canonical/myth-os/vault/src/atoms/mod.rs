// Layer I — Ingestion
pub mod blob_storage;        // VAULT-01
pub mod fingerprinter;       // VAULT-02
pub mod metadata_ingestor;   // VAULT-03
pub mod dedup_engine;        // VAULT-04

// Layer II — Organization
pub mod relationship_weaver; // VAULT-05
pub mod hierarchy_mapper;    // VAULT-06
pub mod versioning;          // VAULT-07
pub mod namespace;           // VAULT-08

// Layer III — Access
pub mod security_gate;       // VAULT-09
pub mod lazy_loader;         // VAULT-10
pub mod cache;               // VAULT-11
pub mod portal;              // VAULT-12

// Layer IV — Archival
pub mod integrity_auditor;   // VAULT-13
pub mod cold_storage;        // VAULT-14
pub mod purge;               // VAULT-15
pub mod audit_log;           // VAULT-16
