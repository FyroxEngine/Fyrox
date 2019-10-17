use std::sync::{Arc, Mutex};
use crate::{
    scene::{
        camera::Camera,
        mesh::Mesh,
        light::Light,
        particle_system::ParticleSystem,
        transform::Transform,
        sprite::Sprite,
        pivot::Pivot
    },
    resource::model::Model,
};
use rg3d_core::{
    math::{vec3::Vec3, mat4::Mat4},
    visitor::{Visit, VisitResult, Visitor},
    pool::Handle,
};

pub trait NodeTrait {
    /// Sets name of node. Can be useful to mark a node to be able to find it later on.
    fn set_name(&mut self, name: &str);

    /// Returns name of node.
    fn get_name(&self) -> &str;

    /// Returns shared reference to local transform of a node, can be used to fetch
    /// some local spatial properties, such as position, rotation, scale, etc.
    fn get_local_transform(&self) -> &Transform;

    /// Returns mutable reference to local transform of a node, can be used to set
    /// some local spatial properties, such as position, rotation, scale, etc.
    fn get_local_transform_mut(&mut self) -> &mut Transform;

    /// Sets lifetime of node in seconds, lifetime is useful for temporary objects.
    /// Example - you firing a gun, it produces two particle systems for each shot:
    /// one for gunpowder fumes and one when bullet hits some surface. These particle
    /// systems won't last very long - usually they will disappear in 1-2 seconds
    /// but nodes will still be in scene consuming precious CPU clocks. This is where
    /// lifetimes become handy - you just set appropriate lifetime for a particle
    /// system node and it will be removed from scene when time will end. This is
    /// efficient algorithm because scene holds every object in pool and allocation
    /// or deallocation of node takes very little amount of time.
    fn set_lifetime(&mut self, time_seconds: f32);

    /// Returns current lifetime of a node. Will be None if node has undefined lifetime.
    /// For more info about lifetimes see [`set_lifetime`].
    fn get_lifetime(&self) -> Option<f32>;

    /// Returns handle of parent node.
    fn get_parent(&self) -> Handle<Node>;

    /// Returns slice of handles to children nodes. This can be used, for example, to
    /// traverse tree starting from some node.
    fn get_children(&self) -> &[Handle<Node>];

    /// Returns global transform matrix, such matrix contains combined transformation
    /// of transforms of parent nodes. This is the final matrix that describes real
    /// location of object in the world.
    fn get_global_transform(&self) -> Mat4;

    /// Returns inverse of bind pose matrix. Bind pose matrix - is special matrix
    /// for bone nodes, it stores initial transform of bone node at the moment
    /// of "binding" vertices to bones.
    fn get_inv_bind_pose_transform(&self) -> Mat4;

    /// Returns resource from which this node was instantiated from.
    fn get_resource(&self) -> Option<Arc<Mutex<Model>>>;

    /// Sets local visibility of a node.
    fn set_visibility(&mut self, visibility: bool);

    /// Returns local visibility of a node.
    fn get_visibility(&self) -> bool;

    /// Returns combined visibility of an node. This is the final visibility of a node.
    /// Global visibility calculated using visibility of all parent nodes until root one,
    /// so if some parent node upper on tree is invisible then all its children will be
    /// invisible. It defines if object will be rendered. It is *not* the same as real
    /// visibility point of view of some camera. To check if object is visible from some
    /// camera, use appropriate method (TODO: which one?)
    fn get_global_visibility(&self) -> bool;

    fn is_resource_instance(&self) -> bool;

    /// Handle to node in scene of model resource from which this node
    /// was instantiated from.
    fn get_original_handle(&self) -> Handle<Node>;

    /// Returns position of the node in absolute coordinates.
    fn get_global_position(&self) -> Vec3 {
        self.get_global_transform().position()
    }

    /// Returns "look" vector of global transform basis, in most cases return vector
    /// will be non-normalized.
    fn get_look_vector(&self) -> Vec3 {
        self.get_global_transform().look()
    }

    /// Returns "side" vector of global transform basis, in most cases return vector
    /// will be non-normalized.
    fn get_side_vector(&self) -> Vec3 {
        self.get_global_transform().side()
    }

    /// Returns "up" vector of global transform basis, in most cases return vector
    /// will be non-normalized.
    fn get_up_vector(&self) -> Vec3 {
        self.get_global_transform().up()
    }
}

