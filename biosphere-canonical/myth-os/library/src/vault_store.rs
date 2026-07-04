use bevy::prelude::*;
use biospark_theatre::LayoutBlueprint;
use chrono::Utc;
use egui::Color32;
use mythos::{
    capsule::{Capsule, CapsuleMeta, CapsuleKind},
    identity::MythId,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};
use uuid::Uuid;
use vault::VaultRegistry;

// ── Vault type ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VaultType {
    Audio,
    #[default]
    TwoD,
    ThreeD,
    Hybrid,
    /// Master Stage vault — a Theatre compositor as a vault.
    Stage,
}

impl VaultType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Audio  => "AUDIO",
            Self::TwoD   => "2D",
            Self::ThreeD => "3D",
            Self::Hybrid => "HYBRID",
            Self::Stage  => "STAGE",
        }
    }

    pub fn tagline(&self) -> &'static str {
        match self {
            Self::Audio  => "Music, sound design, synthesis",
            Self::TwoD   => "Graphics, illustration, motion design",
            Self::ThreeD => "Modeling, animation, rendering",
            Self::Hybrid => "2D + 3D combined workspace",
            Self::Stage  => "BioSpark Theatre — composite renderer & master mixer",
        }
    }

    /// Plugins pre-installed when a vault of this type is created.
    pub fn default_plugins(&self) -> Vec<String> {
        match self {
            Self::Audio  => vec![
                "composer.player".into(),
                "composer.studio".into(),
                "animus.rhythm".into(),
                "core.signal".into(),
            ],
            Self::TwoD   => vec![
                "quill.scrolls".into(),
                "prism.canvas".into(),
                "loom.board".into(),
                "chronicle.log".into(),
            ],
            Self::ThreeD => vec![
                "forge.blueprints".into(),
                "atlas.space".into(),
                "architect.structure".into(),
                "chronicle.log".into(),
            ],
            Self::Hybrid => vec![
                "quill.scrolls".into(),
                "prism.canvas".into(),
                "forge.blueprints".into(),
                "atlas.space".into(),
                "composer.player".into(),
                "chronicle.log".into(),
            ],
            Self::Stage => vec![
                "theatre.stage".into(),
                "theatre.mixer".into(),
                "genesis.forge".into(),
                "loom.board".into(),
            ],
        }
    }
}

// ── Vault status ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaultStatus {
    Active,
    /// Transition state: vault is being prepared for sealing. No structural changes allowed.
    Sealing,
    Sealed,
    Archived,
}

impl VaultStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active   => "ACTIVE",
            Self::Sealing  => "SEALING",
            Self::Sealed   => "SEALED",
            Self::Archived => "ARCHIVED",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::Active   => Color32::from_rgb(0,   192, 96),
            Self::Sealing  => Color32::from_rgb(255, 180,  0),
            Self::Sealed   => Color32::from_rgb(212, 160, 48),
            Self::Archived => Color32::from_rgb(100, 116, 139),
        }
    }
}

// ── Vault metadata ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct VaultMeta {
    pub id:             Uuid,
    pub name:           String,
    pub description:    String,
    pub color:          Color32,
    pub status:         VaultStatus,
    pub vault_type:     VaultType,
    pub is_protected:   bool,
    /// Plugin IDs active in this vault.
    pub plugins:        Vec<String>,
    /// BLAKE3 hex fingerprint of this vault's identity seed.
    /// 64-char hex string. Empty string = not yet assigned (legacy vaults).
    pub bdna_signature: String,
    /// The last plugin the user had open in this vault. Restored on re-entry.
    pub active_plugin:  Option<String>,
    /// Base resonance frequency for this vault in Hz.
    /// Derived from the aura colour chosen at genesis. Default 432.0 (Verdi).
    pub resonance_hz:   f64,
    /// Panel-partition blueprint chosen at vault creation or in the Stage plugin.
    /// Controls how Theatre channels are laid out on the canvas.
    pub layout_blueprint: LayoutBlueprint,
}

