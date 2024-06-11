//! Contains all structures and methods to create and manage scene graph nodes.
//!
//! For more info see [`Node`]

#![warn(missing_docs)]

use crate::resource::model::Model;
use crate::{
    asset::untyped::UntypedResource,
    core::{
        algebra::{Matrix4, Vector2},
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::prelude::*,
        uuid::Uuid,
        uuid_provider, variable,
        variable::mark_inheritable_properties_non_modified,
        visitor::{Visit, VisitResult, Visitor},
    },
    graph::SceneGraphNode,
    renderer::bundle::RenderContext,
    resource::model::ModelResource,
    scene::{
        self,
        animation::{absm::AnimationBlendingStateMachine, AnimationPlayer},
        base::Base,
        camera::Camera,
        debug::SceneDrawingContext,
        decal::Decal,
        dim2::{self, rectangle::Rectangle},
        graph::{self, Graph, GraphUpdateSwitches, NodePool},
        light::{directional::DirectionalLight, point::PointLight, spot::SpotLight},
        mesh::Mesh,
        navmesh::NavigationalMesh,
        particle_system::ParticleSystem,
        pivot::Pivot,
        ragdoll::Ragdoll,
        sound::{context::SoundContext, listener::Listener, Sound},
        sprite::Sprite,
        terrain::Terrain,
        Scene,
    },
};
use fyrox_core::{ComponentProvider, NameProvider};
use fyrox_resource::Resource;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    ops::{Deref, DerefMut},
};

pub mod constructor;
pub mod container;

/// A set of useful methods that is possible to auto-implement.
pub trait BaseNodeTrait: Any + Debug + Deref<Target = Base> + DerefMut + Send {
    /// This method creates raw copy of a node, it should never be called in normal circumstances
    /// because internally nodes may (and most likely will) contain handles to other nodes. To
    /// correctly clone a node you have to use [copy_node](struct.Graph.html#method.copy_node).
    fn clone_box(&self) -> Node;

    /// Casts self as `Any`
    fn as_any_ref(&self) -> &dyn Any;

    /// Casts self as `Any`
    fn as_any_ref_mut(&mut self) -> &mut dyn Any;
}

impl<T> BaseNodeTrait for T
where
    T: Clone + NodeTrait + 'static,
{
    fn clone_box(&self) -> Node {
        Node(Box::new(self.clone()))
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_ref_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A data for synchronization. See [`NodeTrait::sync_native`] for more info.
pub struct SyncContext<'a, 'b> {
    /// A reference to a pool with nodes from a scene graph.
    pub nodes: &'a NodePool,
    /// A mutable reference to 3D physics world.
    pub physics: &'a mut graph::physics::PhysicsWorld,
    /// A mutable reference to 2D physics world.
    pub physics2d: &'a mut dim2::physics::PhysicsWorld,
    /// A mutable reference to sound context.
    pub sound_context: &'a mut SoundContext,
    /// A reference to graph update switches. See [`GraphUpdateSwitches`] for more info.
    pub switches: Option<&'b GraphUpdateSwitches>,
}

/// A data for update tick. See [`NodeTrait::update`] for more info.
pub struct UpdateContext<'a> {
    /// Size of client area of the window.
    pub frame_size: Vector2<f32>,
    /// A time that have passed since last update call.
    pub dt: f32,
    /// A reference to a pool with nodes from a scene graph.
    pub nodes: &'a mut NodePool,
    /// A mutable reference to 3D physics world.
    pub physics: &'a mut graph::physics::PhysicsWorld,
    /// A mutable reference to 2D physics world.
    pub physics2d: &'a mut dim2::physics::PhysicsWorld,
    /// A mutable reference to sound context.
    pub sound_context: &'a mut SoundContext,
}

/// Implements [`NodeTrait::query_component_ref`] and [`NodeTrait::query_component_mut`] in a much
/// shorter way.
#[macro_export]
macro_rules! impl_query_component {
    ($($comp_field:ident: $comp_type:ty),*) => {
        fn query_component_ref(&self, type_id: std::any::TypeId) -> Option<&dyn std::any::Any> {
            if type_id == std::any::TypeId::of::<Self>() {
                return Some(self);
            }

            $(
                if type_id == std::any::TypeId::of::<$comp_type>() {
                    return Some(&self.$comp_field)
                }
            )*

            None
        }

        fn query_component_mut(
            &mut self,
            type_id: std::any::TypeId,
        ) -> Option<&mut dyn std::any::Any> {
            if type_id == std::any::TypeId::of::<Self>() {
                return Some(self);
            }

            $(
                if type_id == std::any::TypeId::of::<$comp_type>() {
                    return Some(&mut self.$comp_field)
                }
            )*

            None
        }
    };
}

/// An enumeration, that contains all possible render data collection strategies.
#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub enum RdcControlFlow {
    /// Continue collecting render data of descendant nodes.
    Continue,
    /// Breaks further render data collection of descendant nodes.
    Break,
}