/// Provides default implementation for all methods of NodeTrait.
///
/// # Notes
///
/// Target type must contain `common` field of [`CommonNodeData`] type!
macro_rules! impl_node_trait {
    ($ty_name:ident) => {
        impl $crate::scene::node::NodeTrait for $ty_name {
            fn set_name(&mut self, name: &str) {
                self.common.name = name.to_owned();
            }

            fn get_name(&self) -> &str {
                self.common.name.as_str()
            }

            fn get_local_transform(&self) -> &$crate::scene::transform::Transform {
                &self.common.local_transform
            }

            fn get_local_transform_mut(&mut self) -> &mut $crate::scene::transform::Transform {
                &mut self.common.local_transform
            }

            fn set_lifetime(&mut self, time_seconds: f32) {
                self.common.lifetime = Some(time_seconds);
            }

            fn get_lifetime(&self) -> Option<f32> {
                self.common.lifetime.clone()
            }

            fn get_parent(&self) -> rg3d_core::pool::Handle<crate::scene::node::Node> {
                self.common.parent
            }

            fn get_children(&self) -> &[rg3d_core::pool::Handle<$crate::scene::node::Node>] {
                self.common.children.as_slice()
            }

            fn get_global_transform(&self) -> rg3d_core::math::mat4::Mat4 {
                self.common.global_transform
            }

            fn get_inv_bind_pose_transform(&self) -> rg3d_core::math::mat4::Mat4 {
                self.common.inv_bind_pose_transform
            }

            fn is_resource_instance(&self) -> bool {
                self.common.is_resource_instance
            }

            fn get_resource(&self) -> Option<std::sync::Arc<std::sync::Mutex<$crate::resource::model::Model>>> {
                self.common.resource.clone()
            }

            fn set_visibility(&mut self, visibility: bool) {
                self.common.visibility = visibility;
            }

            fn get_visibility(&self) -> bool {
                self.common.visibility
            }

            fn get_global_visibility(&self) -> bool {
                self.common.global_visibility
            }

            fn get_original_handle(&self) -> rg3d_core::pool::Handle<$crate::scene::node::Node> {
                self.common.original
            }
        }
    }
}

/// Provides default implementation of [`NodeTraitPrivate`].
macro_rules! impl_node_trait_private {
    ($ty_name:ident) => {
        impl $crate::scene::node::NodeTraitPrivate for $ty_name {
            fn get_data(&self) -> &CommonNodeData {
                &self.common
            }

            fn get_data_mut(&mut self) -> &mut CommonNodeData {
                &mut self.common
            }
        }
    }
}

/// Private trait that gives direct access to common node data,
/// it helps preserve encapsulation and does not exposes private
/// data to outer world.
pub(in crate) trait NodeTraitPrivate {
    fn get_data(&self) -> &CommonNodeData;

    fn get_data_mut(&mut self) -> &mut CommonNodeData;
}

/// Struct with common node fields to be able to create composite nodes
/// faster. It will just require to define common field and expose its
/// data via NodeTrait.
pub(in crate) struct CommonNodeData {
    pub name: String,
    pub local_transform: Transform,
    pub visibility: bool,
    pub global_visibility: bool,
    pub parent: Handle<Node>,
    pub children: Vec<Handle<Node>>,
    pub global_transform: Mat4,
    /// Bone-specific matrix. Non-serializable.
    pub inv_bind_pose_transform: Mat4,
    /// A resource from which this node was instantiated from, can work in pair
    /// with `original` handle to get corresponding node from resource.
    pub resource: Option<Arc<Mutex<Model>>>,
    /// Handle to node in scene of model resource from which this node
    /// was instantiated from.
    pub original: Handle<Node>,
    /// When `true` it means that this node is instance of `resource`.
    /// More precisely - this node is root of whole descendant nodes
    /// hierarchy which was instantiated from resource.
    pub is_resource_instance: bool,
    /// Maximum amount of Some(time) that node will "live" or None
    /// if node has undefined lifetime.
    pub lifetime: Option<f32>,
}

