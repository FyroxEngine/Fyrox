//! Rectangle is the simplest "2D" node, it can be used to create "2D" graphics. 2D is in quotes
//! here because the node is actually a 3D node, like everything else in the engine.
//!
//! See [`Rectangle`] docs for more info.

use crate::{
    core::{
        color::Color,
        inspect::{Inspect, PropertyInfo},
        math::{aabb::AxisAlignedBoundingBox, Rect},
        pool::Handle,
        reflect::Reflect,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        graph::{map::NodeHandleMap, Graph},
        node::{Node, NodeTrait, TypeUuidProvider},
    },
};
use std::ops::{Deref, DerefMut};

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
///
/// # Specifying region for rendering
///
/// You can specify a portion of the texture that will be used for rendering using [`Self::set_uv_rect`]
/// method. This is especially useful if you need to create sprite sheet animation, you use the single
/// image, but just changing portion for rendering. Keep in mind that the coordinates are normalized
/// which means `[0; 0]` corresponds to top-left corner of the texture and `[1; 1]` corresponds to
/// right-bottom corner.
#[derive(Visit, Inspect, Reflect, Debug, Clone)]
pub struct Rectangle {
    base: Base,

    #[inspect(deref)]
    #[reflect(setter = "set_texture")]
    #[visit(optional)] // Backward compatibility
    texture: InheritableVariable<Option<Texture>>,

    #[inspect(deref)]
    #[reflect(setter = "set_color")]
    color: InheritableVariable<Color>,

    #[inspect(deref)]
    #[reflect(setter = "set_uv_rect")]
    #[visit(optional)] // Backward compatibility
    uv_rect: InheritableVariable<Rect<f32>>,
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            base: Default::default(),
            texture: Default::default(),
            color: Default::default(),
            uv_rect: InheritableVariable::new(Rect::new(0.0, 0.0, 1.0, 1.0)),
        }
    }
}

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

impl TypeUuidProvider for Rectangle {
    fn type_uuid() -> Uuid {
        uuid!("bb57b5e0-367a-4490-bf30-7f547407d5b5")
    }
}

impl Rectangle {
    /// Returns a texture used by the rectangle.
    pub fn texture(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }

    /// Returns a texture used by the rectangle.
    pub fn texture_value(&self) -> Option<Texture> {
        (*self.texture).clone()
    }

    /// Sets new texture for the rectangle.
    pub fn set_texture(&mut self, texture: Option<Texture>) -> Option<Texture> {
        self.texture.set(texture)
    }

    /// Returns current color of the rectangle.
    pub fn color(&self) -> Color {
        *self.color
    }

    /// Sets color of the rectangle.
    pub fn set_color(&mut self, color: Color) -> Color {
        self.color.set(color)
    }

    /// Returns a rectangle that defines the region in texture which will be rendered. The coordinates are normalized
    /// which means `[0; 0]` corresponds to top-left corner of the texture and `[1; 1]` corresponds to right-bottom
    /// corner.
    pub fn uv_rect(&self) -> Rect<f32> {
        *self.uv_rect
    }

    /// Sets a rectangle that defines the region in texture which will be rendered. The coordinates are normalized
    /// which means `[0; 0]` corresponds to top-left corner of the texture and `[1; 1]` corresponds to right-bottom
    /// corner.
    ///
    /// The coordinates can exceed `[1; 1]` boundary to create tiling effect (keep in mind that tiling should be
    /// enabled in texture options).
    ///
    /// The default value is `(0, 0, 1, 1)` rectangle which corresponds to entire texture.
    pub fn set_uv_rect(&mut self, uv_rect: Rect<f32>) -> Rect<f32> {
        self.uv_rect.set(uv_rect)
    }
}

impl NodeTrait for Rectangle {
    crate::impl_query_component!();

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::unit()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        self.base.restore_resources(resource_manager.clone());

        resource_manager
            .state()
            .containers_mut()
            .textures
            .try_restore_inheritable_resource(&mut self.texture);
    }

    fn remap_handles(&mut self, old_new_mapping: &NodeHandleMap) {
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
    uv_rect: Rect<f32>,
}

impl RectangleBuilder {
    /// Creates new rectangle builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            texture: None,
            color: Color::WHITE,
            uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
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

    /// Sets desired portion of the texture for the rectangle. See [`Rectangle::set_uv_rect`]
    /// for more info.
    pub fn with_uv_rect(mut self, uv_rect: Rect<f32>) -> Self {
        self.uv_rect = uv_rect;
        self
    }

    /// Creates new [`Rectangle`] instance.
    pub fn build_rectangle(self) -> Rectangle {
        Rectangle {
            base: self.base_builder.build_base(),
            texture: self.texture.into(),
            color: self.color.into(),
            uv_rect: self.uv_rect.into(),
        }
    }

    /// Creates new [`Rectangle`] instance.
    pub fn build_node(self) -> Node {
        Node::new(self.build_rectangle())
    }

    /// Creates new [`Rectangle`] instance and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

#[cfg(test)]
mod test {
    use crate::core::reflect::Reflect;
    use crate::core::variable::try_inherit_properties;
    use crate::{
        core::color::Color,
        resource::texture::test::create_test_texture,
        scene::{
            base::{test::check_inheritable_properties_equality, BaseBuilder},
            dim2::rectangle::{Rectangle, RectangleBuilder},
        },
    };

    #[test]
    fn test_rectangle_inheritance() {
        let parent = RectangleBuilder::new(BaseBuilder::new())
            .with_color(Color::opaque(1, 2, 3))
            .with_texture(create_test_texture())
            .build_node();

        let mut child = RectangleBuilder::new(BaseBuilder::new()).build_rectangle();

        try_inherit_properties(child.as_reflect_mut(), parent.as_reflect()).unwrap();

        let parent = parent.cast::<Rectangle>().unwrap();

        check_inheritable_properties_equality(&child.base, &parent.base);
        check_inheritable_properties_equality(&child, parent);
    }
}