/// A main trait for any scene graph node.
pub trait NodeTrait: BaseNodeTrait + Reflect + Visit {
    /// Allows a node to provide access to inner components.
    fn query_component_ref(&self, type_id: TypeId) -> Option<&dyn Any>;

    /// Allows a node to provide access to inner components.
    fn query_component_mut(&mut self, type_id: TypeId) -> Option<&mut dyn Any>;

    /// Returns axis-aligned bounding box in **local space** of the node.
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox;

    /// Returns axis-aligned bounding box in **world space** of the node.
    ///
    /// # Important notes
    ///
    /// World bounding box will become valid **only** after first `update` call of the parent scene.
    /// It is because to calculate world bounding box we must get world transform first, but it
    /// can be calculated with a knowledge of parent world transform, so node on its own cannot know
    /// its world bounding box without additional information.
    fn world_bounding_box(&self) -> AxisAlignedBoundingBox;

    /// Returns actual type id. It will be used for serialization, the type will be saved together
    /// with node's data allowing you to create correct node instance on deserialization.
    fn id(&self) -> Uuid;

    /// Gives an opportunity to perform clean up after the node was extracted from the scene graph
    /// (or deleted).
    fn on_removed_from_graph(&mut self, #[allow(unused_variables)] graph: &mut Graph) {}

    /// The method is called when the node was detached from its parent node.
    fn on_unlink(&mut self, #[allow(unused_variables)] graph: &mut Graph) {}

    /// Synchronizes internal state of the node with components of scene graph. It has limited usage
    /// and mostly allows you to sync the state of backing entity with the state of the node.
    /// For example the engine use it to sync native rigid body properties after some property was
    /// changed in the [`crate::scene::rigidbody::RigidBody`] node.
    fn sync_native(
        &self,
        #[allow(unused_variables)] self_handle: Handle<Node>,
        #[allow(unused_variables)] context: &mut SyncContext,
    ) {
    }

    /// Called when node's global transform changes.
    fn sync_transform(
        &self,
        #[allow(unused_variables)] new_global_transform: &Matrix4<f32>,
        _context: &mut SyncContext,
    ) {
    }

    /// The methods is used to manage lifetime of scene nodes, depending on their internal logic.
    fn is_alive(&self) -> bool {
        true
    }

    /// Updates internal state of the node.
    fn update(&mut self, #[allow(unused_variables)] context: &mut UpdateContext) {}

    /// Allows the node to emit a set of render data. This is a high-level rendering method which can only
    /// do culling and provide render data. Render data is just a surface (vertex + index buffers) and a
    /// material.
    fn collect_render_data(
        &self,
        #[allow(unused_variables)] ctx: &mut RenderContext,
    ) -> RdcControlFlow {
        RdcControlFlow::Continue
    }

    /// Allows the node to draw simple shapes to visualize internal data structures for debugging purposes.
    fn debug_draw(&self, #[allow(unused_variables)] ctx: &mut SceneDrawingContext) {}

    /// Validates internal state of a scene node. It can check handles validity, if a handle "points"
    /// to a node of particular type, if node's parameters are in range, etc. It's main usage is to
    /// provide centralized diagnostics for scene graph.
    fn validate(&self, #[allow(unused_variables)] scene: &Scene) -> Result<(), String> {
        Ok(())
    }
}

/// Node is the basic building block for 3D scenes. It has multiple variants, but all of them share some
/// common functionality:
///
/// - Local and global [transform](super::transform::Transform)
/// - Info about connections with other nodes in scene
/// - Visibility state - local and global
/// - Name and tags
/// - Level of details
/// - Physics binding mode
///
/// The exact functionality depends on variant of the node, check the respective docs for a variant you
/// interested in.
///
/// # Hierarchy
///
/// Nodes can be connected with other nodes, so a child node will be moved/rotate/scaled together with parent
/// node. This has some analogy in real world - imagine a pen with a cap. The pen will be the parent node in
/// the hierarchy and the cap will be child node. When you moving the pen, the cap moves with it only if it
/// attached to the pen. The same principle works with scene nodes.
///
/// # Transform
///
/// The node has two kinds of transform - local and global. Local transform defines where the node is located
/// (translation) relative to origin, how much it is scaled (in percent) and rotated (around any arbitrary axis).
/// Global transform is almost the same, but it also includes the whole chain of transforms of parent nodes.
/// In the previous example with the pen, the cap has its own local transform which tells how much it should be
/// moved from origin to be exactly on top of the pen. But global transform of the cap includes transform of the
/// pen. So if you move the pen, the local transform of the cap will remain the same, but global transform will
/// include the transform of the pen.
///
/// # Name and tag
///
/// The node can have arbitrary name and tag. Both could be used to search the node in the graph. Unlike the name,
/// tag could be used to store some gameplay information about the node. For example you can place a [`Mesh`] node
/// that represents health pack model and it will have a name "HealthPack", in the tag you could put additional info
/// like "MediumPack", "SmallPack", etc. So 3D model will not have "garbage" in its name, it will be stored inside tag.
///
/// # Visibility
///
/// The now has two kinds of visibility - local and global. As with transform, everything here is pretty similar.
/// Local visibility defines if the node is visible as if it would be rendered alone, global visibility includes
/// the combined visibility of entire chain of parent nodes.
///
/// Please keep in mind that "visibility" here means some sort of a "switch" that tells the renderer whether to draw
/// the node or not.
///
/// # Level of details
///
/// The node could control which children nodes should be drawn based on the distance to a camera, this is so called
/// level of detail functionality. There is a separate article about LODs, it can be found [here](super::base::LevelOfDetail).
///
/// # Property inheritance
///
/// Property inheritance is used to propagate changes of unmodified properties from a prefab to its instances. For example,
/// you can change scale of a node in a prefab and its instances will have the same scale too, unless the scale is
/// set explicitly in an instance. Such feature allows you to tweak instances, add some unique details to them, but take
/// general properties from parent prefabs.
///
/// ## Important notes
///
/// Property inheritance uses [`variable::InheritableVariable`] to wrap actual property value, such wrapper stores a tiny
/// bitfield for flags that can tell whether or not the property was modified. Property inheritance system is then uses
/// reflection to "walk" over each property in the node and respective parent resource (from which the node was instantiated from,
/// if any) and checks if the property was modified. If it was modified, its value remains the same, otherwise the value
/// from the respective property in a "parent" node in the parent resource is copied to the property of the node. Such
/// process is then repeated for all levels of inheritance, starting from the root and going down to children in inheritance
/// hierarchy.
///
/// The most important thing is that [`variable::InheritableVariable`] will save (serialize) its value only if it was marked
/// as modified (we don't need to save anything if it can be fetched from parent). This saves **a lot** of disk space for
/// inherited assets (in some extreme cases memory consumption can be reduced by 90%, if there's only few properties modified).
/// This fact requires "root" (nodes that are **not** instances) nodes to have **all** inheritable properties to be marked as
/// modified, otherwise their values won't be saved, which is indeed wrong.
///
/// When a node is instantiated from some model resource, all its properties become non-modified. Which allows the inheritance
/// system to correctly handle redundant information.
///
/// Such implementation of property inheritance has its drawbacks, major one is: each instance still holds its own copy of
/// of every field, even those inheritable variables which are non-modified. Which means that there's no benefits of RAM
/// consumption, only disk space usage is reduced.
#[derive(Debug)]
pub struct Node(Box<dyn NodeTrait>);

impl Clone for Node {
    fn clone(&self) -> Self {
        self.0.clone_box()
    }
}

impl SceneGraphNode for Node {
    type Base = Base;
    type SceneGraph = Graph;
    type ResourceData = Model;

    fn base(&self) -> &Self::Base {
        self.0.deref()
    }

    fn set_base(&mut self, base: Self::Base) {
        ***self = base;
    }

    fn is_resource_instance_root(&self) -> bool {
        self.is_resource_instance_root
    }

    fn original_handle_in_resource(&self) -> Handle<Self> {
        self.original_handle_in_resource
    }

    fn set_original_handle_in_resource(&mut self, handle: Handle<Self>) {
        self.original_handle_in_resource = handle;
    }

    fn resource(&self) -> Option<Resource<Self::ResourceData>> {
        self.resource.clone()
    }

    fn self_handle(&self) -> Handle<Self> {
        self.self_handle
    }

    fn parent(&self) -> Handle<Self> {
        self.parent
    }

    fn children(&self) -> &[Handle<Self>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Handle<Self>] {
        &mut self.children
    }
}

impl NameProvider for Node {
    fn name(&self) -> &str {
        &self.0.name
    }
}

impl ComponentProvider for Node {
    fn query_component_ref(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.0.query_component_ref(type_id)
    }

    fn query_component_mut(&mut self, type_id: TypeId) -> Option<&mut dyn Any> {
        self.0.query_component_mut(type_id)
    }
}

uuid_provider!(Node = "a9bc5231-155c-4564-b0ca-f23972673925");

impl Deref for Node {
    type Target = dyn NodeTrait;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

/// Defines as_(variant), as_mut_(variant) and is_(variant) methods.
#[macro_export]
macro_rules! define_is_as {
    ($typ:ty => fn $is:ident, fn $as_ref:ident, fn $as_mut:ident) => {
        /// Returns true if node is instance of given type.
        #[inline]
        pub fn $is(&self) -> bool {
            self.cast::<$typ>().is_some()
        }

        /// Tries to cast shared reference to a node to given type, panics if
        /// cast is not possible.
        #[inline]
        pub fn $as_ref(&self) -> &$typ {
            self.cast::<$typ>()
                .unwrap_or_else(|| panic!("Cast to {} failed!", stringify!($kind)))
        }

        /// Tries to cast mutable reference to a node to given type, panics if
        /// cast is not possible.
        #[inline]
        pub fn $as_mut(&mut self) -> &mut $typ {
            self.cast_mut::<$typ>()
                .unwrap_or_else(|| panic!("Cast to {} failed!", stringify!($kind)))
        }
    };
}

impl Node {
    /// Creates a new node instance from any type that implements [`NodeTrait`].
    #[inline]
    pub fn new<T: NodeTrait>(node: T) -> Self {
        Self(Box::new(node))
    }

    /// Performs downcasting to a particular type.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use fyrox_impl::scene::mesh::Mesh;
    /// # use fyrox_impl::scene::node::Node;
    ///
    /// fn node_as_mesh_ref(node: &Node) -> &Mesh {
    ///     node.cast::<Mesh>().expect("Expected to be an instance of Mesh")
    /// }
    /// ```
    #[inline]
    pub fn cast<T: NodeTrait>(&self) -> Option<&T> {
        self.0.as_any_ref().downcast_ref::<T>()
    }

    /// Performs downcasting to a particular type.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use fyrox_impl::scene::mesh::Mesh;
    /// # use fyrox_impl::scene::node::Node;
    ///
    /// fn node_as_mesh_mut(node: &mut Node) -> &mut Mesh {
    ///     node.cast_mut::<Mesh>().expect("Expected to be an instance of Mesh")
    /// }
    /// ```
    #[inline]
    pub fn cast_mut<T: NodeTrait>(&mut self) -> Option<&mut T> {
        self.0.as_any_ref_mut().downcast_mut::<T>()
    }

    /// Allows a node to provide access to a component of specified type.
    ///
    /// # Example
    ///
    /// A good example is a light source node, it gives access to internal `BaseLight`:
    ///
    /// ```rust
    /// # use fyrox_impl::scene::light::BaseLight;
    /// # use fyrox_impl::scene::light::directional::DirectionalLight;
    /// # use fyrox_impl::scene::node::{Node};
    ///
    /// fn base_light_ref(directional_light: &Node) -> &BaseLight {
    ///     directional_light.query_component_ref::<BaseLight>().expect("Must have base light")
    /// }
    ///
    /// ```
    ///
    /// Some nodes could also provide access to inner components, check documentation of a node.
    #[inline]
    pub fn query_component_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.0
            .query_component_ref(TypeId::of::<T>())
            .and_then(|c| c.downcast_ref::<T>())
    }

    /// Allows a node to provide access to a component of specified type.
    ///
    /// # Example
    ///
    /// A good example is a light source node, it gives access to internal `BaseLight`:
    ///
    /// ```rust
    /// # use fyrox_impl::scene::light::BaseLight;
    /// # use fyrox_impl::scene::light::directional::DirectionalLight;
    /// # use fyrox_impl::scene::node::{Node};
    ///
    /// fn base_light_mut(directional_light: &mut Node) -> &mut BaseLight {
    ///     directional_light.query_component_mut::<BaseLight>().expect("Must have base light")
    /// }
    ///
    /// ```
    ///
    /// Some nodes could also provide access to inner components, check documentation of a node.
    #[inline]
    pub fn query_component_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.0
            .query_component_mut(TypeId::of::<T>())
            .and_then(|c| c.downcast_mut::<T>())
    }

    pub(crate) fn mark_inheritable_variables_as_modified(&mut self) {
        variable::mark_inheritable_properties_modified(self, &[TypeId::of::<UntypedResource>()])
    }

    pub(crate) fn set_inheritance_data(
        &mut self,
        original_handle: Handle<Node>,
        model: ModelResource,
    ) {
        // Notify instantiated node about resource it was created from.
        self.resource = Some(model.clone());

        // Reset resource instance root flag, this is needed because a node after instantiation cannot
        // be a root anymore.
        self.is_resource_instance_root = false;

        // Reset inheritable properties, so property inheritance system will take properties
        // from parent objects on resolve stage.
        self.as_reflect_mut(&mut |reflect| {
            mark_inheritable_properties_non_modified(reflect, &[TypeId::of::<UntypedResource>()])
        });

        // Fill original handles to instances.
        self.original_handle_in_resource = original_handle;
    }

    define_is_as!(Mesh => fn is_mesh, fn as_mesh, fn as_mesh_mut);
    define_is_as!(Pivot => fn is_pivot, fn as_pivot, fn as_pivot_mut);
    define_is_as!(Camera  => fn is_camera, fn as_camera, fn as_camera_mut);
    define_is_as!(SpotLight  => fn is_spot_light, fn as_spot_light, fn as_spot_light_mut);
    define_is_as!(PointLight  => fn is_point_light, fn as_point_light, fn as_point_light_mut);
    define_is_as!(DirectionalLight  => fn is_directional_light, fn as_directional_light, fn as_directional_light_mut);
    define_is_as!(ParticleSystem => fn is_particle_system, fn as_particle_system, fn as_particle_system_mut);
    define_is_as!(Sprite  => fn is_sprite, fn as_sprite, fn as_sprite_mut);
    define_is_as!(Terrain  => fn is_terrain, fn as_terrain, fn as_terrain_mut);
    define_is_as!(Decal => fn is_decal, fn as_decal, fn as_decal_mut);
    define_is_as!(Rectangle => fn is_rectangle, fn as_rectangle, fn as_rectangle_mut);
    define_is_as!(scene::rigidbody::RigidBody  => fn is_rigid_body, fn as_rigid_body, fn as_rigid_body_mut);
    define_is_as!(scene::collider::Collider => fn is_collider, fn as_collider, fn as_collider_mut);
    define_is_as!(scene::joint::Joint  => fn is_joint, fn as_joint, fn as_joint_mut);
    define_is_as!(dim2::rigidbody::RigidBody => fn is_rigid_body2d, fn as_rigid_body2d, fn as_rigid_body2d_mut);
    define_is_as!(dim2::collider::Collider => fn is_collider2d, fn as_collider2d, fn as_collider2d_mut);
    define_is_as!(dim2::joint::Joint => fn is_joint2d, fn as_joint2d, fn as_joint2d_mut);
    define_is_as!(Sound => fn is_sound, fn as_sound, fn as_sound_mut);
    define_is_as!(Listener => fn is_listener, fn as_listener, fn as_listener_mut);
    define_is_as!(NavigationalMesh => fn is_navigational_mesh, fn as_navigational_mesh, fn as_navigational_mesh_mut);
    define_is_as!(AnimationBlendingStateMachine => fn is_absm, fn as_absm, fn as_absm_mut);
    define_is_as!(AnimationPlayer => fn is_animation_player, fn as_animation_player, fn as_animation_player_mut);
    define_is_as!(Ragdoll => fn is_ragdoll, fn as_ragdoll, fn as_ragdoll_mut);
}

impl Visit for Node {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl Reflect for Node {
    fn source_path() -> &'static str {
        file!()
    }

    fn type_name(&self) -> &'static str {
        self.0.deref().type_name()
    }

    fn doc(&self) -> &'static str {
        self.0.deref().doc()
    }

    fn assembly_name(&self) -> &'static str {
        self.0.deref().assembly_name()
    }

