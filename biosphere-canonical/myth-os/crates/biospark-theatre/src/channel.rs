// THEATRE-CHANNEL: TheaterChannel and ChannelMixer.
//
// The Channel Mixer is the instrument side of the Theatre — a DJ mixer
// where each fader controls one compositable layer in the canvas.
//
// Capacity is always a power of 2: 16, 32, or 64.
// Channels are ordered by z_order during composition (low = back, high = front).

use crate::{glyph::GlyphPreset, layer::LayerType, layout::LayoutBlueprint, TheatreError};
use myth_wire::{ChannelId, MythId};
use serde::{Deserialize, Serialize};

/// One channel in the Channel Mixer.
///
/// Each channel controls one compositable layer in the Theatre canvas.
/// Think DJ mixer strip: fader (level), mute, color tint, layer type knob,
/// glyph drop zone, and a generative audio sample slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TheaterChannel {
    /// Numeric identity, used by the routing graph.
    pub id: ChannelId,
    pub name: String,
    /// Which layer type this channel currently renders.
    pub layer_type: LayerType,
    /// Opacity for visual layers / volume for audio [0.0, 1.0].
    /// The fader.
    pub level: f32,
    /// RGBA color tint applied on top of the layer's output.
    /// [1.0, 1.0, 1.0, 1.0] = no tint (passthrough).
    pub tint: [f32; 4],
    pub muted: bool,
    /// The glyph currently loaded on this channel.
    pub glyph: Option<GlyphPreset>,
    /// Z-order in the compositor stack. Lower = further back.
    /// Default: same as the channel's numeric ID (channels added later sit on top).
    pub z_order: u32,
    /// MythId for Vault persistence of channel state snapshots.
    pub myth_id: MythId,
    /// Which panel zone this channel renders into.
    /// Defaults to Fullscreen (occupies the entire Theatre canvas).
    pub layout_blueprint: LayoutBlueprint,
}

impl TheaterChannel {
    pub fn new(id: ChannelId, name: impl Into<String>, layer_type: LayerType) -> Self {
        Self {
            id,
            name: name.into(),
            layer_type,
            level: 1.0,
            tint: [1.0, 1.0, 1.0, 1.0],
            muted: false,
            glyph: None,
            z_order: id.get(),
            myth_id: MythId::new(),
            layout_blueprint: LayoutBlueprint::Fullscreen,
        }
    }

    /// Drop a ready glyph onto this channel.
    ///
    /// Returns `GlyphLayerMismatch` if the glyph targets a different layer type.
    /// Returns `GlyphPending` if the glyph hasn't been fulfilled yet.
    /// Use `drop_glyph_pending()` to stage LLM glyphs before fulfillment.
    pub fn drop_glyph(&mut self, glyph: GlyphPreset) -> Result<(), TheatreError> {
        if glyph.layer_type != self.layer_type {
            return Err(TheatreError::GlyphLayerMismatch {
                channel_layer: self.layer_type,
                glyph_layer: glyph.layer_type,
            });
        }
        if !glyph.is_ready() {
            return Err(TheatreError::GlyphPending);
        }
        self.glyph = Some(glyph);
        Ok(())
    }

    /// Drop a pending glyph onto this channel (e.g. while LLM is still generating).
    ///
    /// The channel will show no output until `fulfill()` is called on the glyph
    /// and the channel re-evaluates `is_active()`.
    pub fn drop_glyph_pending(&mut self, glyph: GlyphPreset) -> Result<(), TheatreError> {
        if glyph.layer_type != self.layer_type {
            return Err(TheatreError::GlyphLayerMismatch {
                channel_layer: self.layer_type,
                glyph_layer: glyph.layer_type,
            });
        }
        self.glyph = Some(glyph);
        Ok(())
    }

    /// Remove the loaded glyph without changing the layer type.
    pub fn clear_glyph(&mut self) {
        self.glyph = None;
    }

    /// Change the layer type selector knob.
    ///
    /// Clears the loaded glyph — it was written for a different renderer.
    pub fn set_layer_type(&mut self, layer_type: LayerType) {
        if self.layer_type != layer_type {
            self.glyph = None;
            self.layer_type = layer_type;
        }
    }

    /// Whether this channel contributes output to the compositor.
    ///
    /// A channel is active when: not muted, level > 0, and a ready glyph is loaded.
    pub fn is_active(&self) -> bool {
        !self.muted
            && self.level > 0.0
            && self.glyph.as_ref().is_some_and(|g| g.is_ready())
    }
}

// ── ChannelMixer ──────────────────────────────────────────────────────────────

/// Valid mixer capacities — always a power of 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MixerCapacity {
    Sixteen = 16,
    ThirtyTwo = 32,
    SixtyFour = 64,
}

impl MixerCapacity {
    fn from_u32(n: u32) -> Self {
        if n <= 16 {
            MixerCapacity::Sixteen
        } else if n <= 32 {
            MixerCapacity::ThirtyTwo
        } else {
            MixerCapacity::SixtyFour
        }
    }

    fn next(self) -> Option<Self> {
        match self {
            MixerCapacity::Sixteen => Some(MixerCapacity::ThirtyTwo),
            MixerCapacity::ThirtyTwo => Some(MixerCapacity::SixtyFour),
            MixerCapacity::SixtyFour => None,
        }
    }

    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// The Channel Mixer — the instrument side of the Theatre.
///
/// Manages 16–64 channels (always a power of 2). Each channel controls one
/// compositable layer in the Theatre canvas output.
///
/// Composition order: channels sorted by `z_order`, lowest first (back → front).
#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelMixer {
    channels: Vec<TheaterChannel>,
    capacity: MixerCapacity,
    next_id: u32,
}

