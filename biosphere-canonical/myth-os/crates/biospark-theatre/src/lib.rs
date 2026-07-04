// biospark-theatre — Phase 1: Foundation
//
// Traits, data model, and routing graph for the BioSpark Theatre compositor.
//
// WHAT THE THEATRE IS:
//   Pure output display — header + canvas + footer.
//   The Channel Mixer (instrument side) drives it. Nothing in this crate renders to screen yet.
//
// PHASE 1 PROVIDES:
//   LayerType          — P5 | GL | BV | HT | AU
//   OutputHandler      — trait every layer type implements (Phases 2–5 add impls)
//   FrameContext       — per-tick timing data from myth-core clock
//   GlyphPreset        — visual/behavioral preset (Vault, LLM, or Inline source)
//   TheaterChannel     — one mixer strip: fader, tint, mute, glyph drop zone
//   ChannelMixer       — 16–64 channels (expandable 16→32→64)
//   ChannelRouter      — directed signal graph with DFS cycle detection
//   TheatreError       — unified error type
//
// PHASE ROADMAP:
//   Phase 2 → BevyLayer impl (BV)
//   Phase 3 → P5Layer + HtmlLayer impls (P5, HT) via WebView
//   Phase 4 → GlslLayer impl (GL) via wgpu + NarrativeLightMixer blend modes
//   Phase 5 → AudioLayer impl (AU) via cpal + QuillClock sync
//   Phase 6 → Glyph Vault read/write, drag-to-channel
//   Phase 7 → LLM glyph generation (prompt → code → Vault → channel)
//
// Dependency rule: myth-wire + serde + chrono + thiserror + tracing only.
// No bevy, wgpu, cpal, or egui here — those are added per-phase as optional features
// or in separate crates.

pub mod channel;
pub mod glyph;
pub mod layer;
pub mod layout;
pub mod routing;

pub use channel::{ChannelMixer, MixerCapacity, TheaterChannel};
pub use glyph::{GlyphPreset, GlyphSource};
pub use layer::{FrameContext, LayerType, OutputHandler};
pub use layout::LayoutBlueprint;
pub use routing::ChannelRouter;

use thiserror::Error;

/// Unified error type for the Theatre and Channel Mixer.
#[derive(Debug, Error)]
pub enum TheatreError {
    #[error("channel {0} not found")]
    ChannelNotFound(u32),

    #[error("mixer at capacity ({0} channels) — expand first")]
    CapacityExceeded(u32),

    #[error("routing cycle detected: connecting from channel {0} would close a loop")]
    RoutingCycle(u32),

    #[error("glyph is not ready — LLM generation or Vault load is still pending")]
    GlyphPending,

    #[error("glyph layer type mismatch: channel is {channel_layer}, glyph targets {glyph_layer}")]
    GlyphLayerMismatch {
        channel_layer: LayerType,
        glyph_layer: LayerType,
    },

    #[error("layer render error: {0}")]
    Layer(String),
}
