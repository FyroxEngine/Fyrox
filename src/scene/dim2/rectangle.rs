//! Rectangle is the simplest "2D" node, it can be used to create "2D" graphics. 2D is in quotes
//! here because the node is actually a 3D node, like everything else in the engine.
//!
//! See [`Rectangle`] docs for more info.

use crate::scene::node::NodeTrait;
use crate::{
    core::{
        color::Color,
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    impl_directly_inheritable_entity_trait,
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
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

/// Rectangle is the simplest "2D" node, it can be used to create "2D" graphics. 2D is in quotes
/// here because the node is actually a 3D node, like everything else in the engine.
///
/// ## Performance
///
/// Rectangles use specialized renderer that is heavily optimized to render tons of rectangles at
/// once, so you can use rectangles almost for everything in 2D games.
///
/// ## Limitations
///
/// Rectangle nodes does not support custom materials - it is a simplified version of a Mesh node
/// that allows you draw a rectangle with a texture and a color. Its main purpose is to be able to
/// start making games as quick as possible without diving too deep into details (shaders, render
/// passes, etc.). You can still create a "rectangle" with custom material, use Mesh node with
/// single rectangle surface:
///
/// ```rust
/// use fyrox::{
///     core::{
///         algebra::{Matrix4, Vector3},
///         parking_lot::Mutex,
///         pool::Handle,
///     },
///     material::Material,
///     scene::{
///         base::BaseBuilder,
///         graph::Graph,
///         mesh::{
///             surface::{SurfaceBuilder, SurfaceData},
///             MeshBuilder, RenderPath,
///         },
///         node::Node,
///         transform::TransformBuilder,
///     },
/// };
/// use std::sync::Arc;
///
/// fn create_rect_with_custom_material(
///     graph: &mut Graph,
///     material: Arc<Mutex<Material>>,
/// ) -> Handle<Node> {
///     MeshBuilder::new(
///         BaseBuilder::new().with_local_transform(
///             TransformBuilder::new()
///                 .with_local_scale(Vector3::new(0.4, 0.2, 1.0))
///                 .build(),
///         ),
///     )
///     .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(
///         SurfaceData::make_quad(&Matrix4::identity()),
///     )))
///     .with_material(material)
///     .build()])
///     .with_render_path(RenderPath::Forward)
///     .build(graph)
/// }
/// ```
///
/// This will effectively "mimic" the Rectangle node, but will allow you to use the full power of
/// custom shaders. Keep in mind that Mesh nodes will be rendered via Deferred Renderer, while
/// Rectangle nodes rendered with specialized renderer, that might result in some graphical artifacts.
///
/// Rectangle nodes has limited lighting support, it means that they still will be lit by standard
/// scene lights, but it will be a very simple diffuse lighting without any "physically correct"
/// lighting. This is perfectly ok for 95% of 2D games, if you want to add custom lighting then
/// you should use custom shader.
#[derive(Visit, Inspect, Debug, Clone, Default)]
pub struct Rectangle {
    base: Base,

    #[inspect(getter = "Deref::deref")]
    texture: TemplateVariable<Option<Texture>>,

    #[inspect(getter = "Deref::deref")]
    color: TemplateVariable<Color>,
}

impl_directly_inheritable_entity_trait!(Rectangle;
    texture,
    color
);

impl Deref for Rectangle {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Rectangle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Rectangle {
    pub fn type_uuid() -> Uuid {
        Uuid::from_str("bb57b5e0-367a-4490-bf30-7f547407d5b5").unwrap()
    }

    /// Returns a texture used by the rectangle.
    pub fn texture(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }

    /// Returns a texture used by the rectangle.
    pub fn texture_value(&self) -> Option<Texture> {
        (*self.texture).clone()
    }

    /// Sets new texture for the rectangle.
    pub fn set_texture(&mut self, texture: Option<Texture>) {
        self.texture.set(texture);
    }

    /// Returns current color of the rectangle.
    pub fn color(&self) -> Color {
        *self.color
    }

    /// Sets color of the rectangle.
    pub fn set_color(&mut self, color: Color) {
        self.color.set(color);
    }
}

impl NodeTrait for Rectangle {
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.local_bounding_box()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        resource_manager
            .state()
            .containers_mut()
            .textures
            .try_restore_template_resource(&mut self.texture);
    }

    fn reset_inheritable_properties(&mut self) {
        self.base.reset_inheritable_properties();
        self.reset_self_inheritable_properties();
    }

    // Prefab inheritance resolving.
    fn inherit(&mut self, parent: &Node) -> Result<(), InheritError> {
        self.base.inherit_properties(parent)?;
        if let Some(parent) = parent.cast::<Self>() {
            self.try_inherit_self_properties(parent)?;
        }
        Ok(())
    }

    fn remap_handles(&mut self, old_new_mapping: &FxHashMap<Handle<Node>, Handle<Node>>) {
        self.base.remap_handles(old_new_mapping);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}

/// Allows you to create rectangle in declarative manner.
pub struct RectangleBuilder {
    base_builder: BaseBuilder,
    texture: Option<Texture>,
    color: Color,
}

impl RectangleBuilder {
    /// Creates new rectangle builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            texture: None,
            color: Color::WHITE,
        }
    }

    /// Sets desired texture of the rectangle.
    pub fn with_texture(mut self, texture: Texture) -> Self {
        self.texture = Some(texture);
        self
    }

    /// Sets desired color of the rectangle.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Creates new [`Rectangle`] instance.
    pub fn build_rectangle(self) -> Rectangle {
        Rectangle {
            base: self.base_builder.build_base(),
            texture: self.texture.into(),
            color: self.color.into(),
        }
    }

    /// Creates new [`Node::Rectangle`] instance.
    pub fn build_node(self) -> Node {
        Node::new(self.build_rectangle())
    }

    /// Creates new [`Node::Rectangle`] instance and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

#[cfg(test)]
mod test {
    use crate::scene::node::NodeTrait;
    use crate::{
        core::color::Color,
        resource::texture::test::create_test_texture,
        scene::{
            base::{test::check_inheritable_properties_equality, BaseBuilder},
            dim2::rectangle::RectangleBuilder,
            node::Node,
        },
    };

    #[test]
    fn test_rectangle_inheritance() {
        let parent = RectangleBuilder::new(BaseBuilder::new())
            .with_color(Color::opaque(1, 2, 3))
            .with_texture(create_test_texture())
            .build_node();

        let mut child = RectangleBuilder::new(BaseBuilder::new()).build_rectangle();

        child.inherit(&parent).unwrap();

        if let Node::Rectangle(parent) = parent {
            check_inheritable_properties_equality(&child.base, &parent.base);
            check_inheritable_properties_equality(&child, &parent);
        } else {
            unreachable!()
        }
    }
}