impl From<CommonNodeBuilderData> for CommonNodeData {
    fn from(data: CommonNodeBuilderData) -> Self {
        Self {
            name: data.name.unwrap_or_default(),
            children: data.children.unwrap_or_default(),
            local_transform: data.local_transform.unwrap_or_else(Transform::identity),
            lifetime: data.lifetime,
            visibility: data.visibility.unwrap_or(true),
            global_visibility: true,
            parent: Handle::NONE,
            global_transform: Mat4::IDENTITY,
            inv_bind_pose_transform: Mat4::IDENTITY,
            resource: None,
            original: Handle::NONE,
            is_resource_instance: false,
        }
    }
}

/// Shallow copy of node data.
impl Clone for CommonNodeData {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            local_transform: self.local_transform.clone(),
            global_transform: self.global_transform,
            visibility: self.visibility,
            global_visibility: self.global_visibility,
            inv_bind_pose_transform: self.inv_bind_pose_transform,
            resource: self.resource.clone(),
            is_resource_instance: self.is_resource_instance,
            lifetime: self.lifetime,
            // Rest of data is *not* copied!
            .. Default::default()
        }
    }
}

impl Default for CommonNodeData {
    fn default() -> Self {
        Self::from(CommonNodeBuilderData::default())
    }
}

impl Visit for CommonNodeData {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.name.visit("Name", visitor)?;
        self.local_transform.visit("Transform", visitor)?;
        self.visibility.visit("Visibility", visitor)?;
        self.parent.visit("Parent", visitor)?;
        self.children.visit("Children", visitor)?;
        self.resource.visit("Resource", visitor)?;
        self.is_resource_instance.visit("IsResourceInstance", visitor)?;
        self.lifetime.visit("Lifetime", visitor)?;

        visitor.leave_region()
    }
}

impl Visit for Node {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut kind_id = self.id();
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            *self = Node::from_id(kind_id)?;
        }

        // Call appropriate visit method based on actual variant.
        match self {
            Node::Pivot(pivot) => pivot.visit(name, visitor),
            Node::Light(light) => light.visit(name, visitor),
            Node::Camera(camera) => camera.visit(name, visitor),
            Node::Mesh(mesh) => mesh.visit(name, visitor),
            Node::Sprite(sprite) => sprite.visit(name, visitor),
            Node::ParticleSystem(particle_system) => particle_system.visit(name, visitor)
        }
    }
}

#[derive(Clone)]
pub enum Node {
    Pivot(Pivot),
    Light(Light),
    Camera(Camera),
    Mesh(Mesh),
    Sprite(Sprite),
    ParticleSystem(ParticleSystem),
}

impl Default for Node {
    fn default() -> Self {
        Node::Pivot(Default::default())
    }
}

/// Defines as_(variant), as_mut_(variant) and is_(variant) methods.
macro_rules! define_is_as {
    ($is:ident, $as_ref:ident, $as_mut:ident, $kind:ident, $result:ty) => {
        pub fn $is(&self) -> bool {
            match self {
                Node::$kind(_) => true,
                _ => false
            }
        }

        pub fn $as_ref(&self) -> &$result {
            match self {
                Node::$kind(ref val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind))
            }
        }

        pub fn $as_mut(&mut self) -> &mut $result {
            match self {
                Node::$kind(ref mut val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind))
            }
        }
    }
}

impl Node {
    /// Creates new Node based on variant id.
    pub fn from_id(id: u8) -> Result<Self, String> {
        match id {
            0 => Ok(Node::Pivot(Default::default())),
            1 => Ok(Node::Light(Default::default())),
            2 => Ok(Node::Camera(Default::default())),
            3 => Ok(Node::Mesh(Default::default())),
            4 => Ok(Node::Sprite(Default::default())),
            5 => Ok(Node::ParticleSystem(Default::default())),
            _ => Err(format!("Invalid node kind {}", id))
        }
    }

    /// Returns actual variant id.
    pub fn id(&self) -> u8 {
        match self {
            Node::Pivot(_) => 0,
            Node::Light(_) => 1,
            Node::Camera(_) => 2,
            Node::Mesh(_) => 3,
            Node::Sprite(_) => 4,
            Node::ParticleSystem(_) => 5,
        }
    }

