//! Renderer module.
//!
//! # Overview
//!
//! Renderer processes samples from each sound source before they'll be passed to output device. Exact
//! behaviour of renderer depends of variant being used.

use crate::{
    hrtf::HrtfRenderer,
    source::{
        Status,
        SoundSource,
        generic::GenericSource,
    },
    listener::Listener,
    context::DistanceModel,
    math,
};

/// See module docs.
// This "large size difference" is not a problem because renderer
// can be only one at a time on context.
#[allow(clippy::large_enum_variant)]
pub enum Renderer {
    /// Stateless default renderer.
    Default,

    /// Can be used *only* with mono sounds, stereo sounds will be rendered through
    /// default renderer.
    HrtfRenderer(HrtfRenderer),
}

fn render_with_params(
    source: &mut GenericSource,
    left_gain: f32,
    right_gain: f32,
    mix_buffer: &mut [(f32, f32)],
) {
    if let Some(mut buffer) = source.buffer().as_ref().and_then(|b| b.lock().ok()) {
        if !buffer.generic().is_empty() {
            let step = 1.0 / mix_buffer.len() as f32;
            let mut t = 0.0;
            for (left, right) in mix_buffer {
                if source.status() != Status::Playing {
                    break;
                }

                let (raw_left, raw_right) = source.next_sample_pair(&mut buffer);

                // Interpolation of gain is very important to remove clicks which appears
                // when gain changes by significant value between frames.
                *left += math::lerpf(*source.last_left_gain.get_or_insert(left_gain), left_gain, t) * raw_left;
                *right += math::lerpf(*source.last_right_gain.get_or_insert(right_gain), right_gain, t) * raw_right;

                t += step;
            }
        }
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
