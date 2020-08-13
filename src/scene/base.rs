//! Contains all structures and methods to create and manage base scene graph nodes.
//!
//! Base scene graph node is a simplest possible node, it is used to build more complex
//! ones using composition. It contains all fundamental properties for each scene graph
//! nodes, like local and global transforms, name, lifetime, etc. Base node is a building
//! block for all complex node hierarchies - it contains list of children and handle to
//! parent node.

use crate::{
    core::{
        math::{mat4::Mat4, vec3::Vec3},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    resource::model::Model,
    scene::{node::Node, transform::Transform},
};
use std::sync::{Arc, Mutex};

/// See module docs.
#[derive(Debug)]
pub struct Base {
    name: String,
    local_transform: Transform,
    visibility: bool,
    pub(in crate) global_visibility: bool,
    pub(in crate) parent: Handle<Node>,
    pub(in crate) children: Vec<Handle<Node>>,
    pub(in crate) global_transform: Mat4,
    /// Bone-specific matrix. Non-serializable.
    pub(in crate) inv_bind_pose_transform: Mat4,
    /// A resource from which this node was instantiated from, can work in pair
    /// with `original` handle to get corresponding node from resource.
    pub(in crate) resource: Option<Arc<Mutex<Model>>>,
    /// Handle to node in scene of model resource from which this node
    /// was instantiated from.
    pub(in crate) original: Handle<Node>,
    /// When `true` it means that this node is instance of `resource`.
    /// More precisely - this node is root of whole descendant nodes
    /// hierarchy which was instantiated from resource.
    pub(in crate) is_resource_instance: bool,
    /// Maximum amount of Some(time) that node will "live" or None
    /// if node has undefined lifetime.
    lifetime: Option<f32>,
    depth_offset: f32,
}

impl Base {
    /// Sets name of node. Can be useful to mark a node to be able to find it later on.
    pub fn set_name<N: AsRef<str>>(&mut self, name: N) -> &mut Self {
        self.name = name.as_ref().to_owned();
        self
    }

    /// Returns name of node.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns shared reference to local transform of a node, can be used to fetch
    /// some local spatial properties, such as position, rotation, scale, etc.
    pub fn local_transform(&self) -> &Transform {
        &self.local_transform
    }

    /// Returns mutable reference to local transform of a node, can be used to set
    /// some local spatial properties, such as position, rotation, scale, etc.
    pub fn local_transform_mut(&mut self) -> &mut Transform {
        &mut self.local_transform
    }

    /// Sets new local transform of a node.
    pub fn set_local_transform(&mut self, transform: Transform) -> &mut Self {
        self.local_transform = transform;
        self
    }

    /// Sets lifetime of node in seconds, lifetime is useful for temporary objects.
    /// Example - you firing a gun, it produces two particle systems for each shot:
    /// one for gunpowder fumes and one when bullet hits some surface. These particle
    /// systems won't last very long - usually they will disappear in 1-2 seconds
    /// but nodes will still be in scene consuming precious CPU clocks. This is where
    /// lifetimes become handy - you just set appropriate lifetime for a particle
    /// system node and it will be removed from scene when time will end. This is
    /// efficient algorithm because scene holds every object in pool and allocation
    /// or deallocation of node takes very little amount of time.
    pub fn set_lifetime(&mut self, time_seconds: f32) -> &mut Self {
        self.lifetime = Some(time_seconds);
        self
    }

    /// Returns current lifetime of a node. Will be None if node has undefined lifetime.
    /// For more info about lifetimes see [`set_lifetime`].
    pub fn lifetime(&self) -> Option<f32> {
        self.lifetime
    }

    /// Returns handle of parent node.
    pub fn parent(&self) -> crate::core::pool::Handle<Node> {
        self.parent
    }

    /// Returns slice of handles to children nodes. This can be used, for example, to
    /// traverse tree starting from some node.
    pub fn children(&self) -> &[Handle<Node>] {
        self.children.as_slice()
    }

    /// Returns global transform matrix, such matrix contains combined transformation
    /// of transforms of parent nodes. This is the final matrix that describes real
    /// location of object in the world.
    pub fn global_transform(&self) -> Mat4 {
        self.global_transform
    }

    /// Returns inverse of bind pose matrix. Bind pose matrix - is special matrix
    /// for bone nodes, it stores initial transform of bone node at the moment
    /// of "binding" vertices to bones.
    pub fn inv_bind_pose_transform(&self) -> Mat4 {
        self.inv_bind_pose_transform
    }

    /// Returns true if this node is model resource instance root node.
    pub fn is_resource_instance(&self) -> bool {
        self.is_resource_instance
    }

    /// Returns resource from which this node was instantiated from.
    pub fn resource(&self) -> Option<Arc<Mutex<Model>>> {
        self.resource.clone()
    }

    /// Sets local visibility of a node.
    pub fn set_visibility(&mut self, visibility: bool) -> &mut Self {
        self.visibility = visibility;
        self
    }

    /// Returns local visibility of a node.
    pub fn visibility(&self) -> bool {
        self.visibility
    }

    /// Returns combined visibility of an node. This is the final visibility of a node.
    /// Global visibility calculated using visibility of all parent nodes until root one,
    /// so if some parent node upper on tree is invisible then all its children will be
    /// invisible. It defines if object will be rendered. It is *not* the same as real
    /// visibility point of view of some camera. To check if object is visible from some
    /// camera, use frustum visibility check. However this still can't tell you if object
    /// is behind obstacle or not.
    pub fn global_visibility(&self) -> bool {
        self.global_visibility
    }

    /// Handle to node in scene of model resource from which this node
    /// was instantiated from.
    pub fn original_handle(&self) -> Handle<Node> {
        self.original
    }

    /// Returns position of the node in absolute coordinates.
    pub fn global_position(&self) -> Vec3 {
        self.global_transform.position()
    }

    /// Returns "look" vector of global transform basis, in most cases return vector
    /// will be non-normalized.
    pub fn look_vector(&self) -> Vec3 {
        self.global_transform.look()
    }

    /// Returns "side" vector of global transform basis, in most cases return vector
    /// will be non-normalized.
    pub fn side_vector(&self) -> Vec3 {
        self.global_transform.side()
    }

    /// Returns "up" vector of global transform basis, in most cases return vector
    /// will be non-normalized.
    pub fn up_vector(&self) -> Vec3 {
        self.global_transform.up()
    }

    /// Sets depth range offset factor. It allows you to move depth range by given
    /// value. This can be used to draw weapons on top of other stuff in scene.
    ///
    /// # Details
    ///
    /// This value is used to modify projection matrix before render node.
    /// Element m[4][3] of projection matrix usually set to -1 to which makes w coordinate
    /// of in homogeneous space to be -z_fragment for further perspective divide. We can
    /// abuse this to shift z of fragment by some value.
    pub fn set_depth_offset_factor(&mut self, factor: f32) {
        self.depth_offset = factor.abs().min(1.0).max(0.0);
    }

    /// Returns depth offset factor.
    pub fn depth_offset_factor(&self) -> f32 {
        self.depth_offset
    }
}

impl Clone for Base {
    /// Shallow copy of node data. You should never use this directly, shallow copy
    /// will produce invalid node in most cases!
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
            ..Default::default()
        }
    }
}

