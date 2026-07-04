// THEATRE-GLYPH-VAULT: File-backed glyph library + all egui panel / mixer state.
//
// GlyphStore      — reads/writes GlyphPreset JSON to data/theatre/glyphs/
// GlyphLibrary    — Bevy Resource: in-memory Vec<GlyphPreset> + backing store
// MixerScene      — snapshot of all channel levels/mutes + BPM (scene A-D)
// TheatreUiState  — Bevy Resource: ALL egui panel state for the full session
// GlyphVaultPlugin — inserts both resources at app startup

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::prelude::*;
use biospark_theatre::{GlyphPreset, LayerType};
use myth_wire::ChannelId;

// ── GlyphStore ────────────────────────────────────────────────────────────────

pub struct GlyphStore {
    pub dir: PathBuf,
}

impl GlyphStore {
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, String> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("GlyphStore: cannot create {}: {e}", dir.display()))?;
        Ok(Self { dir })
    }

    pub fn load_all(&self) -> Vec<GlyphPreset> {
        let mut glyphs = Vec::new();
        let rd = match std::fs::read_dir(&self.dir) {
            Ok(r)  => r,
            Err(e) => {
                tracing::warn!("GlyphStore: cannot read {}: {e}", self.dir.display());
                return glyphs;
            }
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") { continue; }
            match std::fs::read_to_string(&path) {
                Ok(raw) => match serde_json::from_str::<GlyphPreset>(&raw) {
                    Ok(g)  => glyphs.push(g),
                    Err(e) => tracing::warn!("GlyphStore: skipping {}: {e}", path.display()),
                },
                Err(e) => tracing::warn!("GlyphStore: cannot read {}: {e}", path.display()),
            }
        }
        glyphs.sort_by_key(|g| g.created_at);
        glyphs
    }

    pub fn save(&self, glyph: &GlyphPreset) -> Result<(), String> {
        let path = self.dir.join(format!("{}.json", glyph.id));
        let json = serde_json::to_string_pretty(glyph)
            .map_err(|e| format!("GlyphStore: serialize: {e}"))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("GlyphStore: write {}: {e}", path.display()))
    }
}

// ── GlyphLibrary ──────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct GlyphLibrary {
    pub glyphs: Vec<GlyphPreset>,
    store:      GlyphStore,
}

impl GlyphLibrary {
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, String> {
        let store  = GlyphStore::open(dir)?;
        let glyphs = store.load_all();
        tracing::info!("GlyphLibrary: loaded {} glyph(s)", glyphs.len());
        Ok(Self { glyphs, store })
    }

    pub fn degraded() -> Self {
        Self { glyphs: Vec::new(), store: GlyphStore { dir: PathBuf::from(GLYPH_DIR) } }
    }

    pub fn add_and_save(&mut self, glyph: GlyphPreset) {
        if let Err(e) = self.store.save(&glyph) { tracing::error!("{e}"); }
        self.glyphs.push(glyph);
    }
}

// ── MixerScene ────────────────────────────────────────────────────────────────

/// A complete snapshot of mixer state — stored in scene slots A-D.
///
/// Capturing: per-channel user-intended level, per-channel user mute
/// preference, and BPM. Playing state is NOT captured (stop/start is
/// a performance decision, not a scene).
#[derive(Clone, Debug)]
pub struct MixerScene {
    pub bpm:      f32,
    /// (channel_id, user_level [0-1], user_muted)
    pub channels: Vec<(ChannelId, f32, bool)>,
}

// ── TheatreUiState ────────────────────────────────────────────────────────────

/// Persistent egui + mixer state (Bevy Resource, full app lifetime).
///
/// Covers:
///   - Glyph library panel (Phase 6)
///   - Channel mixing board (Phase 7)
///   - Phase 8 stub fields (MIDI/OSC ready: currently unused)
#[derive(Resource)]
pub struct TheatreUiState {
    // ── Panel toggles ─────────────────────────────────────────────────────────
    pub library_open:  bool,
    pub mixer_open:    bool,

    // ── Glyph library ─────────────────────────────────────────────────────────
    pub selected_channel: Option<ChannelId>,
    pub selected_glyph:   Option<usize>,
    pub new_name:         String,
    pub new_layer:        LayerType,
    pub new_code:         String,
    pub new_form_open:    bool,

    // ── Channel mixer ─────────────────────────────────────────────────────────
    /// User-intended fader levels (0-1). Effective mixer level = level * master_level.
    pub channel_levels:  HashMap<ChannelId, f32>,
    /// User's explicit per-channel mute preference (NOT the solo-overridden state).
    pub user_mutes:      HashMap<ChannelId, bool>,
    /// If Some, only this channel plays; all others are effectively muted.
    pub solo_channel:    Option<ChannelId>,
    /// Global level multiplier applied on top of per-channel faders.
    pub master_level:    f32,
    /// Silences ALL audio (AU) channels without touching their fader levels.
    pub master_muted:    bool,

    // ── Transport ─────────────────────────────────────────────────────────────
    /// Raw tap timestamps for tap-tempo averaging (cleared after 3 s gap).
    pub tap_times: Vec<Instant>,

    // ── Scene snapshots (A-D) ─────────────────────────────────────────────────
    pub scenes: [Option<MixerScene>; 4],

    // ── Phase 8 stubs (MIDI / Ableton Link) ──────────────────────────────────
    /// True when a MIDI clock source is connected and driving the beat.
    /// Phase 8 sets this when `midir` receives clock pulses.
    pub midi_sync_active: bool,
    /// True when Ableton Link is connected and driving BPM + phase.
    /// Phase 8 sets this when `rusty_link` joins a Link session.
    pub link_sync_active: bool,
}

impl Default for TheatreUiState {
    fn default() -> Self {
        Self {
            library_open:  false,
            mixer_open:    false,
            selected_channel: None,
            selected_glyph:   None,
            new_name:  String::new(),
            new_layer: LayerType::P5,
            new_code:  String::new(),
            new_form_open: false,
            channel_levels:  HashMap::new(),
            user_mutes:      HashMap::new(),
            solo_channel:    None,
            master_level:    1.0,
            master_muted:    false,
            tap_times:       Vec::new(),
            scenes:          [None, None, None, None],
            midi_sync_active: false,
            link_sync_active: false,
        }
    }
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub(crate) const GLYPH_DIR: &str = "data/theatre/glyphs";

pub struct GlyphVaultPlugin;

impl Plugin for GlyphVaultPlugin {
    fn build(&self, app: &mut App) {
        let lib = GlyphLibrary::open(GLYPH_DIR)
            .unwrap_or_else(|e| {
                tracing::error!("{e} — running with empty (degraded) glyph library");
                GlyphLibrary::degraded()
            });
        app.insert_resource(lib)
            .insert_resource(TheatreUiState::default());
    }
}
