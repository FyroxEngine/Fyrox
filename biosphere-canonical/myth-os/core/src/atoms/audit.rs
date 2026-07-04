// CORE-ATOM-16: Audit Trail Manager — immutable system transition log
use chrono::Utc;
use mythos::identity::MythId;
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Serialize)]
struct SystemEvent {
    at: String,
    source: String,
    event: String,
}

pub struct CoreAudit {
    file: Arc<Mutex<File>>,
}

impl CoreAudit {
    pub fn open(dir: impl AsRef<Path>) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir.as_ref())?;
        let path = dir.as_ref().join("core_audit.ndjson");
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self { file: Arc::new(Mutex::new(file)) })
    }

    pub fn log(&self, source: &MythId, event: impl Into<String>) {
        let entry = SystemEvent {
            at: Utc::now().to_rfc3339(),
            source: source.as_str(),
            event: event.into(),
        };
        if let Ok(mut line) = serde_json::to_string(&entry) {
            line.push('\n');
            let _ = self.file.lock().unwrap().write_all(line.as_bytes());
        }
    }
}
