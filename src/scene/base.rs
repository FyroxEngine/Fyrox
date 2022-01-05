//! Contains all structures and methods to create and manage base scene graph nodes.
//!
//! For more info see [`Base`]

use self::legacy::PhysicsBinding;
use crate::{
    core::{
        algebra::{Matrix4, Vector3},
        inspect::{Inspect, PropertyInfo},
        math::{aabb::AxisAlignedBoundingBox, Matrix4Ext},
        pool::{ErasedHandle, Handle},
        visitor::{Visit, VisitError, VisitResult, Visitor},
    },
    resource::model::Model,
    scene::{graph::Graph, node::Node, transform::Transform},
};
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

pub(crate) mod legacy {
    use crate::core::{
        inspect::{Inspect, PropertyInfo},
        visitor::{Visit, VisitResult, Visitor},
    };

    /// Defines a kind of binding between rigid body and a scene node. Check variants
    /// for more info.
    #[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Inspect)]
    #[repr(u32)]
    pub enum PhysicsBinding {
        /// Forces engine to sync transform of a node with its associated rigid body.
        /// This is default binding.
        NodeWithBody = 0,

        /// Forces engine to sync transform of a rigid body with its associated node. This could be useful for
        /// specific situations like add "hit boxes" to a character.
        ///
        /// # Use cases
        ///
        /// This option has limited usage, but the most common is to create hit boxes. To do that create kinematic
        /// rigid bodies with appropriate colliders and set [`PhysicsBinding::BodyWithNode`] binding to make them
        /// move together with parent nodes.
        BodyWithNode = 1,
    }

    impl Default for PhysicsBinding {
        fn default() -> Self {
            Self::NodeWithBody
        }
    }

    impl PhysicsBinding {
        fn from_id(id: u32) -> Result<Self, String> {
            match id {
                0 => Ok(Self::NodeWithBody),
                1 => Ok(Self::BodyWithNode),
                _ => Err(format!("Invalid physics binding id {}!", id)),
            }
        }
    }

    impl Visit for PhysicsBinding {
        fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
            let mut id = *self as u32;
            id.visit(name, visitor)?;
            if visitor.is_reading() {
                *self = Self::from_id(id)?;
            }
            Ok(())
        }
    }
}

/// A handle to scene node that will be controlled by LOD system.
#[derive(Inspect, Default, Debug, Clone, Copy, PartialEq, Hash)]
pub struct LodControlledObject(pub Handle<Node>);