// ── B-DNA generation ──────────────────────────────────────────────────────────

/// Generate a deterministic B-DNA signature from a vault's identity.
/// `with_time = true` mixes in the current timestamp for uniqueness (new vaults).
/// `with_time = false` uses only id + name, giving a deterministic result (seeds).
pub fn generate_bdna(id: Uuid, name: &str, with_time: bool) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(id.as_bytes());
    hasher.update(name.as_bytes());
    if with_time {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        hasher.update(&ts.to_le_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

/// Expand the first 8 bytes of the B-DNA signature into a `[bool; 64]` array.
/// Returns all-false if the signature is empty or malformed.
pub fn bdna_to_bits(signature: &str) -> [bool; 64] {
    let mut bits = [false; 64];
    let bytes: Vec<u8> = (0..signature.len())
        .step_by(2)
        .take(8)
        .filter_map(|i| u8::from_str_radix(&signature[i..i + 2], 16).ok())
        .collect();
    for (byte_idx, byte) in bytes.iter().enumerate() {
        for bit_idx in 0..8 {
            bits[byte_idx * 8 + bit_idx] = (byte >> bit_idx) & 1 == 1;
        }
    }
    bits
}

// ── Integrity hashing ─────────────────────────────────────────────────────────

/// Compute a BLAKE3 integrity hash for a stored vault.
/// Hashes all stable fields (excludes `integrity_hash` itself).
fn compute_integrity_hash(s: &StoredVault) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(s.id.as_bytes());
    hasher.update(s.name.as_bytes());
    hasher.update(s.description.as_bytes());
    hasher.update(&s.color_rgba);
    hasher.update(s.status.label().as_bytes());
    hasher.update(s.vault_type.label().as_bytes());
    hasher.update(if s.is_protected { &[1u8] } else { &[0u8] });
    for p in &s.plugins {
        hasher.update(p.as_bytes());
    }
    hasher.update(s.bdna_signature.as_bytes());
    hasher.update(&s.resonance_hz.to_le_bytes());
    hasher.update(s.layout_blueprint.id().as_bytes());
    hasher.finalize().to_hex().to_string()
}

// ── Serde mirror ──────────────────────────────────────────────────────────────

fn default_resonance_hz() -> f64 { 432.0 }

/// Serde-friendly mirror of `VaultMeta`.
/// `Color32` is stored as `[r, g, b, a]` bytes so we don't need egui's serde feature.
/// New fields use `#[serde(default)]` for backwards-compatible loading of older files.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredVault {
    id:           Uuid,
    name:         String,
    description:  String,
    color_rgba:   [u8; 4],
    status:       VaultStatus,
    vault_type:   VaultType,
    is_protected: bool,
    plugins:      Vec<String>,

    /// BLAKE3 hex fingerprint — empty string means "not yet generated".
    #[serde(default)]
    bdna_signature: String,

    /// Last active plugin id — None if no plugin was active.
    #[serde(default)]
    active_plugin: Option<String>,

    /// Base resonance frequency in Hz. Default 432.0 for legacy vaults.
    #[serde(default = "default_resonance_hz")]
    resonance_hz: f64,

    /// Integrity hash — verified on load. Empty = legacy vault (no hash yet).
    #[serde(default)]
    integrity_hash: String,

    /// Panel-partition blueprint. Defaults to Fullscreen for legacy vaults.
    #[serde(default)]
    layout_blueprint: LayoutBlueprint,
}

impl From<&VaultMeta> for StoredVault {
    fn from(v: &VaultMeta) -> Self {
        Self {
            id:             v.id,
            name:           v.name.clone(),
            description:    v.description.clone(),
            color_rgba:     [v.color.r(), v.color.g(), v.color.b(), v.color.a()],
            status:         v.status.clone(),
            vault_type:     v.vault_type.clone(),
            is_protected:   v.is_protected,
            plugins:        v.plugins.clone(),
            bdna_signature: v.bdna_signature.clone(),
            active_plugin:  v.active_plugin.clone(),
            resonance_hz:     v.resonance_hz,
            layout_blueprint: v.layout_blueprint,
            integrity_hash:   String::new(), // filled in by save()
        }
    }
}

