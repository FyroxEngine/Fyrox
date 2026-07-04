// BioSpark Theatre — entry point.
//
// Phase 6: Glyph Vault — five active channels + file-backed glyph library.
//
//   Ch0  BV  — native Bevy scene (spinning cubes)              z = 0      (front)
//   Ch1  P5  — animated wave gradient (mock p5.js)             z = -0.5
//   Ch2  GL  — nebula swirl WGSL shader (Bevy Material)        z = -0.9
//   Ch3  HT  — glowing tile grid (mock HTML)                   z = -1.4   (furthest back)
//   Ch4  AU  — ambient drone synth (cpal, no visual quad)
//
//   Glyph Library: data/theatre/glyphs/  — seeded on first run.
//   ⊞ button (top-right header) opens the right-side library panel.
//   Click a channel pip (footer) to select it, then ASSIGN a glyph.
//
// Audio: A2 pedal drone + E3 fifth + A3 octave, beat-sync'd swell on downbeat.
//        If no audio device is present the theatre continues silently.
//
// TODO(phase-3-wry):   Replace WebViewLayer mock renderer → run_wry_renderer()
// TODO(phase-4-glsl):  Replace embedded nebula → user WGSL at runtime
// TODO(phase-7-llm):   Replace DroneGen → Glyph audio from myth-vault; LLM generation

use biospark_theatre::{ChannelMixer, ChannelRouter, GlyphPreset, LayerType};
use biospark_theatre_bevy::{AuLayer, TheatreApp, WebViewLayer};

fn main() {
    let mut mixer = ChannelMixer::new(16);
    let router    = ChannelRouter::new();

    // ── Channel 0: BV — native Bevy scene ────────────────────────────────────
    mixer
        .add_channel("BV Scene", LayerType::Bv)
        .expect("failed to add BV channel");

    // ── Channel 1: P5 — animated wave (render-to-texture) ────────────────────
    let p5_id = mixer
        .add_channel("P5 Wave", LayerType::P5)
        .expect("failed to add P5 channel");

    let p5_code = r#"
function setup() { createCanvas(640, 360); }
function draw() {
  background(5, 8, 20, 20);
  noFill();
  for (let i = 0; i < 8; i++) {
    let r = map(sin(frameCount * 0.02 + i), -1, 1, 40, 180);
    stroke(60 + i * 20, 20, 200 - i * 20, 160);
    strokeWeight(1.5);
    beginShape();
    for (let x = 0; x <= width; x += 8) {
      let y = height / 2 + sin((x * 0.015) + frameCount * 0.03 + i * 0.8) * r;
      vertex(x, y);
    }
    endShape();
  }
}
"#;
    let p5_layer = WebViewLayer::new(p5_id, LayerType::P5, 640, 360, p5_code.into());

    // ── Channel 2: GL — nebula swirl WGSL shader ──────────────────────────────
    let gl_id = mixer
        .add_channel("GL Nebula", LayerType::Gl)
        .expect("failed to add GL channel");

    // ── Channel 3: HT — glowing grid (render-to-texture) ─────────────────────
    let ht_id = mixer
        .add_channel("HT Grid", LayerType::Ht)
        .expect("failed to add HT channel");

    let ht_layer = WebViewLayer::new(ht_id, LayerType::Ht, 640, 360, String::new());

    // ── Channel 4: AU — ambient drone ────────────────────────────────────────
    let au_id = mixer
        .add_channel("AU Drone", LayerType::Au)
        .expect("failed to add AU channel");

    let au_layer = AuLayer::new(au_id)
        .map_err(|e| eprintln!("WARN [theatre] Audio unavailable: {e}"))
        .ok();

    // ── Starter glyph library ─────────────────────────────────────────────────
    // These are seeded once on first run (when data/theatre/glyphs/ is empty).
    // On subsequent runs the persisted JSON files are loaded instead.
    let seed_glyphs = vec![
        GlyphPreset::new_inline(
            "P5 Wave Gradient",
            LayerType::P5,
            p5_code.trim(),
        ),
        GlyphPreset::new_inline(
            "GL Nebula Swirl",
            LayerType::Gl,
            "// WGSL — domain-warped FBM nebula (see GlLayerPlugin embedded shader)",
        ),
        GlyphPreset::new_inline(
            "HT Tile Grid",
            LayerType::Ht,
            "<!-- HTML mock — teal grid rendered by Theatre's software rasteriser -->",
        ),
        GlyphPreset::new_inline(
            "AU Ambient Drone",
            LayerType::Au,
            "// cpal — A2 + E3 + A3 additive sine drone, beat-sync'd swell (see DroneGen)",
        ),
    ];

    // ── Build & run ───────────────────────────────────────────────────────────
    TheatreApp::new(mixer, router)
        .with_webview_layer(p5_layer, -0.5)   // P5 wave   — just behind BV
        .with_gl_layer(gl_id,         -0.9)   // GL nebula — between P5 and HT
        .with_webview_layer(ht_layer, -1.4)   // HT grid   — furthest back
        .with_au_layer(au_layer)              // AU drone  — no visual quad
        .with_seed_glyphs(seed_glyphs)        // Library   — seeded on first run
        .run();
}
