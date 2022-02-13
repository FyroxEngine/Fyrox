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

use crate::scene::base::Base;
use crate::scene::node::NodeTrait;
use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    impl_directly_inheritable_entity_trait,
    resource::texture::Texture,
    scene::{
        graph::Graph,
        light::{BaseLight, BaseLightBuilder},
        node::Node,
        variable::{InheritError, TemplateVariable},
        DirectlyInheritableEntity,
    },
};
use fxhash::FxHashMap;
use fyrox_core::math::aabb::AxisAlignedBoundingBox;
use fyrox_core::uuid::Uuid;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

/// See module docs.
#[derive(Debug, Inspect, Clone)]
pub struct SpotLight {
    base_light: BaseLight,

    #[inspect(
        min_value = 0.0,
        max_value = 3.14159,
        step = 0.1,
        getter = "Deref::deref"
    )]
    hotspot_cone_angle: TemplateVariable<f32>,

    #[inspect(min_value = 0.0, step = 0.1, getter = "Deref::deref")]
    falloff_angle_delta: TemplateVariable<f32>,

    #[inspect(min_value = 0.0, step = 0.001, getter = "Deref::deref")]
    shadow_bias: TemplateVariable<f32>,

    #[inspect(min_value = 0.0, step = 0.1, getter = "Deref::deref")]
    distance: TemplateVariable<f32>,

    #[inspect(getter = "Deref::deref")]
    cookie_texture: TemplateVariable<Option<Texture>>,
}

impl_directly_inheritable_entity_trait!(SpotLight;
    hotspot_cone_angle,
    falloff_angle_delta,
    shadow_bias,
    distance,
    cookie_texture
);

impl Deref for SpotLight {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base_light.base
    }
}

impl DerefMut for SpotLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_light.base
    }
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            base_light: Default::default(),
            hotspot_cone_angle: TemplateVariable::new(90.0f32.to_radians()),
            falloff_angle_delta: TemplateVariable::new(5.0f32.to_radians()),
            shadow_bias: TemplateVariable::new(0.00005),
            distance: TemplateVariable::new(10.0),
            cookie_texture: TemplateVariable::new(None),
        }
    }
}

impl SpotLight {
    pub fn type_uuid() -> Uuid {
        Uuid::from_str("9856a3c1-ced7-47ec-b682-4dc4dea89d8f").unwrap()
    }

    pub fn base_light_ref(&self) -> &BaseLight {
        &self.base_light
    }

    pub fn base_light_mut(&mut self) -> &mut BaseLight {
        &mut self.base_light
    }

    /// Returns hotspot angle of light.
    #[inline]
    pub fn hotspot_cone_angle(&self) -> f32 {
        *self.hotspot_cone_angle
    }

    /// Sets new value of hotspot angle of light.
    #[inline]
    pub fn set_hotspot_cone_angle(&mut self, cone_angle: f32) -> &mut Self {
        self.hotspot_cone_angle.set(cone_angle.abs());
        self
    }

    /// Sets new falloff angle range for spot light.
    #[inline]
    pub fn set_falloff_angle_delta(&mut self, delta: f32) -> &mut Self {
        self.falloff_angle_delta.set(delta);
        self
    }

    /// Returns falloff angle range of light.
    #[inline]
    pub fn falloff_angle_delta(&self) -> f32 {
        *self.falloff_angle_delta
    }

    /// Returns full angle at top of light cone.
    #[inline]
    pub fn full_cone_angle(&self) -> f32 {
        *self.hotspot_cone_angle + *self.falloff_angle_delta
    }

    /// Sets new shadow bias value. Bias will be used to offset fragment's depth before
    /// compare it with shadow map value, it is used to remove "shadow acne".
    pub fn set_shadow_bias(&mut self, bias: f32) {
        self.shadow_bias.set(bias);
    }

    /// Returns current value of shadow bias.
    pub fn shadow_bias(&self) -> f32 {
        *self.shadow_bias
    }

    /// Sets maximum distance at which light intensity will be zero. Intensity
    /// of light will be calculated using inverse square root law.
    #[inline]
    pub fn set_distance(&mut self, distance: f32) -> &mut Self {
        self.distance.set(distance.abs());
        self
    }

    /// Returns maximum distance of light.
    #[inline]
    pub fn distance(&self) -> f32 {
        *self.distance
    }

    /// Set cookie texture. Also called gobo this texture gets projected
    /// by the spot light.
    #[inline]
    pub fn set_cookie_texture(&mut self, texture: Option<Texture>) -> &mut Self {
        self.cookie_texture.set(texture);
        self
    }

    /// Get cookie texture. Also called gobo this texture gets projected
    /// by the spot light.
    #[inline]
    pub fn cookie_texture(&self) -> Option<Texture> {
        (*self.cookie_texture).clone()
    }

    /// Get cookie texture by ref. Also called gobo this texture gets projected
    /// by the spot light.
    #[inline]
    pub fn cookie_texture_ref(&self) -> Option<&Texture> {
        self.cookie_texture.as_ref()
    }
}

impl NodeTrait for SpotLight {
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base_light.base.local_bounding_box()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base_light.base.world_bounding_box()
    }

    // Prefab inheritance resolving.
    fn inherit(&mut self, parent: &Node) -> Result<(), InheritError> {
        if let Some(parent) = parent.cast::<SpotLight>() {
            self.base_light.inherit(parent.base_light_ref())?;
            self.try_inherit_self_properties(parent)?;
        }
        Ok(())
    }

    fn reset_inheritable_properties(&mut self) {
        self.base_light.reset_inheritable_properties();
        self.reset_self_inheritable_properties();
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        let mut state = resource_manager.state();
        let texture_container = &mut state.containers_mut().textures;
        texture_container.try_restore_template_resource(&mut self.cookie_texture);
    }

    fn remap_handles(&mut self, old_new_mapping: &FxHashMap<Handle<Node>, Handle<Node>>) {
        self.base_light.remap_handles(old_new_mapping);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
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
            hotspot_cone_angle: self.hotspot_cone_angle.into(),
            falloff_angle_delta: self.falloff_angle_delta.into(),
            shadow_bias: self.shadow_bias.into(),
            distance: self.distance.into(),
            cookie_texture: self.cookie_texture.into(),
        }
    }

    /// Creates new spot light node.
    pub fn build_node(self) -> Node {
        Node::new(self.build_spot_light())
    }

    /// Creates new spot light instance and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        resource::texture::test::create_test_texture,
        scene::{
            base::{test::check_inheritable_properties_equality, BaseBuilder},
            light::{spot::SpotLightBuilder, BaseLightBuilder, Light},
            node::{Node, NodeTrait},
        },
    };

    #[test]
    fn test_spot_light_inheritance() {
        let parent = SpotLightBuilder::new(BaseLightBuilder::new(BaseBuilder::new()))
            .with_distance(1.0)
            .with_cookie_texture(create_test_texture())
            .with_falloff_angle_delta(0.1)
            .with_shadow_bias(1.0)
            .with_hotspot_cone_angle(0.1)
            .build_node();

        let mut child =
            SpotLightBuilder::new(BaseLightBuilder::new(BaseBuilder::new())).build_spot_light();

        child.inherit(&parent).unwrap();

        if let Node::Light(Light::Spot(parent)) = parent {
            check_inheritable_properties_equality(&child.base, &parent.base);
            check_inheritable_properties_equality(&child.base_light, &parent.base_light);
            check_inheritable_properties_equality(&child, &parent);
        } else {
            unreachable!()
        }
    }
}