impl From<StoredVault> for VaultMeta {
    fn from(s: StoredVault) -> Self {
        Self {
            id:             s.id,
            name:           s.name,
            description:    s.description,
            color:          Color32::from_rgba_unmultiplied(
                                s.color_rgba[0], s.color_rgba[1],
                                s.color_rgba[2], s.color_rgba[3]),
            status:         s.status,
            vault_type:     s.vault_type,
            is_protected:   s.is_protected,
            plugins:        s.plugins,
            bdna_signature:   s.bdna_signature,
            active_plugin:    s.active_plugin,
            resonance_hz:     s.resonance_hz,
            layout_blueprint: s.layout_blueprint,
        }
    }
}

// ── Vault store ───────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct VaultStore {
    pub vaults: Vec<VaultMeta>,
}

impl VaultStore {
    pub fn by_id(&self, id: Uuid) -> Option<&VaultMeta> {
        self.vaults.iter().find(|v| v.id == id)
    }

    pub fn by_id_mut(&mut self, id: Uuid) -> Option<&mut VaultMeta> {
        self.vaults.iter_mut().find(|v| v.id == id)
    }

    pub fn add(&mut self, meta: VaultMeta) {
        self.vaults.push(meta);
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    fn json_path() -> PathBuf { PathBuf::from("data/library/vaults.json") }
    fn registry_root() -> &'static str { "data/vault" }

    /// Persist all vaults.
    ///
    /// Primary storage: `data/vault/` VaultRegistry (mmap blobs, BLAKE3, audit log,
    /// version history).  Each vault is stored as a `Capsule(Vault)` keyed by its UUID.
    ///
    /// Human-readable companion: `data/library/vaults.json` — auto-generated on every
    /// save so users and external tools can read vault metadata without touching the
    /// binary blob store.
    pub fn save(&self) -> anyhow::Result<()> {
        let registry = VaultRegistry::open(Self::registry_root())?;

        for vault in &self.vaults {
            let mut stored = StoredVault::from(vault);
            if stored.bdna_signature.is_empty() {
                stored.bdna_signature = generate_bdna(vault.id, &vault.name, false);
            }
            stored.integrity_hash = compute_integrity_hash(&stored);

            let payload = serde_json::to_vec_pretty(&stored)?;
            let myth_id = MythId::from_uuid(vault.id);

            // Build the Capsule
            let capsule = Capsule {
                id:             myth_id.clone(),
                fingerprint:    None, // set by ingest()
                kind:           CapsuleKind::Vault,
                schema_version: 1,
                created_at:     Utc::now(),
                payload:        payload.clone(),
                metadata:       CapsuleMeta {
                    name:         vault.name.clone(),
                    tags:         vec!["vault".into()],
                    origin_vault: None,
                    parent:       None,
                },
            };

            // ingest(): BLAKE3 fingerprint → dedup check → write page → audit log entry
            registry.ingest(&capsule)?;

            // Record a version delta (in-memory; cross-session history is in the blobs)
            registry.versioning.commit(&myth_id, payload);
        }

        // Human-readable companion (atomic write)
        let stored_all: Vec<StoredVault> = self.vaults.iter().map(|v| {
            let mut s = StoredVault::from(v);
            if s.bdna_signature.is_empty() {
                s.bdna_signature = generate_bdna(v.id, &v.name, false);
            }
            s.integrity_hash = compute_integrity_hash(&s);
            s
        }).collect();

        let json_path = Self::json_path();
        if let Some(parent) = json_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&stored_all)?;
        let tmp = json_path.with_extension("json.tmp");
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &json_path)?;

