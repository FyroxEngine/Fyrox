//! Point light can be represented as light bulb which hangs on wire - it is
//! spherical light source which emits light in all directions. It has single
//! parameter - radius at which intensity will be zero. Intensity of light will
//! be calculated using inverse square root law.
//!
//! # Light scattering
//!
//! Point light support light scattering feature - it means that you'll see light
//! volume as well as lighted surfaces. Simple example from real life: light bulb
//! in the fog. This effect significantly improves perception of light, but should
//! be used carefully with sane values of light scattering, otherwise you'll get
//! bright glowing sphere instead of slightly visible light volume.
//!
//! # Performance notes
//!
//! Point lights supports shadows, but keep in mind - they're very expensive and
//! can easily ruin performance of your game, especially on low-end hardware. Light
//! scattering is relatively heavy too.

use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        uuid::Uuid,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    impl_directly_inheritable_entity_trait,
    scene::{
        base::Base,
        graph::Graph,
        light::{BaseLight, BaseLightBuilder},
        node::{Node, NodeTrait},
        variable::{InheritError, TemplateVariable},
        DirectlyInheritableEntity,
    },
};
use fxhash::FxHashMap;
use std::{
    ops::{Deref, DerefMut},
    str::FromStr,
};

/// See module docs.
#[derive(Debug, Inspect, Clone)]
pub struct PointLight {
    base_light: BaseLight,

    #[inspect(min_value = 0.0, step = 0.001, getter = "Deref::deref")]
    shadow_bias: TemplateVariable<f32>,

    #[inspect(min_value = 0.0, step = 0.1, getter = "Deref::deref")]
    radius: TemplateVariable<f32>,
}

impl_directly_inheritable_entity_trait!(PointLight;
    shadow_bias,
    radius
);

impl Deref for PointLight {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base_light.base
    }
}

impl DerefMut for PointLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_light.base
    }
}

impl PointLight {
    pub fn type_uuid() -> Uuid {
        Uuid::from_str("c81dcc31-7cb9-465f-abd9-b385ac6f4d37").unwrap()
    }

    pub fn base_light_ref(&self) -> &BaseLight {
        &self.base_light
    }

    pub fn base_light_mut(&mut self) -> &mut BaseLight {
        &mut self.base_light
    }

    /// Sets radius of point light. This parameter also affects radius of spherical
    /// light volume that is used in light scattering.
    #[inline]
    pub fn set_radius(&mut self, radius: f32) {
        self.radius.set(radius.abs());
    }

    /// Returns radius of point light.
    #[inline]
    pub fn radius(&self) -> f32 {
        *self.radius
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
}

impl NodeTrait for PointLight {
    crate::impl_query_component!(base_light: BaseLight);

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base_light.base.local_bounding_box()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base_light.base.world_bounding_box()
    }

    // Prefab inheritance resolving.
    fn inherit(&mut self, parent: &Node) -> Result<(), InheritError> {
        if let Some(parent) = parent.cast::<Self>() {
            self.base_light.inherit(parent.base_light_ref())?;
            self.try_inherit_self_properties(parent)?;
        }
        Ok(())
    }

    fn reset_inheritable_properties(&mut self) {
        self.base_light.reset_inheritable_properties();
        self.reset_self_inheritable_properties();
    }

    fn restore_resources(&mut self, _resource_manager: ResourceManager) {}

    fn remap_handles(&mut self, old_new_mapping: &FxHashMap<Handle<Node>, Handle<Node>>) {
        self.base_light.remap_handles(old_new_mapping);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}

impl Visit for PointLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.base_light.visit("BaseLight", visitor)?;
        self.radius.visit("Radius", visitor)?;
        self.shadow_bias.visit("ShadowBias", visitor)?;

        visitor.leave_region()
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            base_light: Default::default(),
            shadow_bias: TemplateVariable::new(0.025),
            radius: TemplateVariable::new(10.0),
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
            radius: self.radius.into(),
            shadow_bias: self.shadow_bias.into(),
        }
    }

    /// Builds new instance of point light node.
    pub fn build_node(self) -> Node {
        Node::new(self.build_point_light())
    }

    /// Builds new instance of point light and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

#[cfg(test)]
mod test {
    use crate::scene::{
        base::{test::check_inheritable_properties_equality, BaseBuilder},
        light::{point::PointLightBuilder, BaseLightBuilder, Light},
        node::{Node, NodeTrait},
    };

    #[test]
    fn test_point_light_inheritance() {
        let parent = PointLightBuilder::new(BaseLightBuilder::new(BaseBuilder::new()))
            .with_radius(1.0)
            .with_shadow_bias(0.1)
            .build_node();

        let mut child =
            PointLightBuilder::new(BaseLightBuilder::new(BaseBuilder::new())).build_point_light();

        child.inherit(&parent).unwrap();

        if let Node::Light(Light::Point(parent)) = parent {
            check_inheritable_properties_equality(&child.base, &parent.base);
            check_inheritable_properties_equality(&child.base_light, &parent.base_light);
            check_inheritable_properties_equality(&child, &parent);
        } else {
            unreachable!()
        }
    }
}
