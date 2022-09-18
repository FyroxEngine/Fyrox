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

use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::Reflect,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    resource::texture::Texture,
    scene::{
        base::Base,
        graph::Graph,
        light::{BaseLight, BaseLightBuilder},
        node::{Node, NodeTrait, TypeUuidProvider},
    },
};
use std::ops::{Deref, DerefMut};

/// See module docs.
#[derive(Debug, Inspect, Reflect, Clone, Visit)]
pub struct SpotLight {
    base_light: BaseLight,

    #[inspect(min_value = 0.0, max_value = 3.14159, step = 0.1)]
    #[reflect(setter = "set_hotspot_cone_angle")]
    hotspot_cone_angle: InheritableVariable<f32>,

    #[inspect(min_value = 0.0, step = 0.1)]
    #[reflect(setter = "set_falloff_angle_delta")]
    falloff_angle_delta: InheritableVariable<f32>,

    #[inspect(min_value = 0.0, step = 0.001)]
    #[reflect(setter = "set_shadow_bias")]
    shadow_bias: InheritableVariable<f32>,

    #[inspect(min_value = 0.0, step = 0.1)]
    #[reflect(setter = "set_distance")]
    distance: InheritableVariable<f32>,

    #[reflect(setter = "set_cookie_texture")]
    cookie_texture: InheritableVariable<Option<Texture>>,
}

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
            hotspot_cone_angle: InheritableVariable::new(90.0f32.to_radians()),
            falloff_angle_delta: InheritableVariable::new(5.0f32.to_radians()),
            shadow_bias: InheritableVariable::new(0.00005),
            distance: InheritableVariable::new(10.0),
            cookie_texture: InheritableVariable::new(None),
        }
    }
}

impl TypeUuidProvider for SpotLight {
    fn type_uuid() -> Uuid {
        uuid!("9856a3c1-ced7-47ec-b682-4dc4dea89d8f")
    }
}

impl SpotLight {
    /// Returns a reference to base light.
    pub fn base_light_ref(&self) -> &BaseLight {
        &self.base_light
    }

    /// Returns a reference to base light.
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
    pub fn set_hotspot_cone_angle(&mut self, cone_angle: f32) -> f32 {
        self.hotspot_cone_angle.set(cone_angle.abs())
    }

    /// Sets new falloff angle range for spot light.
    #[inline]
    pub fn set_falloff_angle_delta(&mut self, delta: f32) -> f32 {
        self.falloff_angle_delta.set(delta)
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
    pub fn set_shadow_bias(&mut self, bias: f32) -> f32 {
        self.shadow_bias.set(bias)
    }

    /// Returns current value of shadow bias.
    pub fn shadow_bias(&self) -> f32 {
        *self.shadow_bias
    }

    /// Sets maximum distance at which light intensity will be zero. Intensity
    /// of light will be calculated using inverse square root law.
    #[inline]
    pub fn set_distance(&mut self, distance: f32) -> f32 {
        self.distance.set(distance.abs())
    }

    /// Returns maximum distance of light.
    #[inline]
    pub fn distance(&self) -> f32 {
        *self.distance
    }

    /// Set cookie texture. Also called gobo this texture gets projected
    /// by the spot light.
    #[inline]
    pub fn set_cookie_texture(&mut self, texture: Option<Texture>) -> Option<Texture> {
        self.cookie_texture.set(texture)
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
    crate::impl_query_component!(base_light: BaseLight);

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::unit()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        self.base_light.restore_resources(resource_manager.clone());

        let mut state = resource_manager.state();
        let texture_container = &mut state.containers_mut().textures;
        texture_container.try_restore_inheritable_resource(&mut self.cookie_texture);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
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
    use crate::core::reflect::Reflect;
    use crate::core::variable::try_inherit_properties;
    use crate::{
        resource::texture::test::create_test_texture,
        scene::{
            base::{test::check_inheritable_properties_equality, BaseBuilder},
            light::{
                spot::{SpotLight, SpotLightBuilder},
                BaseLightBuilder,
            },
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

        try_inherit_properties(child.as_reflect_mut(), parent.as_reflect()).unwrap();

        let parent = parent.cast::<SpotLight>().unwrap();

        check_inheritable_properties_equality(&child.base_light.base, &parent.base_light.base);
        check_inheritable_properties_equality(&child.base_light, &parent.base_light);
        check_inheritable_properties_equality(&child, parent);
    }
}
