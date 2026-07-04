// THEATRE-LAYER: Layer types and the OutputHandler trait.
//
// Every layer type (P5, GL, BV, HT, AU) implements OutputHandler.
// The compositor calls render() on each active channel in z_order.

use serde::{Deserialize, Serialize};

/// The five layer types a channel can host.
///
/// Changed via the selector knob on the Channel Mixer.
/// Changing layer type on a live channel clears its loaded glyph —
/// the old glyph was written for a different renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerType {
    /// p5.js generative sketch, rendered via WebView. (Phase 3)
    P5,
    /// GLSL shader, rendered via wgpu. (Phase 4)
    Gl,
    /// Bevy ECS scene. (Phase 2)
    Bv,
    /// HTML/CSS content, rendered via WebView. (Phase 3)
    Ht,
    /// Generative audio sample via cpal. (Phase 5)
    Au,
}

impl LayerType {
    /// Short uppercase tag used in the mixer UI and glyph metadata.
    pub fn tag(&self) -> &'static str {
        match self {
            LayerType::P5 => "P5",
            LayerType::Gl => "GL",
            LayerType::Bv => "BV",
            LayerType::Ht => "HT",
            LayerType::Au => "AU",
        }
    }

    /// Whether this layer produces visual output (not audio).
    pub fn is_visual(&self) -> bool {
        !matches!(self, LayerType::Au)
    }

    /// Whether this layer produces audio output.
    pub fn is_audio(&self) -> bool {
        matches!(self, LayerType::Au)
    }
}

impl std::fmt::Display for LayerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.tag())
    }
}

/// Context passed to every OutputHandler on each compositor tick.
///
/// Carries timing from myth-core's clock atom so all layers stay in sync.
#[derive(Debug, Clone)]
pub struct FrameContext {
    /// Monotonic tick counter from myth-core's ClockSignal.
    pub tick: u64,
    /// Milliseconds elapsed since the last frame.
    pub delta_ms: f32,
    /// Canvas width in pixels.
    pub width: u32,
    /// Canvas height in pixels.
    pub height: u32,
    /// Beat position within the current bar [0.0, 1.0).
    /// Synced to QuillClock when active.
    pub beat: f32,
    /// Current tempo in BPM.
    pub tempo_bpm: f32,
}

impl Default for FrameContext {
    fn default() -> Self {
        Self {
            tick: 0,
            delta_ms: 16.67, // ~60fps
            width: 1920,
            height: 1080,
            beat: 0.0,
            tempo_bpm: 120.0,
        }
    }
}

/// Every layer type implements this trait to participate in the Theatre compositor.
///
/// Implementations are added in later phases:
///   Phase 2 → BevyLayer  (BV)
///   Phase 3 → P5Layer    (P5), HtmlLayer (HT)
///   Phase 4 → GlslLayer  (GL)
///   Phase 5 → AudioLayer (AU)
pub trait OutputHandler: Send + Sync {
    /// Which layer type this handler serves.
    fn layer_type(&self) -> LayerType;

    /// Render one frame. Called by the compositor on each tick.
    /// Returns `Err` if the layer fails — compositor logs and skips.
    fn render(&mut self, ctx: &FrameContext) -> Result<(), crate::TheatreError>;

    /// Set the opacity (visual) or volume (audio) level [0.0, 1.0].
    fn set_level(&mut self, level: f32);

    /// Set RGBA color tint, each component [0.0, 1.0].
    /// [1.0, 1.0, 1.0, 1.0] = no tint.
    fn set_tint(&mut self, tint: [f32; 4]);

    /// Suppress this layer entirely. render() is not called while muted.
    fn mute(&mut self);

    /// Resume this layer.
    fn unmute(&mut self);

    fn is_muted(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_type_tags_are_correct() {
        assert_eq!(LayerType::P5.tag(), "P5");
        assert_eq!(LayerType::Gl.tag(), "GL");
        assert_eq!(LayerType::Bv.tag(), "BV");
        assert_eq!(LayerType::Ht.tag(), "HT");
        assert_eq!(LayerType::Au.tag(), "AU");
    }

    #[test]
    fn only_au_is_audio() {
        assert!(LayerType::Au.is_audio());
        assert!(!LayerType::P5.is_audio());
        assert!(!LayerType::Gl.is_audio());
        assert!(!LayerType::Bv.is_audio());
        assert!(!LayerType::Ht.is_audio());
    }

    #[test]
    fn au_is_not_visual() {
        assert!(!LayerType::Au.is_visual());
        assert!(LayerType::P5.is_visual());
        assert!(LayerType::Bv.is_visual());
    }
}
