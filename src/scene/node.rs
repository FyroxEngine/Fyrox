use std::sync::{Arc, Mutex};
use crate::{
    scene::{
        camera::Camera,
        mesh::Mesh,
        light::Light,
        particle_system::ParticleSystem,
        transform::Transform,
        sprite::Sprite,
    },
    resource::model::Model,
};
use rg3d_core::{
    math::{vec3::Vec3, mat4::Mat4},
    visitor::{Visit, VisitResult, Visitor},
    pool::Handle,
};
use crate::scene::graph::Graph;

pub enum NodeKind {
    Base,
    Light(Light),
    Camera(Camera),
    Mesh(Mesh),
    Sprite(Sprite),
    ParticleSystem(ParticleSystem),
}

impl Visit for NodeKind {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        match self {
            NodeKind::Base => Ok(()),
            NodeKind::Light(light) => light.visit(name, visitor),
            NodeKind::Camera(camera) => camera.visit(name, visitor),
            NodeKind::Mesh(mesh) => mesh.visit(name, visitor),
            NodeKind::Sprite(sprite) => sprite.visit(name, visitor),
            NodeKind::ParticleSystem(particle_system) => particle_system.visit(name, visitor)
        }
    }
}

impl Clone for NodeKind {
    fn clone(&self) -> Self {
        match &self {
            NodeKind::Base => NodeKind::Base,
            NodeKind::Camera(camera) => NodeKind::Camera(camera.clone()),
            NodeKind::Light(light) => NodeKind::Light(light.clone()),
            NodeKind::Mesh(mesh) => NodeKind::Mesh(mesh.clone()),
            NodeKind::Sprite(sprite) => NodeKind::Sprite(sprite.clone()),
            NodeKind::ParticleSystem(particle_system) => NodeKind::ParticleSystem(particle_system.clone())
        }
    }
}

impl NodeKind {
    /// Creates new NodeKind based on variant id.
    pub fn new(id: u8) -> Result<Self, String> {
        match id {
            0 => Ok(NodeKind::Base),
            1 => Ok(NodeKind::Light(Default::default())),
            2 => Ok(NodeKind::Camera(Default::default())),
            3 => Ok(NodeKind::Mesh(Default::default())),
            4 => Ok(NodeKind::Sprite(Default::default())),
            5 => Ok(NodeKind::ParticleSystem(Default::default())),
            _ => Err(format!("Invalid node kind {}", id))
        }
    }

    /// Returns actual variant id.
    pub fn id(&self) -> u8 {
        match self {
            NodeKind::Base => 0,
            NodeKind::Light(_) => 1,
            NodeKind::Camera(_) => 2,
            NodeKind::Mesh(_) => 3,
            NodeKind::Sprite(_) => 4,
            NodeKind::ParticleSystem(_) => 5,
        }
    }
}

pub struct Node {
    name: String,
    kind: NodeKind,
    pub(in crate::scene) local_transform: Transform,
    pub(in crate::scene) visibility: bool,
    pub(in crate::scene) global_visibility: bool,
    pub(in crate::scene) parent: Handle<Node>,
    pub(in crate::scene) children: Vec<Handle<Node>>,
    pub(in crate::scene) global_transform: Mat4,
    /// Bone-specific matrix. Non-serializable.
    inv_bind_pose_transform: Mat4,
    /// A resource from which this node was instantiated from, can work in pair
    /// with `original` handle to get corresponding node from resource.
    resource: Option<Arc<Mutex<Model>>>,
    /// Handle to node in scene of model resource from which this node
    /// was instantiated from.
    pub(in crate::scene) original: Handle<Node>,
    /// When `true` it means that this node is instance of `resource`.
    /// More precisely - this node is root of whole descendant nodes
    /// hierarchy which was instantiated from resource.
    pub(in crate) is_resource_instance: bool,
    /// Maximum amount of Some(time) that node will "live" or None
    /// if node has undefined lifetime.
    pub(in crate::scene) lifetime: Option<f32>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            kind: NodeKind::Base,
            name: String::new(),
            children: Vec::new(),
            parent: Handle::NONE,
            visibility: true,
            global_visibility: true,
            local_transform: Transform::identity(),
            global_transform: Mat4::identity(),
            inv_bind_pose_transform: Mat4::identity(),
            resource: None,
            original: Handle::NONE,
            is_resource_instance: false,
            lifetime: None,
        }
    }
}

