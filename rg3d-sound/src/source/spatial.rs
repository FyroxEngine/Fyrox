//! Spatial sound source module.
//!
//! # Overview
//!
//! Spatial sound source are most interesting source in the engine. They can have additional effects such as positioning,
//! distance attenuation, can be processed via HRTF, etc.
//!
//! # Usage
//!
//! ```no_run
//! use std::sync::{Arc, Mutex};
//! use rg3d_sound::buffer::SoundBuffer;
//! use rg3d_sound::pool::Handle;
//! use rg3d_sound::source::{SoundSource, Status};
//! use rg3d_sound::source::generic::GenericSourceBuilder;
//! use rg3d_sound::context::Context;
//! use rg3d_sound::source::spatial::SpatialSourceBuilder;
//!
//! fn make_source(context: &mut Context, buffer: Arc<Mutex<SoundBuffer>>) -> Handle<SoundSource> {
//!     let source = SpatialSourceBuilder::new(GenericSourceBuilder::new(buffer)
//!         .with_status(Status::Playing)
//!         .build()
//!         .unwrap())
//!         .build_source();
//!     context.add_source(source)
//! }
//! ```

use crate::{
    context::DistanceModel,
    listener::Listener,
    source::{generic::GenericSource, SoundSource},
};
use rg3d_core::algebra::Vector3;
use rg3d_core::visitor::{Visit, VisitResult, Visitor};
use std::ops::{Deref, DerefMut};

/// See module docs.
#[derive(Debug, Clone)]
pub struct SpatialSource {
    pub(in crate) generic: GenericSource,
    radius: f32,
    position: Vector3<f32>,
    max_distance: f32,
    rolloff_factor: f32,
    // Some data that needed for iterative overlap-save convolution.
    pub(in crate) prev_left_samples: Vec<f32>,
    pub(in crate) prev_right_samples: Vec<f32>,
    pub(in crate) prev_sampling_vector: Vector3<f32>,
    pub(in crate) prev_distance_gain: Option<f32>,
}

impl SpatialSource {
    /// Sets position of source in world space.
    pub fn set_position(&mut self, position: &Vector3<f32>) -> &mut Self {
        self.position = *position;
        self
    }

    /// Returns positions of source.
    pub fn position(&self) -> Vector3<f32> {
        self.position
    }

    /// Sets radius of imaginable sphere around source in which no distance attenuation is applied.
    pub fn set_radius(&mut self, radius: f32) -> &mut Self {
        self.radius = radius;
        self
    }

    /// Returns radius of source.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Sets rolloff factor. Rolloff factor is used in distance attenuation and has different meaning
    /// in various distance models. It is applicable only for InverseDistance and ExponentDistance
    /// distance models. See DistanceModel docs for formulae.
    pub fn set_rolloff_factor(&mut self, rolloff_factor: f32) -> &mut Self {
        self.rolloff_factor = rolloff_factor;
        self
    }

    /// Returns rolloff factor.
    pub fn rolloff_factor(&self) -> f32 {
        self.rolloff_factor
    }

    /// Sets maximum distance until which distance gain will be applicable. Basically it doing this
    /// min(max(distance, radius), max_distance) which clamps distance in radius..max_distance range.
    /// From listener's perspective this will sound like source has stopped decreasing its volume even
    /// if distance continue to grow.
    pub fn set_max_distance(&mut self, max_distance: f32) -> &mut Self {
        self.max_distance = max_distance;
        self
    }

    /// Returns max distance.
    pub fn max_distance(&self) -> f32 {
        self.max_distance
    }

    /// Returns shared reference to inner generic source.
    pub fn generic(&self) -> &GenericSource {
        &self.generic
    }

    /// Returns mutable reference to inner generic source.
    pub fn generic_mut(&mut self) -> &mut GenericSource {
        &mut self.generic
    }

