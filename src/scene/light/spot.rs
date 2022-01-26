//! Spot light is can be imagined as flash light - it has direction and cone
//! shape of light volume. It defined by two angles:
//! 1) Hot spot inner angle - this is zone where intensity of light is max.
//! 2) Falloff outer angle delta - small angle that adds to hotspot angle and
//! at this final angle light will have zero intensity. Intensity between those
//! two angles will have smooth transition.
//!
//! Same as point lights, spot lights have distance attenuation which defines
//! how intensity of light changes over distance to point in world. Currently
//! engine uses inverse square root law of distance attenuation.
//!
//! # Light scattering
//!
//! Spot lights support light scattering feature - it means that you will see
//! light volume itself, not just lighted surfaces. Example from real life: flash
//! light in the fog. This effect significantly improves perception of light, but
//! should be used carefully with sane values of light scattering, otherwise you'll
//! get bright glowing cone instead of slightly visible light volume.
//!
//! # Performance notes
//!
//! Light scattering feature may significantly impact performance on low-end
//! hardware!

use crate::engine::resource_manager::ResourceManager;
use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    resource::texture::Texture,
    scene::{
        graph::Graph,
        light::{BaseLight, BaseLightBuilder, Light},
        node::Node,
    },
};
use std::ops::{Deref, DerefMut};

/// See module docs.
#[derive(Debug, Inspect)]
pub struct SpotLight {
    base_light: BaseLight,
    #[inspect(min_value = 0.0, max_value = 3.14159, step = 0.1)]
    hotspot_cone_angle: f32,
    #[inspect(min_value = 0.0, step = 0.1)]
    falloff_angle_delta: f32,
    #[inspect(min_value = 0.0, step = 0.001)]
    shadow_bias: f32,
    #[inspect(min_value = 0.0, step = 0.1)]
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
    pub fn set_cookie_texture(&mut self, texture: Option<Texture>) -> &mut Self {
        self.cookie_texture = texture;
        self
    }

    /// Get cookie texture. Also called gobo this texture gets projected
    /// by the spot light.
    #[inline]
    pub fn cookie_texture(&self) -> Option<Texture> {
        self.cookie_texture.clone()
    }

    /// Get cookie texture by ref. Also called gobo this texture gets projected
    /// by the spot light.
    #[inline]
    pub fn cookie_texture_ref(&self) -> Option<&Texture> {
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

    pub(crate) fn restore_resources(&mut self, resource_manager: ResourceManager) {
        self.cookie_texture = resource_manager.map_texture(self.cookie_texture.clone());
    }

    // Prefab inheritance resolving.
    pub(crate) fn inherit(&mut self, parent: &Node) {
        self.base_light.inherit(parent);

        // TODO: Add properties. https://github.com/FyroxEngine/Fyrox/issues/282
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
        self.shadow_bias.visit("ShadowBias", visitor)?;
        self.cookie_texture.visit("CookieTexture", visitor)?;

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