macro_rules! define_is_as {
    ($is_name:ident, $as_ref:ident, $as_mut:ident, $kind:ident, $result:ty) => {
        #[inline]
        pub fn $is_name(&self) -> bool {
            match self.kind {
                NodeKind::$kind(_) => true,
                _ => false
            }
        }

        #[inline]
        pub fn $as_ref(&self) -> &$result {
            match self.kind {
                NodeKind::$kind(ref val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind))
            }
        }

        #[inline]
        pub fn $as_mut(&mut self) -> &mut $result {
            match self.kind {
                NodeKind::$kind(ref mut val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind))
            }
        }
    }
}

impl Node {
    /// Creates new node of specified kind. Node must be put in some graph so
    /// engine will know that there is such node.
    pub fn new(kind: NodeKind) -> Self {
        Node {
            kind,
            name: String::new(),
            children: Vec::new(),
            parent: Handle::NONE,
            visibility: true,
            global_visibility: true,
            local_transform: Transform::identity(),
            global_transform: Mat4::identity(),
            inv_bind_pose_transform: Mat4::identity(),
            resource: None,
            original: Handle::NONE,
            is_resource_instance: false,
            lifetime: None,
        }
    }

    /// Creates copy of node without copying children nodes. Children nodes has to be
    /// copied explicitly, use [`copy_node`] of [`Graph`] to make deep copy.
    pub fn make_copy(&self, original: Handle<Node>) -> Self {
        Self {
            kind: self.kind.clone(),
            name: self.name.clone(),
            local_transform: self.local_transform.clone(),
            global_transform: self.global_transform,
            visibility: self.visibility,
            global_visibility: self.global_visibility,
            inv_bind_pose_transform: self.inv_bind_pose_transform,
            children: Vec::new(),
            parent: Handle::NONE,
            resource: self.get_resource(),
            is_resource_instance: self.is_resource_instance,
            lifetime: self.lifetime,
            original,
        }
    }

    #[inline]
    pub(in crate) fn set_resource(&mut self, resource_handle: Arc<Mutex<Model>>) {
        self.resource = Some(resource_handle);
    }

    /// Returns resource from which this node was instantiated from.
    #[inline]
    pub fn get_resource(&self) -> Option<Arc<Mutex<Model>>> {
        self.resource.clone()
    }

    /// Returns shared reference to local transform of a node, can be used to fetch
    /// some local spatial properties, such as position, rotation, scale, etc.
    #[inline]
    pub fn get_local_transform(&self) -> &Transform {
        &self.local_transform
    }

    /// Returns mutable reference to local transform of a node, can be used to set
    /// some local spatial properties, such as position, rotation, scale, etc.
    #[inline]
    pub fn get_local_transform_mut(&mut self) -> &mut Transform {
        &mut self.local_transform
    }

    /// Returns mutable reference to kind of a node. Useful to do some kind-specific
    /// action on node or to retrieve some information from it. For example you can
    /// have a light node, to set light color you have to call this method and use
    /// pattern matching to check if this node is a light, then you'll have access
    /// to light and free to modify it as you want to.
    #[inline]
    pub fn get_kind_mut(&mut self) -> &mut NodeKind {
        &mut self.kind
    }

    /// Returns shared reference to kind of a node. Useful to fetch some information
    /// from a node.
    #[inline]
    pub fn get_kind(&self) -> &NodeKind {
        &self.kind
    }

    /// Sets local visibility of a node.
    #[inline]
    pub fn set_visibility(&mut self, visibility: bool) {
        self.visibility = visibility;
    }

    /// Returns local visibility of a node.
    #[inline]
    pub fn get_visibility(&self) -> bool {
        self.visibility
    }

    /// Returns combined visibility of an node. This is the final visibility of a node.
    /// Global visibility calculated using visibility of all parent nodes until root one,
    /// so if some parent node upper on tree is invisible then all its children will be
    /// invisible. It defines if object will be rendered. It is *not* the same as real
    /// visibility point of view of some camera. To check if object is visible from some
    /// camera, use appropriate method (TODO: which one?)
    #[inline]
    pub fn get_global_visibility(&self) -> bool {
        self.global_visibility
    }

