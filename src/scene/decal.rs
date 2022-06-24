//! Decal is an image that gets projected to a geometry of a scene.
//!
//! For more info see [`Decal`]

use crate::scene::graph::map::NodeHandleMap;
use crate::{
    core::variable::{InheritError, TemplateVariable},
    core::{
        color::Color,
        inspect::{Inspect, PropertyInfo},
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    impl_directly_inheritable_entity_trait,
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{Node, NodeTrait, TypeUuidProvider},
        DirectlyInheritableEntity,
    },
};
use std::ops::{Deref, DerefMut};

/// Decal is an image that gets projected to a geometry of a scene. Blood splatters, bullet holes, scratches
/// etc. are done via decals.
///
/// # Size and transformations
///
/// A decal defines a cube that projects a texture on every pixel of a scene that got into the cube. Exact cube
/// size is defines by decal's `local scale`. For example, if you have a decal with scale (1.0, 2.0, 0.1) then
/// the size of the cube (in local coordinates) will be `width = 1.0`, `height = 2.0`, `depth = 0.1`. The decal
/// can be rotated as any other scene node. Its final size and orientation is defined by the chain of
/// transformations of parent nodes.
///
/// # Masking
///
/// Often you need to ensure that decal will be applied only on desired surfaces. For example a crack on the wall
/// should not affect any surrounding objects, this can be achieved by using decal mask. Each decal has layer index,
/// it will be drawn only if the index matches the index of the object that inside of decal bounds.
///
/// # Supported maps
///
/// Currently, only diffuse and normal maps are supported. Diffuse and normal maps will be automatically projected
/// on the data stored in G-Buffer.
///
/// # Limitations
///
/// Current implementation works only with Deferred render path. Custom materials that uses Forward pass should
/// implement decals manually.
///
/// # Performance
///
/// It should be noted that decals are not cheap, keep amount (and size) of decals at reasonable values! This
/// means that unused decals (bullet holes for example) must be removed after some time.
///
/// # Example
///
/// ```
/// use fyrox::{
///         engine::resource_manager::ResourceManager,
///         core::pool::Handle,
///         scene::{
///         node::Node,
///         graph::Graph,
///         decal::DecalBuilder,
///         base::BaseBuilder,
///         transform::TransformBuilder
///     },
///     core::algebra::Vector3
/// };
///
/// fn create_bullet_hole(resource_manager: ResourceManager, graph: &mut Graph) -> Handle<Node> {
///     DecalBuilder::new(
///             BaseBuilder::new()
///                 .with_local_transform(
///                     TransformBuilder::new()
///                         .with_local_scale(Vector3::new(2.0, 2.0, 2.0))
///                         .build()
///         ))
///         .with_diffuse_texture(resource_manager.request_texture("bullet_hole.png"))
///         .build(graph)
/// }
/// ```
#[derive(Debug, Visit, Default, Clone, Inspect)]
pub struct Decal {
    base: Base,

    #[inspect(getter = "Deref::deref")]
    diffuse_texture: TemplateVariable<Option<Texture>>,

    #[inspect(getter = "Deref::deref")]
    normal_texture: TemplateVariable<Option<Texture>>,

    #[inspect(getter = "Deref::deref")]
    color: TemplateVariable<Color>,

    #[inspect(min_value = 0.0, getter = "Deref::deref")]
    layer: TemplateVariable<u8>,
}

impl_directly_inheritable_entity_trait!(Decal;
    diffuse_texture,
    normal_texture,
    color,
    layer
);

impl Deref for Decal {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Decal {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl TypeUuidProvider for Decal {
    fn type_uuid() -> Uuid {
        uuid!("c4d24e48-edd1-4fb2-ad82-4b3d3ea985d8")
    }
}

impl Decal {
    /// Sets new diffuse texture.
    pub fn set_diffuse_texture(&mut self, diffuse_texture: Option<Texture>) -> Option<Texture> {
        std::mem::replace(self.diffuse_texture.get_mut(), diffuse_texture)
    }

    /// Returns current diffuse texture.
    pub fn diffuse_texture(&self) -> Option<&Texture> {
        self.diffuse_texture.as_ref()
    }

    /// Returns current diffuse texture.
    pub fn diffuse_texture_value(&self) -> Option<Texture> {
        (*self.diffuse_texture).clone()
    }

    /// Sets new normal texture.
    pub fn set_normal_texture(&mut self, normal_texture: Option<Texture>) -> Option<Texture> {
        std::mem::replace(self.normal_texture.get_mut(), normal_texture)
    }