    // Distance models were taken from OpenAL Specification because it looks like they're
    // standard in industry and there is no need to reinvent it.
    // https://www.openal.org/documentation/openal-1.1-specification.pdf
    pub(in crate) fn get_distance_gain(
        &self,
        listener: &Listener,
        distance_model: DistanceModel,
    ) -> f32 {
        let distance = self
            .position
            .metric_distance(&listener.position())
            .max(self.radius)
            .min(self.max_distance);
        match distance_model {
            DistanceModel::None => 1.0,
            DistanceModel::InverseDistance => {
                self.radius / (self.radius + self.rolloff_factor * (distance - self.radius))
            }
            DistanceModel::LinearDistance => {
                1.0 - self.radius * (distance - self.radius) / (self.max_distance - self.radius)
            }
            DistanceModel::ExponentDistance => (distance / self.radius).powf(-self.rolloff_factor),
        }
    }

    pub(in crate) fn get_panning(&self, listener: &Listener) -> f32 {
        (self.position - listener.position())
            .try_normalize(std::f32::EPSILON)
            // Fallback to look axis will give zero panning which will result in even
            // gain in each channels (as if there was no panning at all).
            .unwrap_or_else(|| listener.look_axis())
            .dot(&listener.ear_axis())
    }

    pub(in crate) fn get_sampling_vector(&self, listener: &Listener) -> Vector3<f32> {
        let to_self = self.position - listener.position();

        (listener.basis() * to_self)
            .try_normalize(std::f32::EPSILON)
            // This is ok to fallback to (0, 0, 1) vector because it's given
            // in listener coordinate system.
            .unwrap_or_else(|| Vector3::new(0.0, 0.0, 1.0))
    }
}

impl Deref for SpatialSource {
    type Target = GenericSource;

    fn deref(&self) -> &Self::Target {
        &self.generic
    }
}

impl DerefMut for SpatialSource {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.generic
    }
}

impl Visit for SpatialSource {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;
        self.position.visit("Position", visitor)?;

        visitor.leave_region()
    }
}

impl Default for SpatialSource {
    fn default() -> Self {
        Self {
            generic: Default::default(),
            radius: 1.0,
            position: Vector3::new(0.0, 0.0, 0.0),
            max_distance: std::f32::MAX,
            rolloff_factor: 1.0,
            prev_left_samples: Default::default(),
            prev_right_samples: Default::default(),
            prev_sampling_vector: Vector3::new(0.0, 0.0, 1.0),
            prev_distance_gain: None,
        }
    }
}

/// Spatial source builder allows you to construct new spatial sound source with desired parameters.
pub struct SpatialSourceBuilder {
    generic: GenericSource,
    radius: f32,
    position: Vector3<f32>,
    max_distance: f32,
    rolloff_factor: f32,
}

impl SpatialSourceBuilder {
    /// Creates new spatial source builder from given generic source which. Generic source can be created
    /// using GenericSourceBuilder. See module docs for example.
    pub fn new(generic: GenericSource) -> Self {
        Self {
            generic,
            radius: 1.0,
            position: Vector3::new(0.0, 0.0, 0.0),
            max_distance: std::f32::MAX,
            rolloff_factor: 1.0,
        }
    }

    /// See `set_position` of SpatialSource.
    pub fn with_position(mut self, position: Vector3<f32>) -> Self {
        self.position = position;
        self
    }

    /// See `set_radius` of SpatialSource.
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// See `set_max_distance` of SpatialSource.
    pub fn with_max_distance(mut self, max_distance: f32) -> Self {
        self.max_distance = max_distance;
        self
    }

    /// See `set_rolloff_factor` of SpatialSource.
    pub fn with_rolloff_factor(mut self, rolloff_factor: f32) -> Self {
        self.rolloff_factor = rolloff_factor;
        self
    }

    /// Creates new instance of spatial sound source.
    pub fn build(self) -> SpatialSource {
        SpatialSource {
            generic: self.generic,
            radius: self.radius,
            position: self.position,
            max_distance: self.max_distance,
            rolloff_factor: self.rolloff_factor,
            prev_left_samples: Default::default(),
            prev_right_samples: Default::default(),
            ..Default::default()
        }
    }

    /// Creates new instance of sound source of `Spatial` variant.
    pub fn build_source(self) -> SoundSource {
        SoundSource::Spatial(self.build())
    }
}
