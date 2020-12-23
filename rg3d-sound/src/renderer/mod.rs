//! Renderer module.
//!
//! # Overview
//!
//! Renderer processes samples from each sound source before they'll be passed to output device. Exact
//! behaviour of renderer depends of variant being used.

use crate::{
    context::DistanceModel,
    listener::Listener,
    math,
    renderer::hrtf::HrtfRenderer,
    source::{generic::GenericSource, SoundSource},
};
use rg3d_core::visitor::{Visit, VisitResult, Visitor};

pub mod hrtf;

/// See module docs.
// This "large size difference" is not a problem because renderer
// can be only one at a time on context.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Renderer {
    /// Stateless default renderer.
    Default,

    /// Can be used *only* with mono sounds, stereo sounds will be rendered through
    /// default renderer.
    HrtfRenderer(HrtfRenderer),
}

impl Renderer {
    fn id(&self) -> u32 {
        match self {
            Renderer::Default => 0,
            Renderer::HrtfRenderer(_) => 1,
        }
    }

    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Default),
            1 => Ok(Self::HrtfRenderer(HrtfRenderer::default())),
            _ => Err(format!("Invalid renderer id {}!", id)),
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::Default
    }
}

fn render_with_params(
    source: &mut GenericSource,
    left_gain: f32,
    right_gain: f32,
    mix_buffer: &mut [(f32, f32)],
) {
    let step = 1.0 / mix_buffer.len() as f32;
    let mut t = 0.0;

    let last_left_gain = *source.last_left_gain.get_or_insert(left_gain);
    let last_right_gain = *source.last_right_gain.get_or_insert(right_gain);

    for ((out_left, out_right), &(raw_left, raw_right)) in
        mix_buffer.iter_mut().zip(source.frame_samples())
    {
        // Interpolation of gain is very important to remove clicks which appears
        // when gain changes by significant value between frames.
        *out_left += math::lerpf(last_left_gain, left_gain, t) * raw_left;
        *out_right += math::lerpf(last_right_gain, right_gain, t) * raw_right;

        t += step;
    }
}

pub(in crate) fn render_source_default(
    source: &mut SoundSource,
    listener: &Listener,
    distance_model: DistanceModel,
    mix_buffer: &mut [(f32, f32)],
) {
    match source {
        SoundSource::Generic(generic) => {
            let gain = generic.gain();
            let panning = generic.panning();
            let left_gain = gain * (1.0 + panning);
            let right_gain = gain * (1.0 - panning);
            render_with_params(generic, left_gain, right_gain, mix_buffer);
            generic.last_left_gain = Some(left_gain);
            generic.last_right_gain = Some(right_gain);
        }
        SoundSource::Spatial(spatial) => {
            let distance_gain = spatial.get_distance_gain(listener, distance_model);
            let panning = spatial.get_panning(listener);
            let gain = distance_gain * spatial.generic().gain();
            let left_gain = gain * (1.0 + panning);
            let right_gain = gain * (1.0 - panning);
            render_with_params(spatial.generic_mut(), left_gain, right_gain, mix_buffer);
            spatial.generic_mut().last_left_gain = Some(left_gain);
            spatial.generic_mut().last_right_gain = Some(right_gain);
        }
    }
}

impl Visit for Renderer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = self.id();
        id.visit("Id", visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }
        match self {
            Renderer::Default => {}
            Renderer::HrtfRenderer(hrtf) => {
                hrtf.visit("Hrtf", visitor)?;
            }
        }

        visitor.leave_region()
    }
}