impl Default for Base {
    fn default() -> Self {
        BaseBuilder::new().build()
    }
}

impl Visit for Base {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.name.visit("Name", visitor)?;
        self.local_transform.visit("Transform", visitor)?;
        self.visibility.visit("Visibility", visitor)?;
        self.parent.visit("Parent", visitor)?;
        self.children.visit("Children", visitor)?;
        self.resource.visit("Resource", visitor)?;
        self.is_resource_instance
            .visit("IsResourceInstance", visitor)?;
        self.lifetime.visit("Lifetime", visitor)?;
        self.depth_offset.visit("DepthOffset", visitor)?;

        visitor.leave_region()
    }
}

/// Base node builder allows you to create nodes in declarative manner.
pub struct BaseBuilder {
    name: Option<String>,
    visibility: Option<bool>,
    local_transform: Option<Transform>,
    children: Option<Vec<Handle<Node>>>,
    lifetime: Option<f32>,
    depth_offset: f32,
}

impl Default for BaseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseBuilder {
    /// Creates new builder instance.
    pub fn new() -> Self {
        Self {
            name: None,
            visibility: None,
            local_transform: None,
            children: None,
            lifetime: None,
            depth_offset: 0.0,
        }
    }

    /// Sets desired name.
    pub fn with_name<P: AsRef<str>>(mut self, name: P) -> Self {
        self.name = Some(name.as_ref().to_owned());
        self
    }

    /// Sets desired visibility.
    pub fn with_visibility(mut self, visibility: bool) -> Self {
        self.visibility = Some(visibility);
        self
    }

    /// Sets desired local transform.
    pub fn with_local_transform(mut self, transform: Transform) -> Self {
        self.local_transform = Some(transform);
        self
    }

    /// Sets desired list of children nodes.
    pub fn with_children(mut self, children: Vec<Handle<Node>>) -> Self {
        self.children = Some(children);
        self
    }

    /// Sets desired lifetime.
    pub fn with_lifetime(mut self, time_seconds: f32) -> Self {
        self.lifetime = Some(time_seconds);
        self
    }

    /// Sets desired depth offset.
    pub fn with_depth_offset(mut self, offset: f32) -> Self {
        self.depth_offset = offset;
        self
    }

    /// Creates new instance of base scene node. Do not forget to add
    /// node to scene or pass to other nodes as base.
    pub fn build(self) -> Base {
        Base {
            name: self.name.unwrap_or_default(),
            children: self.children.unwrap_or_default(),
            local_transform: self.local_transform.unwrap_or_else(Transform::identity),
            lifetime: self.lifetime,
            visibility: self.visibility.unwrap_or(true),
            global_visibility: true,
            parent: Handle::NONE,
            global_transform: Mat4::IDENTITY,
            inv_bind_pose_transform: Mat4::IDENTITY,
            resource: None,
            original: Handle::NONE,
            is_resource_instance: false,
            depth_offset: self.depth_offset,
        }
    }
}