impl ChannelMixer {
    /// Create a new mixer. `requested_capacity` is rounded up to 16, 32, or 64.
    pub fn new(requested_capacity: u32) -> Self {
        let capacity = MixerCapacity::from_u32(requested_capacity);
        Self {
            channels: Vec::with_capacity(capacity.as_u32() as usize),
            capacity,
            next_id: 0,
        }
    }

    /// Add a new channel. Returns its assigned `ChannelId`.
    ///
    /// Fails with `CapacityExceeded` if the mixer is full.
    /// Expand with `expand()` first (16 → 32 → 64).
    pub fn add_channel(
        &mut self,
        name: impl Into<String>,
        layer_type: LayerType,
    ) -> Result<ChannelId, TheatreError> {
        if self.channels.len() >= self.capacity.as_u32() as usize {
            return Err(TheatreError::CapacityExceeded(self.capacity.as_u32()));
        }
        let id = ChannelId::new(self.next_id);
        self.next_id += 1;
        self.channels.push(TheaterChannel::new(id, name, layer_type));
        Ok(id)
    }

    pub fn channel(&self, id: ChannelId) -> Option<&TheaterChannel> {
        self.channels.iter().find(|c| c.id == id)
    }

    pub fn channel_mut(&mut self, id: ChannelId) -> Option<&mut TheaterChannel> {
        self.channels.iter_mut().find(|c| c.id == id)
    }

    /// Active (non-muted) channels sorted by z_order, back to front.
    pub fn active_channels(&self) -> Vec<&TheaterChannel> {
        let mut active: Vec<_> = self.channels.iter().filter(|c| !c.muted).collect();
        active.sort_by_key(|c| c.z_order);
        active
    }

    /// All channels sorted by z_order, back to front.
    pub fn all_channels(&self) -> Vec<&TheaterChannel> {
        let mut all: Vec<_> = self.channels.iter().collect();
        all.sort_by_key(|c| c.z_order);
        all
    }

    pub fn len(&self) -> usize {
        self.channels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }

    pub fn capacity(&self) -> u32 {
        self.capacity.as_u32()
    }

    /// Expand the mixer to the next capacity tier (16→32, 32→64).
    ///
    /// Returns the new capacity, or `CapacityExceeded(64)` if already at maximum.
    pub fn expand(&mut self) -> Result<u32, TheatreError> {
        match self.capacity.next() {
            Some(next) => {
                self.capacity = next;
                Ok(next.as_u32())
            }
            None => Err(TheatreError::CapacityExceeded(64)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixer_defaults_to_16_channels() {
        let m = ChannelMixer::new(10);
        assert_eq!(m.capacity(), 16);
    }

    #[test]
    fn add_channels_up_to_capacity() {
        let mut m = ChannelMixer::new(16);
        for i in 0..16u32 {
            assert!(m.add_channel(format!("ch{i}"), LayerType::P5).is_ok());
        }
        assert!(m.add_channel("overflow", LayerType::P5).is_err());
    }

    #[test]
    fn expand_increases_capacity() {
        let mut m = ChannelMixer::new(16);
        let new_cap = m.expand().unwrap();
        assert_eq!(new_cap, 32);
        assert_eq!(m.capacity(), 32);
    }

    #[test]
    fn cannot_expand_past_64() {
        let mut m = ChannelMixer::new(64);
        assert!(m.expand().is_err());
    }

    #[test]
    fn channel_is_not_active_without_glyph() {
        let ch = TheaterChannel::new(ChannelId::new(0), "test", LayerType::Gl);
        assert!(!ch.is_active());
    }

    #[test]
    fn drop_glyph_rejects_wrong_layer_type() {
        let mut ch = TheaterChannel::new(ChannelId::new(0), "test", LayerType::Gl);
        let glyph = GlyphPreset::new_inline("p5-sketch", LayerType::P5, "function setup() {}");
        assert!(matches!(
            ch.drop_glyph(glyph),
            Err(TheatreError::GlyphLayerMismatch { .. })
        ));
    }

    #[test]
    fn drop_glyph_accepts_matching_layer_type() {
        let mut ch = TheaterChannel::new(ChannelId::new(0), "test", LayerType::P5);
        let glyph = GlyphPreset::new_inline("p5-sketch", LayerType::P5, "function setup() {}");
        assert!(ch.drop_glyph(glyph).is_ok());
        assert!(ch.is_active());
    }

    #[test]
    fn set_layer_type_clears_glyph() {
        let mut ch = TheaterChannel::new(ChannelId::new(0), "test", LayerType::P5);
        let glyph = GlyphPreset::new_inline("p5-sketch", LayerType::P5, "function setup() {}");
        ch.drop_glyph(glyph).unwrap();
        ch.set_layer_type(LayerType::Gl);
        assert!(ch.glyph.is_none());
        assert_eq!(ch.layer_type, LayerType::Gl);
    }

    #[test]
    fn z_order_sorts_channels_back_to_front() {
        let mut m = ChannelMixer::new(16);
        let _ = m.add_channel("bg", LayerType::Bv);
        let _ = m.add_channel("mid", LayerType::P5);
        let _ = m.add_channel("fg", LayerType::Gl);
        let channels = m.all_channels();
        assert_eq!(channels[0].name, "bg");
        assert_eq!(channels[1].name, "mid");
        assert_eq!(channels[2].name, "fg");
    }
}
