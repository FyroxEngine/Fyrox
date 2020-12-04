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

use crate::core::algebra::Vector3;
use crate::core::pool::Handle;
use crate::resource::texture::Texture;
use crate::scene::graph::Graph;
use crate::{
    core::{
        color::Color,
        define_is_as,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::{
        base::{Base, BaseBuilder},
        node::Node,
    },
};
use std::ops::{Deref, DerefMut};

/// Default amount of light scattering, it is set to 3% which is fairly
/// significant value and you'll clearly see light volume with such settings.
pub const DEFAULT_SCATTER_R: f32 = 0.03;

/// Default amount of light scattering, it is set to 3% which is fairly
/// significant value and you'll clearly see light volume with such settings.
pub const DEFAULT_SCATTER_G: f32 = 0.03;

/// Default amount of light scattering, it is set to 3% which is fairly
/// significant value and you'll clearly see light volume with such settings.
pub const DEFAULT_SCATTER_B: f32 = 0.03;

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
#[derive(Debug)]
pub struct SpotLight {
    base_light: BaseLight,
    hotspot_cone_angle: f32,
    falloff_angle_delta: f32,
    shadow_bias: f32,
    distance: f32,
    cookie_texture: Option<Texture>,
}

impl Deref for SpotLight {
    type Target = BaseLight;

    fn deref(&self) -> &Self::Target {
        &self.base_light
    }
}

impl DerefMut for SpotLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_light
    }
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            base_light: Default::default(),
            hotspot_cone_angle: 90.0f32.to_radians(),
            falloff_angle_delta: 5.0f32.to_radians(),
            shadow_bias: 0.00005,
            distance: 10.0,
            cookie_texture: None,
        }
    }
}

impl SpotLight {
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

    /// Sets new shadow bias value. Bias will be used to offset fragment's depth before
    /// compare it with shadow map value, it is used to remove "shadow acne".
    pub fn set_shadow_bias(&mut self, bias: f32) {
        self.shadow_bias = bias;
    }

    /// Returns current value of shadow bias.
    pub fn shadow_bias(&self) -> f32 {
        self.shadow_bias
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

    /// Set cookie texture. Also called gobo this texture gets projected
    /// by the spot light.
    #[inline]
    pub fn set_cookie_texture(&mut self, texture: Texture) -> &mut Self {
        self.cookie_texture = Some(texture);
        self
    }

    /// Get cookie texture. Also called gobo this texture gets projected
    /// by the spot light.
    #[inline]
    pub fn cookie_texture(&self) -> Option<&Texture> {
        self.cookie_texture.as_ref()
    }

    /// Creates a raw copy of a light node.
    pub fn raw_copy(&self) -> Self {
        Self {
            base_light: self.base_light.raw_copy(),
            hotspot_cone_angle: self.hotspot_cone_angle,
            falloff_angle_delta: self.falloff_angle_delta,
            shadow_bias: self.shadow_bias,
            distance: self.distance,
            cookie_texture: self.cookie_texture.clone(),
        }
    }
}

impl Visit for SpotLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.base_light.visit("BaseLight", visitor)?;
        self.hotspot_cone_angle.visit("HotspotConeAngle", visitor)?;
        self.falloff_angle_delta
            .visit("FalloffAngleDelta", visitor)?;
        self.distance.visit("Distance", visitor)?;
        let _ = self.shadow_bias.visit("ShadowBias", visitor);
        let _ = self.cookie_texture.visit("CookieTexture", visitor);

        visitor.leave_region()
    }
}

/// Allows you to build spot light in declarative manner.
pub struct SpotLightBuilder {
    base_light_builder: BaseLightBuilder,
    hotspot_cone_angle: f32,
    falloff_angle_delta: f32,
    shadow_bias: f32,
    distance: f32,
    cookie_texture: Option<Texture>,
}

impl SpotLightBuilder {
    /// Creates new builder instance.
    pub fn new(base_light_builder: BaseLightBuilder) -> Self {
        Self {
            base_light_builder,
            hotspot_cone_angle: 90.0f32.to_radians(),
            falloff_angle_delta: 5.0f32.to_radians(),
            shadow_bias: 0.00005,
            distance: 10.0,
            cookie_texture: None,
        }
    }

    /// Sets desired hot spot cone angle.
    pub fn with_hotspot_cone_angle(mut self, hotspot_cone_angle: f32) -> Self {
        self.hotspot_cone_angle = hotspot_cone_angle;
        self
    }

    /// Sets desired falloff angle delta.
    pub fn with_falloff_angle_delta(mut self, falloff_angle_delta: f32) -> Self {
        self.falloff_angle_delta = falloff_angle_delta;
        self
    }