        Ok(())
    }

    /// Load vaults from disk.
    ///
    /// Priority:
    ///   1. VaultRegistry (`data/vault/`) — the authoritative binary store.
    ///   2. JSON companion (`data/library/vaults.json`) — migration path from older installs.
    ///   3. Seed data — first run.
    ///
    /// If the registry is empty but JSON exists, the JSON vaults are migrated into the
    /// registry automatically on the first save.
    pub fn load() -> Self {
        // ── 1. Try VaultRegistry ──────────────────────────────────────────
        if let Ok(registry) = VaultRegistry::open(Self::registry_root()) {
            let ids = registry.list_ids();
            if !ids.is_empty() {
                let mut vaults: Vec<VaultMeta> = Vec::with_capacity(ids.len());
                for id_str in &ids {
                    let Ok(uuid) = Uuid::parse_str(id_str) else { continue };
                    let myth_id = MythId::from_uuid(uuid);
                    match registry.fetch(&myth_id) {
                        Ok(bytes) => match serde_json::from_slice::<StoredVault>(&bytes) {
                            Ok(stored) => {
                                // Background integrity audit (Atom 13)
                                if !stored.integrity_hash.is_empty() {
                                    let expected = compute_integrity_hash(&stored);
                                    if stored.integrity_hash != expected {
                                        warn!(
                                            vault = %stored.name,
                                            "Integrity hash mismatch — blob may have been modified"
                                        );
                                    }
                                }
                                vaults.push(VaultMeta::from(stored));
                            }
                            Err(e) => warn!("Failed to deserialize vault {uuid}: {e}"),
                        },
                        Err(e) => warn!("Failed to fetch vault blob {uuid}: {e}"),
                    }
                }
                if !vaults.is_empty() {
                    info!("Loaded {} vault(s) from VaultRegistry", vaults.len());
                    return Self { vaults };
                }
            }
        }

        // ── 2. Fall back to JSON (migration path) ─────────────────────────
        let json_path = Self::json_path();
        if json_path.exists() {
            match std::fs::read_to_string(&json_path)
                .map_err(anyhow::Error::from)
                .and_then(|s| serde_json::from_str::<Vec<StoredVault>>(&s).map_err(Into::into))
            {
                Ok(stored) if !stored.is_empty() => {
                    for s in &stored {
                        if !s.integrity_hash.is_empty() {
                            let expected = compute_integrity_hash(s);
                            if s.integrity_hash != expected {
                                warn!(
                                    vault = %s.name,
                                    "JSON integrity mismatch — vault may have been edited externally"
                                );
                            }
                        }
                    }
                    let vaults: Vec<VaultMeta> = stored.into_iter().map(VaultMeta::from).collect();
                    info!("Migrating {} vault(s) from JSON → VaultRegistry", vaults.len());
                    let store = Self { vaults };
                    // Migrate immediately so the registry is canonical going forward
                    if let Err(e) = store.save() {
                        warn!("Migration save failed: {e}");
                    }
                    return store;
                }
                Ok(_) => {}
                Err(e) => warn!("Could not load vault JSON ({e}), seeding defaults"),
            }
        }

        // ── 3. Seeds (first run) ──────────────────────────────────────────
        let store = Self { vaults: Self::seeds() };
        if let Err(e) = store.save() {
            warn!("Seed save failed: {e}");
        }
        store
    }

    // ── Export / Import ───────────────────────────────────────────────────────

    /// Serialize a single vault to a self-contained `.qgenesis` JSON string.
    pub fn export_vault(&self, id: Uuid) -> anyhow::Result<String> {
        let vault = self.by_id(id).ok_or_else(|| anyhow::anyhow!("Vault not found: {}", id))?;
        let mut stored = StoredVault::from(vault);
        if stored.bdna_signature.is_empty() {
            stored.bdna_signature = generate_bdna(vault.id, &vault.name, false);
        }
        stored.integrity_hash = compute_integrity_hash(&stored);
        let json = serde_json::to_string_pretty(&stored)?;
        Ok(json)
    }

    /// Load a vault from a `.qgenesis` JSON string and add it to this store.
    /// Returns the new vault's `Uuid` on success.
    /// Warns (but does not reject) if the integrity hash is mismatched.
    /// Returns `Err` if the vault already exists in the store.
    pub fn import_vault(&mut self, json: &str) -> anyhow::Result<Uuid> {
        let stored: StoredVault = serde_json::from_str(json)?;

        // Verify integrity hash if present
        if !stored.integrity_hash.is_empty() {
            let expected = compute_integrity_hash(&stored);
            if stored.integrity_hash != expected {
                warn!(vault = %stored.name, "Imported vault has a hash mismatch — proceeding with caution");
            }
        }

        // Don't import duplicates
        let id = stored.id;
        if self.by_id(id).is_some() {
            return Err(anyhow::anyhow!("Vault '{}' ({}) is already in the Library", stored.name, id));
        }

        self.vaults.push(VaultMeta::from(stored));
        Ok(id)
    }

    // ── Seeds ─────────────────────────────────────────────────────────────────

    fn seeds() -> Vec<VaultMeta> {
        vec![
            VaultMeta {
                id:               Uuid::new_v4(),
                name:             "Master Vault".into(),
                description:      "The root container. Everything begins here.".into(),
                color:            Color32::from_rgb(212, 160,  48),
                status:           VaultStatus::Active,
                vault_type:       VaultType::Hybrid,
                is_protected:     false,
                plugins:          VaultType::Hybrid.default_plugins(),
                bdna_signature:   String::new(),
                active_plugin:    None,
                resonance_hz:     432.0,
                layout_blueprint: LayoutBlueprint::Fullscreen,
            },
            VaultMeta {
                id:               Uuid::new_v4(),
                name:             "Echo Archive".into(),
                description:      "Sealed knowledge. Resonance from prior seals.".into(),
                color:            Color32::from_rgb(140,  80, 255),
                status:           VaultStatus::Active,
                vault_type:       VaultType::TwoD,
                is_protected:     false,
                plugins:          VaultType::TwoD.default_plugins(),
                bdna_signature:   String::new(),
                active_plugin:    None,
                resonance_hz:     528.0,
                layout_blueprint: LayoutBlueprint::SidebarLeft,
            },
            VaultMeta {
                id:               Uuid::new_v4(),
                name:             "Signal Chamber".into(),
                description:      "Pure coherence engine. Energy seeking escape.".into(),
                color:            Color32::from_rgb(0,   200, 180),
                status:           VaultStatus::Active,
                vault_type:       VaultType::ThreeD,
                is_protected:     false,
                plugins:          VaultType::ThreeD.default_plugins(),
                bdna_signature:   String::new(),
                active_plugin:    None,
                resonance_hz:     396.0,
                layout_blueprint: LayoutBlueprint::HolyGrail,
            },
            VaultMeta {
                id:               Uuid::new_v4(),
                name:             "Master Stage".into(),
                description:      "The Theatre. All layers composited. All channels live.".into(),
                color:            Color32::from_rgb(224, 216, 255),
                status:           VaultStatus::Active,
                vault_type:       VaultType::Stage,
                is_protected:     false,
                plugins:          VaultType::Stage.default_plugins(),
                bdna_signature:   String::new(),
                active_plugin:    Some("theatre.mixer".into()),
                resonance_hz:     528.0,
                layout_blueprint: LayoutBlueprint::Fullscreen,
            },
        ]
    }
}

