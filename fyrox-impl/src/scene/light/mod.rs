//! Contains all structures and methods to create and manage lights.
//!
//! Light sources arte basic building blocks of many scenes in games, it improves
//! perception of scene and makes it look natural. Fyrox engine supports three kinds
//! of light sources:
//!
//! 1) Directional - similar to sun in real life, its rays are parallel.
//! 2) Spot - similar to flash light, it has cone light volume and circle spot.
//! 3) Point - similar to light bulb, it has spherical light volume.
//!
//! Each kind of light source is suitable for specific conditions, for example
//! spot light can be used if you have a character with flashlight, point - if
//! you have a character with torch, and finally directional - for outdoor light.
//!
//! Most of light sources supports shadows (via shadows maps) and light scattering,
//! these are common effects for modern games but still can significantly impact
//! performance.

use crate::{
    core::{
        algebra::Vector3,
        color::Color,
        reflect::prelude::*,
        variable::InheritableVariable,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::base::{Base, BaseBuilder},
};
use std::ops::{Deref, DerefMut};

pub mod directional;
pub mod point;
pub mod spot;

/// Default amount of light scattering, it is set to 3% which is fairly
/// significant value and you'll clearly see light volume with such settings.
pub const DEFAULT_SCATTER_R: f32 = 0.03;

/// Default amount of light scattering, it is set to 3% which is fairly
/// significant value and you'll clearly see light volume with such settings.
pub const DEFAULT_SCATTER_G: f32 = 0.03;

/// Default amount of light scattering, it is set to 3% which is fairly
/// significant value and you'll clearly see light volume with such settings.
pub const DEFAULT_SCATTER_B: f32 = 0.03;

/// Light scene node. It contains common properties of light such as color,
/// scattering factor (per color channel) and other useful properties. Exact
/// behavior defined by specific light kind.
#[derive(Debug, Reflect, Clone, Visit)]
pub struct BaseLight {
    base: Base,

    #[reflect(setter = "set_color")]
    color: InheritableVariable<Color>,

    #[reflect(setter = "set_cast_shadows")]
    cast_shadows: InheritableVariable<bool>,

    #[visit(rename = "ScatterFactor")]
    #[reflect(setter = "set_scatter")]
    scatter: InheritableVariable<Vector3<f32>>,

    #[reflect(setter = "enable_scatter")]
    scatter_enabled: InheritableVariable<bool>,

    #[reflect(min_value = 0.0, step = 0.1)]
    #[reflect(setter = "set_intensity")]
    intensity: InheritableVariable<f32>,
}

impl Deref for BaseLight {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for BaseLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Default for BaseLight {
    fn default() -> Self {
        Self {
            base: Default::default(),
            color: InheritableVariable::new_modified(Color::WHITE),
            cast_shadows: InheritableVariable::new_modified(true),
            scatter: InheritableVariable::new_modified(Vector3::new(
                DEFAULT_SCATTER_R,
                DEFAULT_SCATTER_G,
                DEFAULT_SCATTER_B,
            )),
            scatter_enabled: InheritableVariable::new_modified(true),
            intensity: InheritableVariable::new_modified(1.0),
        }
    }
}

impl BaseLight {
    /// Sets color of light, alpha component of color is ignored.
    #[inline]
    pub fn set_color(&mut self, color: Color) -> Color {
        self.color.set_value_and_mark_modified(color)
    }

    /// Returns current color of light source.
    #[inline]
    pub fn color(&self) -> Color {
        *self.color
    }

    /// Enables or disables shadows for light source.
    #[inline]
    pub fn set_cast_shadows(&mut self, value: bool) -> bool {
        self.cast_shadows.set_value_and_mark_modified(value)
    }

    /// Returns true if light is able to cast shadows, false - otherwise.
    #[inline]
    pub fn is_cast_shadows(&self) -> bool {
        *self.cast_shadows
    }

    /// Sets scatter factor per color channel (red, green, blue) in (0..1) range.
    /// This parameter defines how "thick" environment is and how much light will
    /// be scattered in light volume. Ability to change this parameter per channel
    /// allows you simulate Rayleigh scatter if needed - in simple words Rayleigh
    /// scatter tells us that blue light waves scatters much better than red ones,
    /// this effect makes sky blue. Reasonable value is something near 0.024-0.03
    /// per color channel, higher values will cause too "heavy" light scattering
    /// as if you light source would be in fog.
    #[inline]
    pub fn set_scatter(&mut self, f: Vector3<f32>) -> Vector3<f32> {
        self.scatter.set_value_and_mark_modified(f)
    }

    /// Returns current scatter factor.
    #[inline]
    pub fn scatter(&self) -> Vector3<f32> {
        *self.scatter
    }

    /// Sets new light intensity. Default is 1.0.
    ///
    /// Intensity is used for very bright light sources in HDR. For examples, sun
    /// can be represented as directional light source with very high intensity.
    /// Other lights, however, will remain relatively dim.
    pub fn set_intensity(&mut self, intensity: f32) -> f32 {
        self.intensity.set_value_and_mark_modified(intensity)
    }

    /// Returns current intensity of the light.
    pub fn intensity(&self) -> f32 {
        *self.intensity
    }

    /// Returns current scatter factor in linear color space.
    #[inline]
    pub fn scatter_linear(&self) -> Vector3<f32> {
        self.scatter.map(|v| v.powf(2.2))
    }

    /// Enables or disables light scattering.
    #[inline]
    pub fn enable_scatter(&mut self, state: bool) -> bool {
        self.scatter_enabled.set_value_and_mark_modified(state)
    }

    /// Returns true if light scattering is enabled, false - otherwise.
    #[inline]
    pub fn is_scatter_enabled(&self) -> bool {
        *self.scatter_enabled
    }
}

/// Light scene node builder. Provides easy declarative way of creating light scene
/// nodes.
pub struct BaseLightBuilder {
    base_builder: BaseBuilder,
    color: Color,
    cast_shadows: bool,
    scatter_factor: Vector3<f32>,
    scatter_enabled: bool,
    intensity: f32,
}

impl BaseLightBuilder {
    /// Creates new instance of light scene node builder, you must pass desired
    /// light kind and base scene node builder as parameters. Latter one is needed
    /// because engine uses composition and light scene node built on top of base
    /// scene node.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            color: Color::WHITE,
            cast_shadows: true,
            scatter_factor: Vector3::new(DEFAULT_SCATTER_R, DEFAULT_SCATTER_G, DEFAULT_SCATTER_B),
            scatter_enabled: true,
            intensity: 1.0,
        }
    }

    /// Sets light color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets whether to casts shadows or not.
    pub fn cast_shadows(mut self, cast_shadows: bool) -> Self {
        self.cast_shadows = cast_shadows;
        self
    }

    /// Sets light scatter factor per color channel.
    pub fn with_scatter_factor(mut self, f: Vector3<f32>) -> Self {
        self.scatter_factor = f;
        self
    }

    /// Whether light scatter enabled or not.
    pub fn with_scatter_enabled(mut self, state: bool) -> Self {
        self.scatter_enabled = state;
        self
    }

    /// Sets desired light intensity.
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// Creates new instance of base light.
    pub fn build(self) -> BaseLight {
        BaseLight {
            base: self.base_builder.build_base(),
            color: self.color.into(),
            cast_shadows: self.cast_shadows.into(),
            scatter: self.scatter_factor.into(),
            scatter_enabled: self.scatter_enabled.into(),
            intensity: self.intensity.into(),
        }
    }
}