    /// Sets desired light distance.
    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance = distance;
        self
    }

    /// Sets desired shadow bias.
    pub fn with_shadow_bias(mut self, bias: f32) -> Self {
        self.shadow_bias = bias;
        self
    }

    /// Sets the desired cookie/gobo texture.
    pub fn with_cookie_texture(mut self, texture: Texture) -> Self {
        self.cookie_texture = Some(texture);
        self
    }

    /// Creates new spot light.
    pub fn build_spot_light(self) -> SpotLight {
        SpotLight {
            base_light: self.base_light_builder.build(),
            hotspot_cone_angle: self.hotspot_cone_angle,
            falloff_angle_delta: self.falloff_angle_delta,
            shadow_bias: self.shadow_bias,
            distance: self.distance,
            cookie_texture: self.cookie_texture,
        }
    }

    /// Creates new spot light node.
    pub fn build_node(self) -> Node {
        Node::Light(Light::Spot(self.build_spot_light()))
    }

    /// Creates new spot light instance and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
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
#[derive(Debug)]
pub struct PointLight {
    base_light: BaseLight,
    shadow_bias: f32,
    radius: f32,
}

impl Deref for PointLight {
    type Target = BaseLight;

    fn deref(&self) -> &Self::Target {
        &self.base_light
    }
}

impl DerefMut for PointLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_light
    }
}

impl PointLight {
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

    /// Sets new shadow bias value. Bias will be used to offset fragment's depth before
    /// compare it with shadow map value, it is used to remove "shadow acne".
    pub fn set_shadow_bias(&mut self, bias: f32) {
        self.shadow_bias = bias;
    }

    /// Returns current value of shadow bias.
    pub fn shadow_bias(&self) -> f32 {
        self.shadow_bias
    }

    /// Creates a raw copy of a point light node.
    pub fn raw_copy(&self) -> Self {
        Self {
            base_light: self.base_light.raw_copy(),
            radius: self.radius,
            shadow_bias: self.shadow_bias,
        }
    }
}

impl Visit for PointLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.base_light.visit("BaseLight", visitor)?;
        self.radius.visit("Radius", visitor)?;
        let _ = self.shadow_bias.visit("ShadowBias", visitor);

        visitor.leave_region()
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            base_light: Default::default(),
            shadow_bias: 0.025,
            radius: 10.0,
        }
    }
}

/// Allows you to build point light in declarative manner.
pub struct PointLightBuilder {
    base_light_builder: BaseLightBuilder,
    shadow_bias: f32,
    radius: f32,
}

impl PointLightBuilder {
    /// Creates new builder instance.
    pub fn new(base_light_builder: BaseLightBuilder) -> Self {
        Self {
            base_light_builder,
            shadow_bias: 0.025,
            radius: 10.0,
        }
    }

    /// Sets desired radius.
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Sets desired shadow bias.
    pub fn with_shadow_bias(mut self, bias: f32) -> Self {
        self.shadow_bias = bias;
        self
    }

    /// Builds new instance of point light.
    pub fn build_point_light(self) -> PointLight {
        PointLight {
            base_light: self.base_light_builder.build(),
            radius: self.radius,
            shadow_bias: self.shadow_bias,
        }
    }

    /// Builds new instance of point light node.
    pub fn build_node(self) -> Node {
        Node::Light(Light::Point(self.build_point_light()))
    }

    /// Builds new instance of point light and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

/// Directional light is a light source with parallel rays, it has
/// excellent example in real life - Sun. It does not have position,
/// only direction which defined by parent light scene node.
///
/// # Notes
///
/// Current directional light does *not* support shadows, it is still
/// on list of features that should be implemented.
#[derive(Default, Debug)]
pub struct DirectionalLight {
    base_light: BaseLight,
}

impl From<BaseLight> for DirectionalLight {
    fn from(base_light: BaseLight) -> Self {
        Self { base_light }
    }
}

impl Deref for DirectionalLight {
    type Target = BaseLight;

    fn deref(&self) -> &Self::Target {
        &self.base_light
    }
}

impl DerefMut for DirectionalLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_light
    }
}

impl Visit for DirectionalLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.base_light.visit("BaseLight", visitor)?;

        visitor.leave_region()
    }
}

impl DirectionalLight {
    /// Creates a raw copy of a directional light node.
    pub fn raw_copy(&self) -> Self {
        Self {
            base_light: self.base_light.raw_copy(),
        }
    }
}

/// Allows you to build directional light in declarative manner.
pub struct DirectionalLightBuilder {
    base_light_builder: BaseLightBuilder,
}

impl DirectionalLightBuilder {
    /// Creates new builder instance.
    pub fn new(base_light_builder: BaseLightBuilder) -> Self {
        Self { base_light_builder }
    }

    /// Creates new instance of directional light.
    pub fn build_directional_light(self) -> DirectionalLight {
        DirectionalLight {
            base_light: self.base_light_builder.build(),
        }
    }

    /// Creates new instance of directional light node.
    pub fn build_node(self) -> Node {
        Node::Light(Light::Directional(self.build_directional_light()))
    }

    /// Creates new instance of directional light and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

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
#[derive(Debug)]
pub struct BaseLight {
    base: Base,
    color: Color,
    cast_shadows: bool,
    scatter: Vector3<f32>,
    scatter_enabled: bool,
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

    /// Creates new instance of base light.
    pub fn build(self) -> BaseLight {
        BaseLight {
            base: self.base_builder.build_base(),
            color: self.color,
            cast_shadows: self.cast_shadows,
            scatter: self.scatter_factor,
            scatter_enabled: self.scatter_enabled,
        }
    }
}
