//! Contains all structures and methods to create and manage lights.
//!
//! Light sources arte basic building blocks of many scenes in games, it improves
//! perception of scene and makes it look natural. rg3d engine supports three kinds
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
        define_is_as,
        inspect::{Inspect, PropertyInfo},
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::{
        base::{Base, BaseBuilder},
        light::{directional::DirectionalLight, point::PointLight, spot::SpotLight},
    },
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

/// Engine supports limited amount of light source kinds
#[derive(Debug)]
pub enum Light {
    /// See [DirectionalLight](struct.DirectionalLight.html)
    Directional(DirectionalLight),

    /// See [SpotLight](struct.SpotLight.html)
    Spot(SpotLight),

    /// See [PointLight](struct.PointLight.html)
    Point(PointLight),
}

impl Inspect for Light {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        match self {
            Light::Directional(v) => v.properties(),
            Light::Spot(v) => v.properties(),
            Light::Point(v) => v.properties(),
        }
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::Directional(Default::default())
    }
}

impl Light {
    fn new(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Spot(Default::default())),
            1 => Ok(Self::Point(Default::default())),
            2 => Ok(Self::Directional(Default::default())),
            _ => Err(format!("Invalid light kind {}", id)),
        }
    }

    fn id(&self) -> u32 {
        match self {
            Self::Spot(_) => 0,
            Self::Point(_) => 1,
            Self::Directional(_) => 2,
        }
    }

    /// Creates a raw copy of a light node.
    pub fn raw_copy(&self) -> Self {
        match self {
            Light::Directional(v) => Self::Directional(v.raw_copy()),
            Light::Spot(v) => Self::Spot(v.raw_copy()),
            Light::Point(v) => Self::Point(v.raw_copy()),
        }
    }

    define_is_as!(Light : Directional -> ref DirectionalLight => fn is_directional, fn as_directional, fn as_directional_mut);
    define_is_as!(Light : Spot -> ref SpotLight => fn is_spot, fn as_spot, fn as_spot_mut);
    define_is_as!(Light : Point -> ref PointLight => fn is_point, fn as_point, fn as_point_mut);
}

impl Deref for Light {
    type Target = BaseLight;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Directional(v) => v.deref(),
            Self::Spot(v) => v.deref(),
            Self::Point(v) => v.deref(),
        }
    }
}

impl DerefMut for Light {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Directional(v) => v.deref_mut(),
            Self::Spot(v) => v.deref_mut(),
            Self::Point(v) => v.deref_mut(),
        }
    }
}

impl Visit for Light {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind_id = self.id();
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            *self = Self::new(kind_id)?;
        }

        match self {
            Self::Spot(spot_light) => spot_light.visit("Data", visitor)?,
            Self::Point(point_light) => point_light.visit("Data", visitor)?,
            Self::Directional(directional_light) => directional_light.visit("Data", visitor)?,
        }

        visitor.leave_region()
    }
}

/// Light scene node. It contains common properties of light such as color,
/// scattering factor (per color channel) and other useful properties. Exact
/// behavior defined by specific light kind.
#[derive(Debug, Inspect)]
pub struct BaseLight {
    base: Base,
    color: Color,
    cast_shadows: bool,
    scatter: Vector3<f32>,
    scatter_enabled: bool,
    #[inspect(min_value = 0.0, step = 0.1)]
    intensity: f32,
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
            color: Color::WHITE,
            cast_shadows: true,
            scatter: Vector3::new(DEFAULT_SCATTER_R, DEFAULT_SCATTER_G, DEFAULT_SCATTER_B),
            scatter_enabled: true,
            intensity: 1.0,
        }
    }
}

impl Visit for BaseLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.color.visit("Color", visitor)?;
        self.base.visit("Base", visitor)?;
        self.cast_shadows.visit("CastShadows", visitor)?;
        self.scatter.visit("ScatterFactor", visitor)?;
        self.scatter_enabled.visit("ScatterEnabled", visitor)?;
        let _ = self.intensity.visit("Intensity", visitor); // Backward compatibility.

        visitor.leave_region()
    }
}

impl BaseLight {
    /// Sets color of light, alpha component of color is ignored.
    #[inline]
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Returns current color of light source.
    #[inline]
    pub fn color(&self) -> Color {
        self.color
    }

    /// Enables or disables shadows for light source.
    #[inline]
    pub fn set_cast_shadows(&mut self, value: bool) {
        self.cast_shadows = value;
    }

    /// Returns true if light is able to cast shadows, false - otherwise.
    #[inline]
    pub fn is_cast_shadows(&self) -> bool {
        self.cast_shadows
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
    pub fn set_scatter(&mut self, f: Vector3<f32>) {
        self.scatter = f;
    }

    /// Returns current scatter factor.
    #[inline]
    pub fn scatter(&self) -> Vector3<f32> {
        self.scatter
    }

    /// Sets new light intensity. Default is 1.0.
    ///
    /// Intensity is used for very bright light sources in HDR. For examples, sun
    /// can be represented as directional light source with very high intensity.
    /// Other lights, however, will remain relatively dim.
    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
    }

    /// Returns current intensity of the light.
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// Returns current scatter factor in linear color space.
    #[inline]
    pub fn scatter_linear(&self) -> Vector3<f32> {
        self.scatter.map(|v| v.powf(2.2))
    }

    /// Enables or disables light scattering.
    #[inline]
    pub fn enable_scatter(&mut self, state: bool) {
        self.scatter_enabled = state;
    }

    /// Returns true if light scattering is enabled, false - otherwise.
    #[inline]
    pub fn is_scatter_enabled(&self) -> bool {
        self.scatter_enabled
    }

    /// Creates a raw copy of a base light node.
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            color: self.color,
            cast_shadows: self.cast_shadows,
            scatter: self.scatter,
            scatter_enabled: self.scatter_enabled,
            intensity: self.intensity,
        }
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
            color: self.color,
            cast_shadows: self.cast_shadows,
            scatter: self.scatter_factor,
            scatter_enabled: self.scatter_enabled,
            intensity: self.intensity,
        }
    }
}