impl Deref for LodControlledObject {
    type Target = Handle<Node>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LodControlledObject {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Visit for LodControlledObject {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

/// Level of detail is a collection of objects for given normalized distance range.
/// Objects will be rendered **only** if they're in specified range.
/// Normalized distance is a distance in (0; 1) range where 0 - closest to camera,
/// 1 - farthest. Real distance can be obtained by multiplying normalized distance
/// with z_far of current projection matrix.
#[derive(Debug, Default, Clone, Visit, Inspect)]
pub struct LevelOfDetail {
    begin: f32,
    end: f32,
    /// List of objects, where each object represents level of detail of parent's
    /// LOD group.
    pub objects: Vec<LodControlledObject>,
}

impl LevelOfDetail {
    /// Creates new level of detail.
    pub fn new(begin: f32, end: f32, objects: Vec<LodControlledObject>) -> Self {
        for object in objects.iter() {
            // Invalid handles are not allowed.
            assert!(object.is_some());
        }
        let begin = begin.min(end);
        let end = end.max(begin);
        Self {
            begin: begin.min(1.0).max(0.0),
            end: end.min(1.0).max(0.0),
            objects,
        }
    }

    /// Sets new starting point in distance range. Input value will be clamped in
    /// (0; 1) range.
    pub fn set_begin(&mut self, percent: f32) {
        self.begin = percent.min(1.0).max(0.0);
        if self.begin > self.end {
            std::mem::swap(&mut self.begin, &mut self.end);
        }
    }

    /// Returns starting point of the range.
    pub fn begin(&self) -> f32 {
        self.begin
    }

    /// Sets new end point in distance range. Input value will be clamped in
    /// (0; 1) range.
    pub fn set_end(&mut self, percent: f32) {
        self.end = percent.min(1.0).max(0.0);
        if self.end < self.begin {
            std::mem::swap(&mut self.begin, &mut self.end);
        }
    }

    /// Returns end point of the range.
    pub fn end(&self) -> f32 {
        self.end
    }
}

/// LOD (Level-Of-Detail) group is a set of cascades (levels), where each cascade takes specific
/// distance range. Each cascade contains list of objects that should or shouldn't be rendered
/// if distance satisfy cascade range. LOD may significantly improve performance if your scene
/// contains lots of high poly objects and objects may be far away from camera. Distant objects
/// in this case will be rendered with lower details freeing precious GPU resources for other
/// useful tasks.
///
/// Lod group must contain non-overlapping cascades, each cascade with its own set of objects
/// that belongs to level of detail. Engine does not care if you create overlapping cascades,
/// it is your responsibility to create non-overlapping cascades.
#[derive(Debug, Default, Clone, Visit, Inspect)]
pub struct LodGroup {
    /// Set of cascades.
    pub levels: Vec<LevelOfDetail>,
}

/// Mobility defines a group for scene node which has direct impact on performance
/// and capabilities of nodes.
#[derive(
    Copy,
    Clone,
    PartialOrd,
    PartialEq,
    Ord,
    Eq,
    Debug,
    Inspect,
    AsRefStr,
    EnumString,
    EnumVariantNames,
)]
#[repr(u32)]
pub enum Mobility {
    /// Transform cannot be changed.
    ///
    /// ## Scene and performance.
    ///
    /// Nodes with Static mobility should be used all the time you need unchangeable
    /// node. Such nodes will have maximum optimization during the rendering.
    ///
    /// ### Meshes
    ///
    /// Static meshes will be baked into larger blocks to reduce draw call count per frame.
    /// Also static meshes will participate in lightmap generation.
    ///
    /// ### Lights
    ///
    /// Static lights will be baked in lightmap. They lit only static geometry!
    /// Specular lighting is not supported.
    Static = 0,

    /// Transform cannot be changed, but other node-dependent properties are changeable.
    ///
    /// ## Scene and performance.
    ///
    /// ### Meshes
    ///
    /// Same as Static.
    ///
    /// ### Lights
    ///
    /// Stationary lights have complex route for shadows:
    ///   - Shadows from Static/Stationary meshes will be baked into lightmap.
    ///   - Shadows from Dynamic lights will be re-rendered each frame into shadow map.
    /// Stationary lights support specular lighting.
    Stationary = 1,

    /// Transform can be freely changed.
    ///
    /// ## Scene and performance.
    ///
    /// Dynamic mobility should be used only for the objects that are designed to be
    /// moving in the scene, for example - objects with physics, or dynamic lights, etc.
    Dynamic = 2,
}

impl Visit for Mobility {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut id = *self as u32;
        id.visit(name, visitor)?;
        if visitor.is_reading() {
            *self = match id {
                0 => Self::Static,
                1 => Self::Stationary,
                2 => Self::Dynamic,
                _ => return Err(VisitError::User(format!("Invalid mobility id {}!", id))),
            };
        }
        Ok(())
    }
}

