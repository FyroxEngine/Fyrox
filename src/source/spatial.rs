use rg3d_core::visitor::{
    Visitor,
    VisitResult,
    Visit,
};
use rustfft::num_complex::Complex;
use crate::{
    math::vec3::Vec3,
    source::{
        generic::GenericSource,
        SoundSource,
    },
    listener::Listener,
    context::DistanceModel,
};

pub struct SpatialSource {
    generic: GenericSource,
    /// Radius of sphere around sound source at which sound volume is half of initial.
    radius: f32,
    position: Vec3,
    max_distance: f32,
    rolloff_factor: f32,
    // Rest of samples from previous frame that has to be added to output signal.
    pub(in crate) last_frame_left_samples: Vec<Complex<f32>>,
    pub(in crate) last_frame_right_samples: Vec<Complex<f32>>,
}

impl SpatialSource {
    pub fn set_position(&mut self, position: &Vec3) -> &mut Self {
        self.position = *position;
        self
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn set_radius(&mut self, radius: f32) -> &mut Self {
        self.radius = radius;
        self
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub fn generic(&self) -> &GenericSource {
        &self.generic
    }

    pub fn generic_mut(&mut self) -> &mut GenericSource {
        &mut self.generic
    }

    // Distance models were taken from OpenAL Specification because it looks like they're
    // standard in industry and there is no need to reinvent it.
    // https://www.openal.org/documentation/openal-1.1-specification.pdf
    pub(in crate) fn get_distance_gain(&self, listener: &Listener, distance_model: DistanceModel) -> f32 {
        let distance = self.position
            .distance(&listener.position)
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
            DistanceModel::ExponentDistance => {
                (distance / self.radius).powf(-self.rolloff_factor)
            }
        }
    }

    pub(in crate) fn get_panning(&self, listener: &Listener) -> f32 {
        (self.position - listener.position)
            .normalized()
            // Fallback to look axis will give zero panning which will result in even
            // gain in each channels (as if there was no panning at all).
            .unwrap_or(listener.look_axis)
            .dot(&listener.ear_axis)
    }

    pub(in crate) fn get_sampling_vector(&self, listener: &Listener) -> Vec3 {
        listener.view_matrix
            .transform_vector_normal(self.position - listener.position)
            .normalized()
            // This is ok to fallback to (0, 0, 1) vector because it's given
            // in listener coordinate system.
            .unwrap_or_else(|| Vec3::new(0.0, 0.0, 1.0))
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
            position: Vec3::ZERO,
            max_distance: std::f32::MAX,
            rolloff_factor: 1.0,
            last_frame_left_samples: Default::default(),
            last_frame_right_samples: Default::default(),
        }
    }
}

pub struct SpatialSourceBuilder {
    generic: GenericSource,
    radius: f32,
    position: Vec3,
    max_distance: f32,
    rolloff_factor: f32,
}

impl SpatialSourceBuilder {
    pub fn new(generic: GenericSource) -> Self {
        Self {
            generic,
            radius: 1.0,
            position: Default::default(),
            max_distance: std::f32::MAX,
            rolloff_factor: 1.0,
        }
    }

    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_max_distance(mut self, max_distance: f32) -> Self {
        self.max_distance = max_distance;
        self
    }

    pub fn with_rolloff_factor(mut self, rolloff_factor: f32) -> Self {
        self.rolloff_factor = rolloff_factor;
        self
    }

    pub fn build(self) -> SpatialSource {
        SpatialSource {
            generic: self.generic,
            radius: self.radius,
            position: self.position,
            max_distance: self.max_distance,
            rolloff_factor: self.rolloff_factor,
            last_frame_left_samples: Default::default(),
            last_frame_right_samples: Default::default(),
        }
    }

    pub fn build_source(self) -> SoundSource {
        SoundSource::Spatial(self.build())
    }
}