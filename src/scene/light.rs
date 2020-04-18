//! Contains all structures and methods to create and manage lights.
//!
//! Light sources arte basic building blocks of many scenes in games, it improves
//! perception of scene and makes it look natural. rg3d engine supports three kinds
//! of ligth sources:
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

#![warn(missing_docs)]

use crate::{
    core::{
        color::Color,
        visitor::{
            Visit,
            Visitor,
            VisitResult,
        },
        math::vec3::Vec3,
    },
    scene::base::{
        BaseBuilder,
        Base,
    },
};
use std::ops::{DerefMut, Deref};

/// Default amount of light scattering, it is set to 3% which is fairly
/// significant value and you'll clearly see light volume with such settings.
pub const DEFAULT_SCATTER: Vec3 = Vec3::new(0.03, 0.03, 0.03);

/// Spot light is can be imagined as flash light - it has direction and cone
/// shape of light volume. It defined by two angles:
/// 1) Hot spot inner angle - this is zone where intensity of light is max.
/// 2) Falloff outer angle delta - small angle that adds to hotspot angle and
/// at this final angle light will have zero intensity. Intensity between those
/// two angles will have smooth transition.
///
/// Same as point lights, spot lights have distance attenuation which defines
/// how intensity of light changes over distance to point in world. Currently
/// engine uses inverse square root law of distance attenuation.
///
/// # Light scattering
///
/// Spot lights support light scattering feature - it means that you will see
/// light volume itself, not just lighted surfaces. Example from real life: flash
/// light in the fog. This effect significantly improves perception of light, but
/// should be used carefully with sane values of light scattering, otherwise you'll
/// get bright glowing cone instead of slightly visible light volume.
///
/// # Performance notes
///
/// Light scattering feature may significantly impact performance on low-end
/// hardware!
#[derive(Clone)]
pub struct SpotLight {
    hotspot_cone_angle: f32,
    falloff_angle_delta: f32,
    distance: f32,
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            hotspot_cone_angle: 90.0f32.to_radians(),
            falloff_angle_delta: 5.0f32.to_radians(),
            distance: 10.0,
        }
    }
}

impl SpotLight {
    /// Creates new instance of spot light with given parameters. For more info about
    /// parameters see struct docs.
    pub fn new(distance: f32, hotspot_cone_angle: f32, falloff_angle_delta: f32) -> Self {
        Self {
            hotspot_cone_angle: hotspot_cone_angle.abs(),
            falloff_angle_delta: falloff_angle_delta.abs(),
            distance,
        }
    }

    /// Returns hotspot angle of light.
    #[inline]
    pub fn hotspot_cone_angle(&self) -> f32 {
        self.hotspot_cone_angle
    }

    /// Sets new value of hotspot angle of light.
    #[inline]
    pub fn set_hotspot_cone_angle(&mut self, cone_angle: f32) -> &mut Self {
        self.hotspot_cone_angle = cone_angle.abs();
        self
    }

    /// Sets new falloff angle range for spot light.
    #[inline]
    pub fn set_falloff_angle_delta(&mut self, delta: f32) -> &mut Self {
        self.falloff_angle_delta = delta;
        self
    }

    /// Returns falloff angle range of light.
    #[inline]
    pub fn falloff_angle_delta(&self) -> f32 {
        self.falloff_angle_delta
    }

    /// Returns full angle at top of light cone.
    #[inline]
    pub fn full_cone_angle(&self) -> f32 {
        self.hotspot_cone_angle + self.falloff_angle_delta
    }

    /// Sets maximum distance at which light intensity will be zero. Intensity
    /// of light will be calculated using inverse square root law.
    #[inline]
    pub fn set_distance(&mut self, distance: f32) -> &mut Self {
        self.distance = distance.abs();
        self
    }

    /// Returns maximum distance of light.
    #[inline]
    pub fn distance(&self) -> f32 {
        self.distance
    }
}

impl Visit for SpotLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.hotspot_cone_angle.visit("HotspotConeAngle", visitor)?;
        self.falloff_angle_delta.visit("FalloffAngleDelta", visitor)?;
        self.distance.visit("Distance", visitor)?;

        visitor.leave_region()
    }
}

/// Point light can be represented as light bulb which hangs on wire - it is
/// spherical light source which emits light in all directions. It has single
/// parameter - radius at which intensity will be zero. Intensity of light will
/// be calculated using inverse square root law.
///
/// # Light scattering
///
/// Point light support light scattering feature - it means that you'll see light
/// volume as well as lighted surfaces. Simple example from real life: light bulb
/// in the fog. This effect significantly improves perception of light, but should
/// be used carefully with sane values of light scattering, otherwise you'll get
/// bright glowing sphere instead of slightly visible light volume.
///
/// # Performance notes
///
/// Point lights supports shadows, but keep in mind - they're very expensive and
/// can easily ruin performance of your game, especially on low-end hardware. Light
/// scattering is relatively heavy too.
#[derive(Clone)]
pub struct PointLight {
    radius: f32
}

impl PointLight {
    /// Creates new point light with given radius.
    pub fn new(radius: f32) -> Self {
        Self {
            radius
        }
    }