    fn type_assembly_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn fields_info(&self, func: &mut dyn FnMut(&[FieldInfo])) {
        self.0.deref().fields_info(func)
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self.0.into_any()
    }

    fn as_any(&self, func: &mut dyn FnMut(&dyn Any)) {
        self.0.deref().as_any(func)
    }

    fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn Any)) {
        self.0.deref_mut().as_any_mut(func)
    }

    fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
        self.0.deref().as_reflect(func)
    }

    fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
        self.0.deref_mut().as_reflect_mut(func)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        self.0.deref_mut().set(value)
    }

    fn set_field(
        &mut self,
        field: &str,
        value: Box<dyn Reflect>,
        func: &mut dyn FnMut(Result<Box<dyn Reflect>, Box<dyn Reflect>>),
    ) {
        self.0.deref_mut().set_field(field, value, func)
    }

    fn fields(&self, func: &mut dyn FnMut(&[&dyn Reflect])) {
        self.0.deref().fields(func)
    }

    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [&mut dyn Reflect])) {
        self.0.deref_mut().fields_mut(func)
    }

    fn field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
        self.0.deref().field(name, func)
    }

    fn field_mut(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
        self.0.deref_mut().field_mut(name, func)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        asset::manager::ResourceManager,
        core::{
            algebra::{Matrix4, Vector3},
            futures::executor::block_on,
            impl_component_provider,
            reflect::prelude::*,
            uuid::{uuid, Uuid},
            variable::InheritableVariable,
            visitor::{prelude::*, Visitor},
            TypeUuidProvider,
        },
        engine::{self, SerializationContext},
        resource::model::{Model, ModelResourceExtension},
        scene::{
            base::BaseBuilder,
            mesh::{
                surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
                MeshBuilder,
            },
            pivot::PivotBuilder,
            transform::TransformBuilder,
            Scene,
        },
        script::ScriptTrait,
    };
    use fyrox_graph::SceneGraph;
    use fyrox_resource::untyped::ResourceKind;
    use std::{fs, path::Path, sync::Arc};

    #[derive(Debug, Clone, Reflect, Visit, Default)]
    struct MyScript {
        some_field: InheritableVariable<String>,
        some_collection: InheritableVariable<Vec<u32>>,
    }

    impl_component_provider!(MyScript);

    impl TypeUuidProvider for MyScript {
        fn type_uuid() -> Uuid {
            uuid!("d3f66902-803f-4ace-8170-0aa485d98b40")
        }
    }

    impl ScriptTrait for MyScript {}

    fn create_scene() -> Scene {
        let mut scene = Scene::new();

        let mesh;
        PivotBuilder::new(
            BaseBuilder::new()
                .with_name("Pivot")
                .with_script(MyScript {
                    some_field: "Foobar".to_string().into(),
                    some_collection: vec![1, 2, 3].into(),
                })
                .with_children(&[{
                    mesh = MeshBuilder::new(
                        BaseBuilder::new().with_name("Mesh").with_local_transform(
                            TransformBuilder::new()
                                .with_local_position(Vector3::new(3.0, 2.0, 1.0))
                                .build(),
                        ),
                    )
                    .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
                        ResourceKind::Embedded,
                        SurfaceData::make_cone(16, 1.0, 1.0, &Matrix4::identity()),
                    ))
                    .build()])
                    .build(&mut scene.graph);
                    mesh
                }]),
        )
        .build(&mut scene.graph);

        let mesh = scene.graph[mesh].as_mesh();
        assert_eq!(mesh.surfaces().len(), 1);
        assert!(mesh.surfaces()[0].bones.is_modified());
        assert!(mesh.surfaces()[0].data.is_modified());
        assert!(mesh.surfaces()[0].material.is_modified());

        scene
    }

    fn save_scene(scene: &mut Scene, path: &Path) {
        let mut visitor = Visitor::new();
        scene.save("Scene", &mut visitor).unwrap();
        visitor.save_binary(path).unwrap();
    }

    #[test]
    fn test_property_inheritance() {
        if !Path::new("test_output").exists() {
            fs::create_dir_all("test_output").unwrap();
        }

        let root_asset_path = Path::new("test_output/root.rgs");
        let derived_asset_path = Path::new("test_output/derived.rgs");

        // Create root scene and save it.
        {
            let mut scene = create_scene();
            save_scene(&mut scene, root_asset_path);
        }

        // Initialize resource manager and re-load the scene.
        let resource_manager = ResourceManager::new(Arc::new(Default::default()));
        let serialization_context = SerializationContext::new();
        serialization_context
            .script_constructors
            .add::<MyScript>("MyScript");

        assert!(serialization_context
            .script_constructors
            .map()
            .iter()
            .any(|s| s.1.source_path == file!()));

        engine::initialize_resource_manager_loaders(
            &resource_manager,
            Arc::new(serialization_context),
        );

        let root_asset = block_on(resource_manager.request::<Model>(root_asset_path)).unwrap();

        // Create root resource instance in a derived resource.
        {
            let mut derived = Scene::new();
            root_asset.instantiate(&mut derived);
            let pivot = derived.graph.find_by_name_from_root("Pivot").unwrap().0;
            let mesh = derived.graph.find_by_name_from_root("Mesh").unwrap().0;
            // Modify something in the instance.
            let pivot = &mut derived.graph[pivot];
            pivot
                .local_transform_mut()
                .set_position(Vector3::new(1.0, 2.0, 3.0));
            let my_script = pivot.try_get_script_mut::<MyScript>().unwrap();
            my_script.some_collection.push(4);
            let mesh = derived.graph[mesh].as_mesh_mut();
            assert_eq!(
                **mesh.local_transform().position(),
                Vector3::new(3.0, 2.0, 1.0)
            );
            assert_eq!(mesh.surfaces().len(), 1);
            assert!(!mesh.surfaces()[0].bones.is_modified());
            assert!(!mesh.surfaces()[0].data.is_modified());
            assert!(!mesh.surfaces()[0].material.is_modified());
            mesh.set_cast_shadows(false);
            save_scene(&mut derived, derived_asset_path);
        }

        // Reload the derived asset and check its content.
        {
            let derived_asset =
                block_on(resource_manager.request::<Model>(derived_asset_path)).unwrap();

            let derived_data = derived_asset.data_ref();
            let derived_scene = derived_data.get_scene();
            let pivot = derived_scene
                .graph
                .find_by_name_from_root("Pivot")
                .unwrap()
                .0;
            let mesh = derived_scene
                .graph
                .find_by_name_from_root("Mesh")
                .unwrap()
                .0;
            let pivot = &derived_scene.graph[pivot];
            let my_script = pivot.try_get_script::<MyScript>().unwrap();
            assert_eq!(
                **pivot.local_transform().position(),
                Vector3::new(1.0, 2.0, 3.0)
            );
            assert_eq!(*my_script.some_field, "Foobar");
            assert_eq!(*my_script.some_collection, &[1, 2, 3, 4]);
            let mesh = derived_scene.graph[mesh].as_mesh();
            assert!(!mesh.cast_shadows());
            assert_eq!(mesh.surfaces().len(), 1);
            assert!(!mesh.surfaces()[0].bones.is_modified());
            assert!(!mesh.surfaces()[0].data.is_modified());
            assert!(!mesh.surfaces()[0].material.is_modified());
            // Mesh's local position must remain the same as in the root.
            assert_eq!(
                **mesh.local_transform().position(),
                Vector3::new(3.0, 2.0, 1.0)
            );
        }
    }
}
