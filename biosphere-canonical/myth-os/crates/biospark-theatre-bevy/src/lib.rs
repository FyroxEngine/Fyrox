// biospark-theatre-bevy — Phase 6: Glyph Vault read/write, library panel
//
// All five layer types have adapter implementations:
//   Phase 2 → BV  (Bevy ECS scene, spinning cubes)
//   Phase 3 → P5  (background thread → FrameBuffer → textured quad, mock renderer)
//   Phase 3 → HT  (background thread → FrameBuffer → textured quad, mock renderer)
//   Phase 4 → GL  (GlMaterial custom WGSL shader, beat-driven uniforms)
//   Phase 5 → AU  (cpal output stream, additive drone synth, beat-sync'd)
//   Phase 6 → Glyph Vault (file-backed JSON store, egui library panel, assign-to-channel)
//
// Phase 7: LLM glyph generation (prompt → code → Vault → channel)
//
// Run: cargo run -p biospark-theatre-bevy --bin theatre

pub mod bv_layer;
pub mod compositor;
pub mod glyph_vault;
pub mod layers;
pub mod rack_ui;
pub mod theatre_app;

pub use glyph_vault::{GlyphLibrary, GlyphStore, GlyphVaultPlugin, TheatreUiState};
pub use layers::{
    ActiveAuLayers, AuLayer, AuLayerPlugin, AuShared,
    FrameBuffer,
    GlLayerPlugin, GlMaterial, GlQuadConfig, GlUniforms,
    WebViewLayer,
};
pub use theatre_app::TheatreApp;