    /// Sets radius of point light. This parameter also affects radius of spherical
    /// light volume that is used in light scattering.
    #[inline]
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius.abs();
    }

    /// Returns radius of point light.
    #[inline]
    pub fn radius(&self) -> f32 {
        self.radius
    }
}

impl Visit for PointLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;

        visitor.leave_region()
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            radius: 10.0
        }
    }
}

/// Engine supports limited amount of light source kinds
#[derive(Clone)]
pub enum LightKind {
    /// Directional light is a light source with parallel rays, it has
    /// excellent example in real life - Sun. It does not have position,
    /// only direction which defined by parent light scene node.
    ///
    /// # Notes
    ///
    /// Current directional light does *not* support shadows, it is still
    /// on list of features that should be implemented.
    Directional,

    /// See SpotLight struct docs.
    Spot(SpotLight),

    /// See PointLight struct docs.
    Point(PointLight),
}

impl LightKind {
    fn new(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(LightKind::Spot(Default::default())),
            1 => Ok(LightKind::Point(Default::default())),
            2 => Ok(LightKind::Directional),
            _ => Err(format!("Invalid light kind {}", id))
        }
    }

    fn id(&self) -> u32 {
        match self {
            LightKind::Spot(_) => 0,
            LightKind::Point(_) => 1,
            LightKind::Directional => 2,
        }
    }
}

impl Visit for LightKind {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        match self {
            LightKind::Spot(spot_light) => spot_light.visit(name, visitor),
            LightKind::Point(point_light) => point_light.visit(name, visitor),
            LightKind::Directional => Ok(())
        }
    }
}

/// Light scene node. It contains common properties of light such as color,
/// scattering factor (per color channel) and other useful properties. Exact
/// behavior defined by specific light kind.
#[derive(Clone)]
pub struct Light {
    base: Base,
    kind: LightKind,
    color: Color,
    cast_shadows: bool,
    scatter: Vec3,
    scatter_enabled: bool,
}

impl Deref for Light {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Light {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Default for Light {
    fn default() -> Self {
        Self {
            base: Default::default(),
            kind: LightKind::Point(Default::default()),
            color: Color::WHITE,
            cast_shadows: true,
            scatter: DEFAULT_SCATTER,
            scatter_enabled: true,
        }
    }
}

impl Visit for Light {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind_id = self.kind.id();
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            self.kind = LightKind::new(kind_id)?;
        }
        self.kind.visit("Kind", visitor)?;
        self.color.visit("Color", visitor)?;
        self.base.visit("Base", visitor)?;
        self.cast_shadows.visit("CastShadows", visitor)?;
        self.scatter.visit("ScatterFactor", visitor)?;
        self.scatter_enabled.visit("ScatterEnabled", visitor)?;

        visitor.leave_region()
    }
}

impl Light {
    /// Creates new light of given kind.
    pub fn new(kind: LightKind) -> Self {
        Self {
            kind,
            ..Default::default()
        }
    }

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

    /// Returns shared reference to light kind. It can be used to
    /// read properties of specific kind of light source.
    #[inline]
    pub fn kind(&self) -> &LightKind {
        &self.kind
    }

    /// Returns mutable reference to light kind. It can be used to
    /// modify parameters of specific kind of light source.
    #[inline]
    pub fn kind_mut(&mut self) -> &mut LightKind {
        &mut self.kind
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
    pub fn set_scatter(&mut self, f: Vec3) {
        self.scatter = f;
    }

    /// Returns current scatter factor.
    #[inline]
    pub fn scatter(&self) -> Vec3 {
        self.scatter
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
}

/// Light scene node builder. Provides easy declarative way of creating light scene
/// nodes.
pub struct LightBuilder {
    base_builder: BaseBuilder,
    kind: LightKind,
    color: Color,
    cast_shadows: bool,
    scatter_factor: Vec3,
    scatter_enabled: bool,
}

impl LightBuilder {
    /// Creates new instance of light scene node builder, you must pass desired
    /// light kind and base scene node builder as parameters. Latter one is needed
    /// because engine uses composition and light scene node built on top of base
    /// scene node.
    pub fn new(kind: LightKind, base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            kind,
            color: Color::WHITE,
            cast_shadows: true,
            scatter_factor: DEFAULT_SCATTER,
            scatter_enabled: true,
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
    pub fn with_scatter_factor(mut self, f: Vec3) -> Self {
        self.scatter_factor = f;
        self
    }

    /// Whether light scatter enabled or not.
    pub fn with_scatter_enabled(mut self, state: bool) -> Self {
        self.scatter_enabled = state;
        self
    }

    /// Creates new instance of light scene node. Warning: each scene node
    /// must be added to scene, otherwise it won't have any effect and most
    /// likely will be dropped as soon as it go out of scope.
    pub fn build(self) -> Light {
        Light {
            base: self.base_builder.build(),
            kind: self.kind,
            color: self.color,
            cast_shadows: self.cast_shadows,
            scatter: self.scatter_factor,
            scatter_enabled: self.scatter_enabled,
        }
    }
}