    /// Sets name of node. Can be useful to mark a node to be able to find it later on.
    #[inline]
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
    }

    /// Returns name of node.
    #[inline]
    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns handle of parent node.
    #[inline]
    pub fn get_parent(&self) -> Handle<Node> {
        self.parent
    }

    /// Returns slice of handles to children nodes. This can be used, for example, to
    /// traverse tree starting from some node.
    #[inline]
    pub fn get_children(&self) -> &[Handle<Node>] {
        &self.children
    }

    /// Returns global transform matrix, such matrix contains combined transformation
    /// of transforms of parent nodes. This is the final matrix that describes real
    /// location of object in the world.
    #[inline]
    pub fn get_global_transform(&self) -> &Mat4 {
        &self.global_transform
    }

    /// Sets inverse of bind pose matrix. For more info see [`get_inv_bind_pose_transform`]
    #[inline]
    pub fn set_inv_bind_pose_transform(&mut self, transform: Mat4) {
        self.inv_bind_pose_transform = transform;
    }

    /// Returns inverse of bind pose matrix. Bind pose matrix - is special matrix
    /// for bone nodes, it stores initial transform of bone node at the moment
    /// of "binding" vertices to bones.
    #[inline]
    pub fn get_inv_bind_pose_transform(&self) -> Mat4 {
        self.inv_bind_pose_transform
    }

    /// Returns position of the node in absolute coordinates.
    #[inline]
    pub fn get_global_position(&self) -> Vec3 {
        self.global_transform.position()
    }

    /// Returns "look" vector of global transform basis, in most cases return vector
    /// will be non-normalized.
    #[inline]
    pub fn get_look_vector(&self) -> Vec3 {
        self.global_transform.look()
    }

    /// Returns "side" vector of global transform basis, in most cases return vector
    /// will be non-normalized.
    #[inline]
    pub fn get_side_vector(&self) -> Vec3 {
        self.global_transform.side()
    }

    /// Returns "up" vector of global transform basis, in most cases return vector
    /// will be non-normalized.
    #[inline]
    pub fn get_up_vector(&self) -> Vec3 {
        self.global_transform.up()
    }

    define_is_as!(is_mesh, as_mesh, as_mesh_mut, Mesh, Mesh);
    define_is_as!(is_camera, as_camera, as_camera_mut, Camera, Camera);
    define_is_as!(is_light, as_light, as_light_mut, Light, Light);
    define_is_as!(is_particle_system, as_particle_system, as_particle_system_mut, ParticleSystem, ParticleSystem);
    define_is_as!(is_sprite, as_sprite, as_sprite_mut, Sprite, Sprite);

    /// Sets lifetime of node in seconds, lifetime is useful for temporary objects.
    /// Example - you firing a gun, it produces two particle systems for each shot:
    /// one for gunpowder fumes and one when bullet hits some surface. These particle
    /// systems won't last very long - usually they will disappear in 1-2 seconds
    /// but nodes will still be in scene consuming precious CPU clocks. This is where
    /// lifetimes become handy - you just set appropriate lifetime for a particle
    /// system node and it will be removed from scene when time will end. This is
    /// efficient algorithm because scene holds every object in pool and allocation
    /// or deallocation of node takes very little amount of time.
    #[inline]
    pub fn set_lifetime(&mut self, time_seconds: f32) {
        self.lifetime = Some(time_seconds);
    }

    /// Returns current lifetime of a node. Will be None if node has undefined lifetime.
    /// For more info about lifetimes see [`set_lifetime`].
    #[inline]
    pub fn get_lifetime(&self) -> Option<f32> {
        self.lifetime.clone()
    }
}

impl Visit for Node {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind_id: u8 = self.kind.id();
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            self.kind = NodeKind::new(kind_id)?;
        }

        self.kind.visit("Kind", visitor)?;
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

pub struct NodeBuilder {
    kind: NodeKind,
    name: Option<String>,
    visibility: Option<bool>,
    local_transform: Option<Transform>,
    children: Option<Vec<Handle<Node>>>,
    lifetime: Option<f32>,
}

impl NodeBuilder {
    pub fn new(kind: NodeKind) -> Self {
        Self {
            kind,
            name: None,
            visibility: None,
            local_transform: None,
            children: None,
            lifetime: None,
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_owned());
        self
    }

    pub fn with_visibility(mut self, visibility: bool) -> Self {
        self.visibility = Some(visibility);
        self
    }

    pub fn with_local_transform(mut self, transform: Transform) -> Self {
        self.local_transform = Some(transform);
        self
    }

    pub fn with_children(mut self, children: Vec<Handle<Node>>) -> Self {
        self.children = Some(children);
        self
    }

    pub fn with_lifetime(mut self, time_seconds: f32) -> Self {
        self.lifetime = Some(time_seconds);
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(Node {
            name: self.name.unwrap_or(String::new()),
            kind: self.kind,
            local_transform: self.local_transform.unwrap_or(Transform::identity()),
            visibility: true,
            global_visibility: true,
            parent: Handle::NONE,
            children: self.children.unwrap_or(Vec::new()),
            global_transform: Mat4::identity(),
            inv_bind_pose_transform: Mat4::identity(),
            resource: None,
            original: Handle::NONE,
            is_resource_instance: false,
            lifetime: self.lifetime,
        })
    }
}