// GEN-CONTAINERS: .qgenesis file loader and container store.
//
// .qgenesis format (JSON):
//   { "id": "...", "name": "...", "kind": "Actor|Terrain|Audio|Script|Blueprint|Signal|Raw",
//     "schema_version": 1, "tags": [...], "description": "...",
//     "payload_b64": "<optional base64>" }

use bevy::prelude::*;
use crossbeam_channel::{bounded, Receiver, Sender};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

// ── File format ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct GenesisContainerFile {
    pub id: String,
    pub name: String,
    pub kind: String,
    #[serde(default = "default_schema")]
    pub schema_version: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: String,
    /// Optional base64-encoded payload bytes.
    pub payload_b64: Option<String>,
}

fn default_schema() -> u32 { 1 }

// ── Runtime representation ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LoadedContainer {
    pub id:          String,
    pub name:        String,
    pub kind:        String,
    pub tags:        Vec<String>,
    pub description: String,
    pub path:        PathBuf,
    pub byte_len:    usize,
}

impl LoadedContainer {
    pub fn kind_icon(&self) -> &'static str {
        match self.kind.as_str() {
            "Actor"     => "👤",
            "Terrain"   => "⛰",
            "Audio"     => "🎵",
            "Script"    => "📜",
            "Blueprint" => "📐",
            "Signal"    => "⚡",
            _           => "📦",
        }
    }

    pub fn kind_color(&self) -> egui::Color32 {
        use crate::ui::theme;
        match self.kind.as_str() {
            "Actor"     => theme::WIRE_BHV,
            "Terrain"   => theme::WIRE_SPA,
            "Audio"     => theme::WIRE_AUD,
            "Script"    => theme::WIRE_LGC,
            "Blueprint" => theme::WIRE_IDN,
            "Signal"    => theme::WIRE_EVT,
            _           => theme::FG_MUTED,
        }
    }
}

// ── Store resource ────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct ContainerStore {
    pub containers: Vec<LoadedContainer>,
}

impl ContainerStore {
    pub fn remove(&mut self, id: &str) {
        self.containers.retain(|c| c.id != id);
    }
}

// ── Load channel (file dialog runs in its own thread) ────────────────────────

#[derive(Resource)]
pub struct ContainerLoadChannel {
    pub tx: Sender<PathBuf>,
    rx:     Receiver<PathBuf>,
}

impl Default for ContainerLoadChannel {
    fn default() -> Self {
        let (tx, rx) = bounded(16);
        Self { tx, rx }
    }
}

/// Call from a UI button click — spawns thread, opens dialog, sends path back.
pub fn request_load_dialog(tx: Sender<PathBuf>) {
    std::thread::spawn(move || {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Genesis Container", &["qgenesis"])
            .set_title("Load Genesis Container")
            .pick_file()
        {
            let _ = tx.send(path);
        }
    });
}

// ── Bevy system: drain channel and parse ─────────────────────────────────────

pub fn poll_container_loads(
    channel: Res<ContainerLoadChannel>,
    mut store: ResMut<ContainerStore>,
) {
    for path in channel.rx.try_iter() {
        match parse_container(&path) {
            Ok(c) => {
                info!(name = %c.name, kind = %c.kind, "Container loaded");
                // Replace if same id already loaded
                store.containers.retain(|x| x.id != c.id);
                store.containers.push(c);
            }
            Err(e) => {
                warn!(path = %path.display(), error = %e, "Failed to load container");
            }
        }
    }
}

fn parse_container(path: &Path) -> Result<LoadedContainer, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let file: GenesisContainerFile = serde_json::from_str(&content)?;
    let byte_len = file.payload_b64
        .as_deref()
        .map(|b| b64_byte_len(b))
        .unwrap_or(0);
    Ok(LoadedContainer {
        id:          file.id,
        name:        file.name,
        kind:        file.kind,
        tags:        file.tags,
        description: file.description,
        path:        path.to_path_buf(),
        byte_len,
    })
}

/// Approximate decoded byte length from a base64 string without decoding.
fn b64_byte_len(b64: &str) -> usize {
    let trimmed = b64.trim_end_matches('=');
    trimmed.len() * 3 / 4
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct ContainerPlugin;

impl Plugin for ContainerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ContainerStore>()
            .init_resource::<ContainerLoadChannel>()
            .add_systems(Update, poll_container_loads);
    }
}