/// A property value.
#[derive(Debug, Visit, Inspect, Clone, AsRefStr, EnumString, EnumVariantNames)]
pub enum PropertyValue {
    /// A node handle.
    ///
    /// # Important notes
    ///
    /// The value of the property will be remapped when owning node is cloned, this means that the
    /// handle will always be correct.
    NodeHandle(Handle<Node>),
    /// An arbitrary, type-erased handle.
    ///
    /// # Important notes
    ///
    /// The value of the property will **not** be remapped when owning node is cloned, this means
    /// that the handle correctness is not guaranteed on copy.
    Handle(ErasedHandle),
    /// A string value.
    String(String),
    /// A 64-bit signed integer value.
    I64(i64),
    /// A 64-bit unsigned integer value.
    U64(u64),
    /// A 32-bit signed integer value.
    I32(i32),
    /// A 32-bit unsigned integer value.
    U32(u32),
    /// A 16-bit signed integer value.
    I16(i16),
    /// A 16-bit unsigned integer value.
    U16(u16),
    /// A 8-bit signed integer value.
    I8(i8),
    /// A 8-bit unsigned integer value.
    U8(u8),
    /// A 32-bit floating point value.
    F32(f32),
    /// A 64-bit floating point value.
    F64(f64),
}

impl Default for PropertyValue {
    fn default() -> Self {
        Self::I8(0)
    }
}

/// A custom property.
#[derive(Debug, Visit, Inspect, Default, Clone)]
pub struct Property {
    /// Name of the property.
    pub name: String,
    /// A value of the property.
    pub value: PropertyValue,
}

/// Base scene graph node is a simplest possible node, it is used to build more complex ones using composition.
/// It contains all fundamental properties for each scene graph nodes, like local and global transforms, name,
/// lifetime, etc. Base node is a building block for all complex node hierarchies - it contains list of children
/// and handle to parent node.
///
/// # Example
///
/// ```
/// use rg3d::scene::base::BaseBuilder;
/// use rg3d::scene::graph::Graph;
/// use rg3d::scene::node::Node;
/// use rg3d::core::pool::Handle;
///
/// fn create_base_node(graph: &mut Graph) -> Handle<Node> {
///     BaseBuilder::new()
///         .with_name("BaseNode")
///         .build(graph)
/// }
/// ```
#[derive(Debug, Inspect)]
pub struct Base {
    name: String,
    pub(crate) local_transform: Transform,
    visibility: bool,
    #[inspect(skip)]
    pub(in crate) global_visibility: Cell<bool>,
    #[inspect(skip)]
    pub(in crate) parent: Handle<Node>,
    #[inspect(skip)]
    pub(in crate) children: Vec<Handle<Node>>,
    #[inspect(skip)]
    pub(in crate) global_transform: Cell<Matrix4<f32>>,
    // Bone-specific matrix. Non-serializable.
    #[inspect(skip)]
    pub(in crate) inv_bind_pose_transform: Matrix4<f32>,
    // A resource from which this node was instantiated from, can work in pair
    // with `original` handle to get corresponding node from resource.
    #[inspect(read_only)]
    pub(in crate) resource: Option<Model>,
    // Handle to node in scene of model resource from which this node
    // was instantiated from.
    #[inspect(skip)]
    pub(in crate) original_handle_in_resource: Handle<Node>,
    // When `true` it means that this node is instance of `resource`.
    // More precisely - this node is root of whole descendant nodes
    // hierarchy which was instantiated from resource.
    #[inspect(read_only)]
    pub(in crate) is_resource_instance_root: bool,
    // Maximum amount of Some(time) that node will "live" or None
    // if node has undefined lifetime.
    pub(in crate) lifetime: Option<f32>,
    #[inspect(min_value = 0.0, max_value = 1.0, step = 0.1)]
    depth_offset: f32,
    lod_group: Option<LodGroup>,
    mobility: Mobility,
    tag: String,
    /// A set of custom properties that can hold almost any data. It can be used to set additional
    /// properties to scene nodes.
    pub properties: Vec<Property>,
    #[inspect(skip)]
    pub(in crate) transform_modified: Cell<bool>,

    // Legacy.
    #[inspect(skip)]
    pub(in crate) physics_binding: PhysicsBinding,
    frustum_culling: bool,
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