    define_is_as!(is_mesh, as_mesh, as_mesh_mut, Mesh, Mesh);
    define_is_as!(is_camera, as_camera, as_camera_mut, Camera, Camera);
    define_is_as!(is_light, as_light, as_light_mut, Light, Light);
    define_is_as!(is_particle_system, as_particle_system, as_particle_system_mut, ParticleSystem, ParticleSystem);
    define_is_as!(is_sprite, as_sprite, as_sprite_mut, Sprite, Sprite);
}

/// Helper macros to reduce code bloat - its purpose it to dispatch
/// specified call by actual enum variant.
macro_rules! dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            Node::Pivot(v) => v.$func($($args),*),
            Node::Mesh(v) => v.$func($($args),*),
            Node::Camera(v) => v.$func($($args),*),
            Node::Light(v) => v.$func($($args),*),
            Node::ParticleSystem(v) => v.$func($($args),*),
            Node::Sprite(v) => v.$func($($args),*),
        }
    };
}

/// Implement dispatcher for Node variants. This allows Node behave like base
/// class of any other specialized nodes.
impl NodeTrait for Node {
    fn set_name(&mut self, name: &str) {
        dispatch!(self, set_name, name)
    }

    fn get_name(&self) -> &str {
        dispatch!(self, get_name,)
    }

    fn get_local_transform(&self) -> &Transform {
        dispatch!(self, get_local_transform,)
    }

    fn get_local_transform_mut(&mut self) -> &mut Transform {
        dispatch!(self, get_local_transform_mut,)
    }

    fn set_lifetime(&mut self, time_seconds: f32) {
        dispatch!(self, set_lifetime, time_seconds)
    }

    fn get_lifetime(&self) -> Option<f32> {
        dispatch!(self, get_lifetime,)
    }

    fn get_parent(&self) -> Handle<Node> {
        dispatch!(self, get_parent,)
    }

    fn get_children(&self) -> &[Handle<Node>] {
        dispatch!(self, get_children,)
    }

    fn get_global_transform(&self) -> Mat4 {
        dispatch!(self, get_global_transform,)
    }

    fn get_inv_bind_pose_transform(&self) -> Mat4 {
        dispatch!(self, get_inv_bind_pose_transform,)
    }

    fn get_resource(&self) -> Option<Arc<Mutex<Model>>> {
        dispatch!(self, get_resource,)
    }

    fn set_visibility(&mut self, visibility: bool) {
        dispatch!(self, set_visibility, visibility)
    }

    fn get_visibility(&self) -> bool {
        dispatch!(self, get_visibility,)
    }

    fn get_global_visibility(&self) -> bool {
        dispatch!(self, get_global_visibility,)
    }

    fn is_resource_instance(&self) -> bool {
        dispatch!(self, is_resource_instance,)
    }

    fn get_original_handle(&self) -> Handle<Node> {
        dispatch!(self, get_original_handle,)
    }
}

impl NodeTraitPrivate for Node {
    fn get_data(&self) -> &CommonNodeData {
        dispatch!(self, get_data,)
    }

    fn get_data_mut(&mut self) -> &mut CommonNodeData {
        dispatch!(self, get_data_mut,)
    }
}

pub(in crate) struct CommonNodeBuilderData {
    pub name: Option<String>,
    pub visibility: Option<bool>,
    pub local_transform: Option<Transform>,
    pub children: Option<Vec<Handle<Node>>>,
    pub lifetime: Option<f32>,
}

impl Default for CommonNodeBuilderData {
    fn default() -> Self {
        Self {
            name: None,
            visibility: None,
            local_transform: None,
            children: None,
            lifetime: None,
        }
    }
}

macro_rules! impl_common_node_builder_methods {
    () => {
        pub fn with_name(mut self, name: &str) -> Self {
            self.common.name = Some(name.to_owned());
            self
        }

        pub fn with_visibility(mut self, visibility: bool) -> Self {
            self.common.visibility = Some(visibility);
            self
        }

        pub fn with_local_transform(mut self, transform: $crate::scene::transform::Transform) -> Self {
            self.common.local_transform = Some(transform);
            self
        }

        pub fn with_children(mut self, children: Vec<rg3d_core::pool::Handle<$crate::scene::node::Node>>) -> Self {
            self.common.children = Some(children);
            self
        }

        pub fn with_lifetime(mut self, time_seconds: f32) -> Self {
            self.common.lifetime = Some(time_seconds);
            self
        }
    }
}

