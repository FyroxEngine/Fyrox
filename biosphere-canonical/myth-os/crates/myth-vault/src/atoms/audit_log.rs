// VAULT-ATOM-16: Audit Trail Logger — immutable append-only operation history.
//
// Every ingest, fetch, purge, and version commit is recorded here as a
// newline-delimited JSON line. The log is never truncated.
// Format: { "at": "<rfc3339>", "actor": "<myth_id>", "capsule_id": "<myth_id>", "action": "<string>" }

use chrono::{DateTime, Utc};
use myth_wire::MythId;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub at: DateTime<Utc>,
    pub actor: String,
    pub capsule_id: String,
    pub action: String,
}

pub struct AuditLogger {
    #[allow(dead_code)] // retained for future log-rotation tooling
    log_path: PathBuf,
    file: Arc<Mutex<File>>,
}

impl AuditLogger {
    pub fn open(dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let log_path = dir.as_ref().join("audit.ndjson");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        Ok(Self {
            log_path,
            file: Arc::new(Mutex::new(file)),
        })
    }

    pub fn record(&self, actor: &MythId, capsule_id: &MythId, action: impl Into<String>) {
        let entry = AuditEntry {
            at: Utc::now(),
            actor: actor.as_str(),
            capsule_id: capsule_id.as_str(),
            action: action.into(),
        };
        if let Ok(mut line) = serde_json::to_string(&entry) {
            line.push('\n');
            let _ = self.file.lock().unwrap().write_all(line.as_bytes());
        }
    }
}