    /// Returns owned name of node.
    pub fn name_owned(&self) -> String {
        self.name.clone()
    }

    /// Returns shared reference to local transform of a node, can be used to fetch
    /// some local spatial properties, such as position, rotation, scale, etc.
    pub fn local_transform(&self) -> &Transform {
        &self.local_transform
    }

    /// Returns mutable reference to local transform of a node, can be used to set
    /// some local spatial properties, such as position, rotation, scale, etc.
    pub fn local_transform_mut(&mut self) -> &mut Transform {
        self.transform_modified.set(true);
        &mut self.local_transform
    }

    /// Sets new local transform of a node.
    pub fn set_local_transform(&mut self, transform: Transform) -> &mut Self {
        self.local_transform = transform;
        self
    }

    /// Tries to find properties by the name. The method returns an iterator because it possible
    /// to have multiple properties with the same name.
    pub fn find_properties_ref<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Property> {
        self.properties.iter().filter(move |p| p.name == name)
    }

    /// Tries to find a first property with the given name.
    pub fn find_first_property_ref(&self, name: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.name == name)
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
    pub fn set_lifetime(&mut self, time_seconds: Option<f32>) -> &mut Self {
        self.lifetime = time_seconds;
        self
    }

    /// Returns current lifetime of a node. Will be None if node has undefined lifetime.
    /// For more info about lifetimes see [`set_lifetime`](Self::set_lifetime).
    pub fn lifetime(&self) -> Option<f32> {
        self.lifetime
    }