    /// Returns current normal texture.
    pub fn normal_texture(&self) -> Option<&Texture> {
        self.normal_texture.as_ref()
    }

    /// Returns current normal texture.
    pub fn normal_texture_value(&self) -> Option<Texture> {
        (*self.normal_texture).clone()
    }

    /// Sets new color for the decal.
    pub fn set_color(&mut self, color: Color) {
        self.color.set(color);
    }

    /// Returns current color of the decal.
    pub fn color(&self) -> Color {
        *self.color
    }

    /// Sets layer index of the decal. Layer index allows you to apply decals only on desired
    /// surfaces. For example, static geometry could have `index == 0` and dynamic `index == 1`.
    /// To "filter" decals all you need to do is to set appropriate layer index to decal, for
    /// example blood splatter decal will have `index == 0` in this case. In case of dynamic
    /// objects (like bots, etc.) index will be 1.
    pub fn set_layer(&mut self, layer: u8) {
        self.layer.set(layer);
    }

    /// Returns current layer index.
    pub fn layer(&self) -> u8 {
        *self.layer
    }
}

impl NodeTrait for Decal {
    crate::impl_query_component!();

    /// Returns current **local-space** bounding box.
    #[inline]
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        // TODO: Maybe calculate AABB using frustum corners?
        self.base.local_bounding_box()
    }

    /// Returns current **world-space** bounding box.
    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    // Prefab inheritance resolving.
    fn inherit(&mut self, parent: &Node) -> Result<(), InheritError> {
        self.base.inherit_properties(parent)?;
        if let Some(parent) = parent.cast::<Self>() {
            self.try_inherit_self_properties(parent)?;
        }
        Ok(())
    }

    fn reset_inheritable_properties(&mut self) {
        self.base.reset_inheritable_properties();
        self.reset_self_inheritable_properties();
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        self.base.restore_resources(resource_manager.clone());

        let mut state = resource_manager.state();
        let texture_container = &mut state.containers_mut().textures;
        texture_container.try_restore_template_resource(&mut self.diffuse_texture);
        texture_container.try_restore_template_resource(&mut self.normal_texture);
    }

    fn remap_handles(&mut self, old_new_mapping: &NodeHandleMap) {
        self.base.remap_handles(old_new_mapping);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}

/// Allows you to create a Decal in a declarative manner.
pub struct DecalBuilder {
    base_builder: BaseBuilder,
    diffuse_texture: Option<Texture>,
    normal_texture: Option<Texture>,
    color: Color,
    layer: u8,
}

impl DecalBuilder {
    /// Creates a new instance of the builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            diffuse_texture: None,
            normal_texture: None,
            color: Color::opaque(255, 255, 255),
            layer: 0,
        }
    }

    /// Sets desired diffuse texture.
    pub fn with_diffuse_texture(mut self, diffuse_texture: Texture) -> Self {
        self.diffuse_texture = Some(diffuse_texture);
        self
    }

    /// Sets desired normal texture.
    pub fn with_normal_texture(mut self, normal_texture: Texture) -> Self {
        self.normal_texture = Some(normal_texture);
        self
    }

    /// Sets desired decal color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets desired layer index.
    pub fn with_layer(mut self, layer: u8) -> Self {
        self.layer = layer;
        self
    }

    /// Creates new Decal node.
    pub fn build_decal(self) -> Decal {
        Decal {
            base: self.base_builder.build_base(),
            diffuse_texture: self.diffuse_texture.into(),
            normal_texture: self.normal_texture.into(),
            color: self.color.into(),
            layer: self.layer.into(),
        }
    }

    /// Creates new Decal node.
    pub fn build_node(self) -> Node {
        Node::new(self.build_decal())
    }

    /// Creates new instance of Decal node and puts it in the given graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core::color::Color,
        resource::texture::test::create_test_texture,
        scene::{
            base::{test::check_inheritable_properties_equality, BaseBuilder},
            decal::{Decal, DecalBuilder},
            node::NodeTrait,
        },
    };

    #[test]
    fn test_decal_inheritance() {
        let parent = DecalBuilder::new(BaseBuilder::new())
            .with_color(Color::opaque(1, 2, 3))
            .with_layer(1)
            .with_diffuse_texture(create_test_texture())
            .with_normal_texture(create_test_texture())
            .build_node();

        let mut child = DecalBuilder::new(BaseBuilder::new()).build_decal();

        child.inherit(&parent).unwrap();

        let parent = parent.cast::<Decal>().unwrap();

        check_inheritable_properties_equality(&child.base, &parent.base);
        check_inheritable_properties_equality(&child, parent);
    }
}