// ── Library prefs ─────────────────────────────────────────────────────────────

/// Lightweight persistent preferences for the Library app.
/// Stored separately from the vault store — changes don't trigger a full vault save.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LibraryPrefs {
    /// UUID of the last vault the user had open. Restored on next launch.
    #[serde(default)]
    pub last_vault_id: Option<Uuid>,
}

fn prefs_path() -> PathBuf { PathBuf::from("data/library/prefs.json") }

pub fn load_prefs() -> LibraryPrefs {
    let path = prefs_path();
    if !path.exists() { return LibraryPrefs::default(); }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_prefs(prefs: &LibraryPrefs) {
    let path = prefs_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(prefs) {
        let _ = std::fs::write(path, json);
    }
}

// ── Runtime selection ─────────────────────────────────────────────────────────

/// Which vault is currently open. None = at the library landing.
#[derive(Resource, Default)]
pub struct SelectedVault(pub Option<Uuid>);

/// Prevents the "restore last vault" OnEnter(Landing) system from firing more
/// than once per session (so navigating back to Landing doesn't re-redirect).
#[derive(Resource, Default)]
pub struct InitialRestoreDone(pub bool);

// ── New-vault wizard state ────────────────────────────────────────────────────

#[derive(Resource)]
pub struct SetupDraft {
    pub step:             u8,
    pub vault_type:       Option<VaultType>,
    pub name:             String,
    pub description:      String,
    pub color_idx:        usize,
    pub layout_blueprint: LayoutBlueprint,
}

impl Default for SetupDraft {
    fn default() -> Self {
        Self {
            step:             0,
            vault_type:       None,
            name:             String::new(),
            description:      String::new(),
            color_idx:        0,
            layout_blueprint: LayoutBlueprint::Fullscreen,
        }
    }
}

impl SetupDraft {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn chosen_color(&self) -> Color32 {
        AURA_PALETTE.get(self.color_idx)
            .map(|&(c, _, _)| c)
            .unwrap_or(Color32::from_rgb(212, 160, 48))
    }

    pub fn chosen_resonance_hz(&self) -> f64 {
        AURA_PALETTE.get(self.color_idx)
            .map(|&(_, _, hz)| hz)
            .unwrap_or(432.0)
    }
}

/// Preset aura colours for the vault setup wizard.
/// Tuple layout: (Color32, display_name, resonance_hz).
/// Hz values follow the Solfeggio frequency scale and standard tuning references.
pub const AURA_PALETTE: &[(Color32, &str, f64)] = &[
    (Color32::from_rgb(212, 160,  48), "GOLD",    432.0),  // Verdi tuning
    (Color32::from_rgb(140,  80, 255), "VIOLET",  528.0),  // Transformation
    (Color32::from_rgb(  0, 200, 180), "TEAL",    396.0),  // Liberation
    (Color32::from_rgb(220, 140,  30), "AMBER",   440.0),  // Concert A4
    (Color32::from_rgb(220,  60, 120), "CRIMSON", 639.0),  // Connection
    (Color32::from_rgb( 30, 140, 255), "AZURE",   741.0),  // Intuition
    (Color32::from_rgb(  0, 192,  96), "EMERALD", 285.0),  // Cognition
    (Color32::from_rgb(255, 100,  30), "FORGE",   174.0),  // Foundation
];

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct VaultStorePlugin;

impl Plugin for VaultStorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(VaultStore::load())
           .init_resource::<SelectedVault>()
           .init_resource::<SetupDraft>()
           .init_resource::<InitialRestoreDone>()
           .add_systems(Update, (persist_on_change, persist_selected_vault));
    }
}

/// Auto-saves the vault store to VaultRegistry (+ JSON companion) whenever it is mutated.
fn persist_on_change(store: Res<VaultStore>) {
    if store.is_changed() {
        if let Err(e) = store.save() {
            warn!("Vault store save failed: {e}");
        }
    }
}

/// Persists the selected vault ID to prefs whenever the selection changes.
fn persist_selected_vault(selected: Res<SelectedVault>) {
    if selected.is_changed() {
        save_prefs(&LibraryPrefs { last_vault_id: selected.0 });
    }
}