    /// Returns handle of parent node.
    pub fn parent(&self) -> Handle<Node> {
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
    pub fn global_transform(&self) -> Matrix4<f32> {
        self.global_transform.get()
    }

    /// Returns inverse of bind pose matrix. Bind pose matrix - is special matrix
    /// for bone nodes, it stores initial transform of bone node at the moment
    /// of "binding" vertices to bones.
    pub fn inv_bind_pose_transform(&self) -> Matrix4<f32> {
        self.inv_bind_pose_transform
    }

    /// Returns true if this node is model resource instance root node.
    pub fn is_resource_instance_root(&self) -> bool {
        self.is_resource_instance_root
    }

    /// Returns resource from which this node was instantiated from.
    pub fn resource(&self) -> Option<Model> {
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

    /// Returns current **local-space** bounding box. Keep in mind that this value is just
    /// a placeholder, because there is not information to calculate actual bounding box.
    #[inline]
    pub fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::unit()
    }

    /// Returns current **world-space** bounding box.
    pub fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    /// Set new mobility for the node.
    ///
    /// TODO. Mobility still has no effect, it was designed to be used in combined
    /// rendering (dynamic + static lights (lightmaps))
    pub fn set_mobility(&mut self, mobility: Mobility) -> &mut Self {
        self.mobility = mobility;
        self
    }

    /// Return current mobility of the node.
    pub fn mobility(&self) -> Mobility {
        self.mobility
    }

    /// Returns combined visibility of an node. This is the final visibility of a node. Global visibility calculated
    /// using visibility of all parent nodes until root one, so if some parent node upper on tree is invisible then
    /// all its children will be invisible. It defines if object will be rendered. It is *not* the same as real
    /// visibility from point of view of a camera. To check if object is visible from some camera, use
    /// [VisibilityCache](super::visibility::VisibilityCache). However this still can't tell you if object is behind obstacle or not.
    pub fn global_visibility(&self) -> bool {
        self.global_visibility.get()
    }

    /// Handle to node in scene of model resource from which this node was instantiated from.
    ///
    /// # Notes
    ///
    /// This handle is extensively used to fetch information about the state of node in the resource
    /// to sync properties of instance with its original in the resource.
    pub fn original_handle_in_resource(&self) -> Handle<Node> {
        self.original_handle_in_resource
    }

    /// Returns position of the node in absolute coordinates.
    pub fn global_position(&self) -> Vector3<f32> {
        self.global_transform.get().position()
    }

    /// Returns "look" vector of global transform basis, in most cases return vector will be non-normalized.
    pub fn look_vector(&self) -> Vector3<f32> {
        self.global_transform.get().look()
    }

    /// Returns "side" vector of global transform basis, in most cases return vector will be non-normalized.
    pub fn side_vector(&self) -> Vector3<f32> {
        self.global_transform.get().side()
    }

    /// Returns "up" vector of global transform basis, in most cases return vector will be non-normalized.
    pub fn up_vector(&self) -> Vector3<f32> {
        self.global_transform.get().up()
    }

    /// Sets depth range offset factor. It allows you to move depth range by given value. This can be used
    /// to draw weapons on top of other stuff in scene.
    ///
    /// # Details
    ///
    /// This value is used to modify projection matrix before render node. Element m\[4\]\[3\] of projection
    /// matrix usually set to -1 to which makes w coordinate of in homogeneous space to be -z_fragment for
    /// further perspective divide. We can abuse this to shift z of fragment by some value.
    pub fn set_depth_offset_factor(&mut self, factor: f32) {
        self.depth_offset = factor.abs().min(1.0).max(0.0);
    }

    /// Returns depth offset factor.
    pub fn depth_offset_factor(&self) -> f32 {
        self.depth_offset
    }

    /// Sets new lod group.
    pub fn set_lod_group(&mut self, lod_group: Option<LodGroup>) -> Option<LodGroup> {
        std::mem::replace(&mut self.lod_group, lod_group)
    }

    /// Extracts lod group, leaving None in the node.
    pub fn take_lod_group(&mut self) -> Option<LodGroup> {
        std::mem::take(&mut self.lod_group)
    }

    /// Returns shared reference to current lod group.
    pub fn lod_group(&self) -> Option<&LodGroup> {
        self.lod_group.as_ref()
    }

    /// Returns mutable reference to current lod group.
    pub fn lod_group_mut(&mut self) -> Option<&mut LodGroup> {
        self.lod_group.as_mut()
    }

    /// Returns node tag.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Returns a copy of node tag.
    pub fn tag_owned(&self) -> String {
        self.tag.clone()
    }

    /// Sets new tag.
    pub fn set_tag(&mut self, tag: String) {
        self.tag = tag;
    }

    /// Shallow copy of node data. You should never use this directly, shallow copy
    /// will produce invalid node in most cases!
    pub fn raw_copy(&self) -> Self {
        Self {
            name: self.name.clone(),
            local_transform: self.local_transform.clone(),
            global_transform: self.global_transform.clone(),
            visibility: self.visibility,
            global_visibility: self.global_visibility.clone(),
            inv_bind_pose_transform: self.inv_bind_pose_transform,
            resource: self.resource.clone(),
            is_resource_instance_root: self.is_resource_instance_root,
            lifetime: self.lifetime,
            mobility: self.mobility,
            tag: self.tag.clone(),
            lod_group: self.lod_group.clone(),
            properties: self.properties.clone(),

            // Rest of data is *not* copied!
            original_handle_in_resource: Default::default(),
            parent: Default::default(),
            children: Default::default(),
            depth_offset: Default::default(),
            transform_modified: Cell::new(false),
            physics_binding: Default::default(),
            frustum_culling: self.frustum_culling,
        }
    }

    /// Return the frustum_culling flag
    pub fn frustum_culling(&self) -> bool {
        self.frustum_culling
    }

    /// Sets whether to use frustum culling or not
    pub fn set_frustum_culling(&mut self, frustum_culling: bool) {
        self.frustum_culling = frustum_culling;
    }
}

impl Default for Base {
    fn default() -> Self {
        BaseBuilder::new().build_base()
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
        self.is_resource_instance_root
            .visit("IsResourceInstance", visitor)?;
        self.lifetime.visit("Lifetime", visitor)?;
        self.depth_offset.visit("DepthOffset", visitor)?;
        self.lod_group.visit("LodGroup", visitor)?;
        self.mobility.visit("Mobility", visitor)?;
        self.original_handle_in_resource
            .visit("Original", visitor)?;
        self.tag.visit("Tag", visitor)?;
        let _ = self.properties.visit("Properties", visitor);
        let _ = self.physics_binding.visit("PhysicsBinding", visitor);
        let _ = self.frustum_culling.visit("FrustumCulling", visitor);

        visitor.leave_region()
    }
}

/// Base node builder allows you to create nodes in declarative manner.
pub struct BaseBuilder {
    name: String,
    visibility: bool,
    local_transform: Transform,
    children: Vec<Handle<Node>>,
    lifetime: Option<f32>,
    depth_offset: f32,
    lod_group: Option<LodGroup>,
    mobility: Mobility,
    inv_bind_pose_transform: Matrix4<f32>,
    tag: String,
    frustum_culling: bool,
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
            name: Default::default(),
            visibility: true,
            local_transform: Default::default(),
            children: Default::default(),
            lifetime: None,
            depth_offset: 0.0,
            lod_group: None,
            mobility: Mobility::Dynamic,
            inv_bind_pose_transform: Matrix4::identity(),
            tag: Default::default(),
            frustum_culling: true,
        }
    }

    /// Sets desired mobility.
    pub fn with_mobility(mut self, mobility: Mobility) -> Self {
        self.mobility = mobility;
        self
    }

    /// Sets desired name.
    pub fn with_name<P: AsRef<str>>(mut self, name: P) -> Self {
        self.name = name.as_ref().to_owned();
        self
    }

    /// Sets desired visibility.
    pub fn with_visibility(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    /// Sets desired local transform.
    pub fn with_local_transform(mut self, transform: Transform) -> Self {
        self.local_transform = transform;
        self
    }

    /// Sets desired inverse bind pose transform.
    pub fn with_inv_bind_pose_transform(mut self, inv_bind_pose: Matrix4<f32>) -> Self {
        self.inv_bind_pose_transform = inv_bind_pose;
        self
    }

    /// Sets desired list of children nodes.
    pub fn with_children<'a, I: IntoIterator<Item = &'a Handle<Node>>>(
        mut self,
        children: I,
    ) -> Self {
        for &child in children.into_iter() {
            if child.is_some() {
                self.children.push(child)
            }
        }
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

    /// Sets desired lod group.
    pub fn with_lod_group(mut self, lod_group: LodGroup) -> Self {
        self.lod_group = Some(lod_group);
        self
    }

    /// Sets desired tag.
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tag = tag;
        self
    }

    /// Sets desired frustum_culling flag.
    pub fn with_frustum_culling(mut self, frustum_culling: bool) -> Self {
        self.frustum_culling = frustum_culling;
        self
    }

    pub(in crate) fn build_base(self) -> Base {
        Base {
            name: self.name,
            children: self.children,
            local_transform: self.local_transform,
            lifetime: self.lifetime,
            visibility: self.visibility,
            global_visibility: Cell::new(true),
            parent: Handle::NONE,
            global_transform: Cell::new(Matrix4::identity()),
            inv_bind_pose_transform: self.inv_bind_pose_transform,
            resource: None,
            original_handle_in_resource: Handle::NONE,
            is_resource_instance_root: false,
            depth_offset: self.depth_offset,
            lod_group: self.lod_group,
            mobility: self.mobility,
            tag: self.tag,
            properties: Default::default(),
            transform_modified: Cell::new(false),
            physics_binding: Default::default(),
            frustum_culling: self.frustum_culling,
        }
    }

    /// Creates new instance of base node.
    pub fn build_node(self) -> Node {
        Node::Base(self.build_base())
    }

    /// Creates new instance of base node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
