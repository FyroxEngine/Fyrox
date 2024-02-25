//! Renderer module.
//!
//! # Overview
//!
//! Renderer processes samples from each sound source before they'll be passed to output device. Exact
//! behaviour of renderer depends of variant being used.

#![allow(clippy::float_cmp)]

use crate::{
    context::DistanceModel, listener::Listener, math, renderer::hrtf::HrtfRenderer,
    source::SoundSource,
};
use fyrox_core::math::lerpf;
use fyrox_core::{
    reflect::prelude::*,
    uuid_provider,
    visitor::{Visit, VisitResult, Visitor},
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

pub mod hrtf;

/// See module docs.
// This "large size difference" is not a problem because renderer
// can be only one at a time on context.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, AsRefStr, EnumString, VariantNames, Visit, Reflect)]
pub enum Renderer {
    /// Stateless default renderer.
    Default,

    /// Can be used *only* with mono sounds, stereo sounds will be rendered through
    /// default renderer.
    HrtfRenderer(HrtfRenderer),
}

uuid_provider!(Renderer = "13bf8432-987a-4216-b6aa-f5c0e8914a31");

impl Default for Renderer {
    fn default() -> Self {
        Self::Default
    }
}

fn render_with_params(
    source: &mut SoundSource,
    left_gain: f32,
    right_gain: f32,
    mix_buffer: &mut [(f32, f32)],
) {
    let last_left_gain = *source.last_left_gain.get_or_insert(left_gain);
    let last_right_gain = *source.last_right_gain.get_or_insert(right_gain);

    if last_left_gain != left_gain || last_right_gain != right_gain {
        let step = 1.0 / mix_buffer.len() as f32;
        let mut t = 0.0;
        for ((out_left, out_right), &(raw_left, raw_right)) in
            mix_buffer.iter_mut().zip(source.frame_samples())
        {
            // Interpolation of gain is very important to remove clicks which appears
            // when gain changes by significant value between frames.
            *out_left += math::lerpf(last_left_gain, left_gain, t) * raw_left;
            *out_right += math::lerpf(last_right_gain, right_gain, t) * raw_right;

            t += step;
        }
    } else {
        for ((out_left, out_right), &(raw_left, raw_right)) in
            mix_buffer.iter_mut().zip(source.frame_samples())
        {
            // Optimize the common case when the gain did not change since the last call.
            *out_left += left_gain * raw_left;
            *out_right += right_gain * raw_right;
        }
    }
}

pub(crate) fn render_source_default(
    source: &mut SoundSource,
    listener: &Listener,
    distance_model: DistanceModel,
    mix_buffer: &mut [(f32, f32)],
) {
    let distance_gain = lerpf(
        1.0,
        source.calculate_distance_gain(listener, distance_model),
        source.spatial_blend(),
    );
    let panning = lerpf(
        source.panning(),
        source.calculate_panning(listener),
        source.spatial_blend(),
    );
    let gain = distance_gain * source.gain();
    let left_gain = gain * (1.0 + panning);
    let right_gain = gain * (1.0 - panning);
    render_with_params(source, left_gain, right_gain, mix_buffer);
    source.last_left_gain = Some(left_gain);
    source.last_right_gain = Some(right_gain);
}

pub(crate) fn render_source_2d_only(source: &mut SoundSource, mix_buffer: &mut [(f32, f32)]) {
    let gain = (1.0 - source.spatial_blend()) * source.gain();
    let left_gain = gain * (1.0 + source.panning());
    let right_gain = gain * (1.0 - source.panning());
    render_with_params(source, left_gain, right_gain, mix_buffer);
    source.last_left_gain = Some(left_gain);
    source.last_right_gain = Some(right_gain);
}
