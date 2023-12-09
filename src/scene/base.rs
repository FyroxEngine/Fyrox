//! Contains all structures and methods to create and manage base scene graph nodes.
//!
//! For more info see [`Base`]

use crate::{
    core::{
        algebra::{Matrix4, Vector3},
        log::Log,
        math::{aabb::AxisAlignedBoundingBox, Matrix4Ext},
        pool::{ErasedHandle, Handle},
        reflect::prelude::*,
        uuid::Uuid,
        variable::InheritableVariable,
        visitor::{Visit, VisitError, VisitResult, Visitor},
    },
    engine::SerializationContext,
    resource::model::ModelResource,
    scene::{node::Node, transform::Transform},
    script::{Script, ScriptTrait},
};
use fyrox_core::uuid_provider;
use std::{any::Any, cell::Cell, sync::mpsc::Sender};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

/// Level of detail is a collection of objects for given normalized distance range.
/// Objects will be rendered **only** if they're in specified range.
/// Normalized distance is a distance in (0; 1) range where 0 - closest to camera,
/// 1 - farthest. Real distance can be obtained by multiplying normalized distance
/// with z_far of current projection matrix.
#[derive(Debug, Default, Clone, Visit, Reflect, PartialEq)]
pub struct LevelOfDetail {
    #[reflect(
        description = "Beginning of the range in which the level will be visible. \
    It is expressed in normalized coordinates: where 0.0 - closest to camera, 1.0 - \
    farthest from camera."
    )]
    begin: f32,
    #[reflect(description = "End of the range in which the level will be visible. \
    It is expressed in normalized coordinates: where 0.0 - closest to camera, 1.0 - \
    farthest from camera.")]
    end: f32,
    /// List of objects, where each object represents level of detail of parent's
    /// LOD group.
    pub objects: Vec<Handle<Node>>,
}

uuid_provider!(LevelOfDetail = "576b31a2-2b39-4c79-95dd-26aeaf381d8b");

impl LevelOfDetail {
    /// Creates new level of detail.
    pub fn new(begin: f32, end: f32, objects: Vec<Handle<Node>>) -> Self {
        for object in objects.iter() {
            // Invalid handles are not allowed.
            assert!(object.is_some());
        }
        let begin = begin.min(end);
        let end = end.max(begin);
        Self {
            begin: begin.clamp(0.0, 1.0),
            end: end.clamp(0.0, 1.0),
            objects,
        }
    }

    /// Sets new starting point in distance range. Input value will be clamped in
    /// (0; 1) range.
    pub fn set_begin(&mut self, percent: f32) {
        self.begin = percent.clamp(0.0, 1.0);
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
        self.end = percent.clamp(0.0, 1.0);
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
#[derive(Debug, Default, Clone, Visit, Reflect, PartialEq)]
pub struct LodGroup {
    /// Set of cascades.
    pub levels: Vec<LevelOfDetail>,
}

uuid_provider!(LodGroup = "8e7b18b1-c1e0-47d7-b952-4394c1d049e5");

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
    Reflect,
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

uuid_provider!(Mobility = "57c125ff-e408-4318-9874-f59485e95764");

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
#[derive(Debug, Visit, Reflect, PartialEq, Clone, AsRefStr, EnumString, EnumVariantNames)]
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

uuid_provider!(PropertyValue = "cce94b60-a57e-48ba-b6f4-e5e84788f7f8");

impl Default for PropertyValue {
    fn default() -> Self {
        Self::I8(0)
    }
}

/// A custom property.
#[derive(Debug, Visit, Reflect, Default, Clone, PartialEq)]
pub struct Property {
    /// Name of the property.
    pub name: String,
    /// A value of the property.
    pub value: PropertyValue,
}

uuid_provider!(Property = "fc87fd21-a5e6-40d5-a79d-19f96b25d6c9");

/// A script message from scene node. It is used for deferred initialization/deinitialization.
pub enum NodeScriptMessage {
    /// A script was set to a node and needs to be initialized.
    InitializeScript {
        /// Node handle.
        handle: Handle<Node>,
    },
    /// A node script must be destroyed. It can happen if the script was replaced with some other
    /// or a node was destroyed.
    DestroyScript {
        /// Script instance.
        script: Script,
        /// Node handle.
        handle: Handle<Node>,
    },
}

/// Unique id of the node. It can be shared across multiple resources (read - prefabs), to preserve parent-child
/// links. It is useful to create various resources that can bind to any instance of the node. For example, an
/// animation resource could be made for a specific node, but with the `instance_id` it can be retargetted to any
/// instance of the node. Instance here means: any copy of the node in any resource (including nested prefabs).
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Default, Debug, Reflect)]
#[repr(transparent)]
#[reflect(hide_all)]
pub struct InstanceId(pub Uuid);

impl Visit for InstanceId {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

/// Base scene graph node is a simplest possible node, it is used to build more complex ones using composition.
/// It contains all fundamental properties for each scene graph nodes, like local and global transforms, name,
/// lifetime, etc. Base node is a building block for all complex node hierarchies - it contains list of children
/// and handle to parent node.
///
/// # Example
///
/// ```
/// use fyrox::scene::base::BaseBuilder;
/// use fyrox::scene::graph::Graph;
/// use fyrox::scene::node::Node;
/// use fyrox::core::pool::Handle;
/// use fyrox::scene::pivot::PivotBuilder;
///
/// fn create_pivot_node(graph: &mut Graph) -> Handle<Node> {
///     PivotBuilder::new(BaseBuilder::new()
///         .with_name("BaseNode"))
///         .build(graph)
/// }
/// ```
#[derive(Debug, Reflect, Clone)]
pub struct Base {
    #[reflect(hidden)]
    pub(crate) self_handle: Handle<Node>,

    #[reflect(hidden)]
    pub(crate) script_message_sender: Option<Sender<NodeScriptMessage>>,

    // Name is not inheritable, because property inheritance works bad with external 3D models.
    // They use names to search "original" nodes.
    #[reflect(setter = "set_name_internal")]
    pub(crate) name: String,

    pub(crate) local_transform: Transform,

    #[reflect(setter = "set_visibility")]
    visibility: InheritableVariable<bool>,

    #[reflect(
        description = "Maximum amount of Some(time) that node will \"live\" or None if the node has unlimited lifetime."
    )]
    pub(crate) lifetime: InheritableVariable<Option<f32>>,

    #[reflect(min_value = 0.0, max_value = 1.0, step = 0.1)]
    #[reflect(setter = "set_depth_offset_factor")]
    depth_offset: InheritableVariable<f32>,

    #[reflect(setter = "set_lod_group")]
    lod_group: InheritableVariable<Option<LodGroup>>,

    #[reflect(setter = "set_mobility")]
    mobility: InheritableVariable<Mobility>,

    #[reflect(setter = "set_tag")]
    tag: InheritableVariable<String>,

    #[reflect(setter = "set_cast_shadows")]
    cast_shadows: InheritableVariable<bool>,

    /// A set of custom properties that can hold almost any data. It can be used to set additional
    /// properties to scene nodes.

    #[reflect(setter = "set_properties")]
    pub properties: InheritableVariable<Vec<Property>>,

    #[reflect(setter = "set_frustum_culling")]
    frustum_culling: InheritableVariable<bool>,

    #[reflect(hidden)]
    pub(crate) transform_modified: Cell<bool>,

    // When `true` it means that this node is instance of `resource`.
    // More precisely - this node is root of whole descendant nodes
    // hierarchy which was instantiated from resource.
    #[reflect(read_only)]
    pub(crate) is_resource_instance_root: bool,

    #[reflect(hidden)]
    pub(crate) global_visibility: Cell<bool>,

    #[reflect(hidden)]
    pub(crate) parent: Handle<Node>,

    #[reflect(hidden)]
    pub(crate) children: Vec<Handle<Node>>,

    #[reflect(hidden)]
    pub(crate) global_transform: Cell<Matrix4<f32>>,

    // Bone-specific matrix. Non-serializable.
    #[reflect(hidden)]
    pub(crate) inv_bind_pose_transform: Matrix4<f32>,

    // A resource from which this node was instantiated from, can work in pair
    // with `original` handle to get corresponding node from resource.
    #[reflect(read_only)]
    pub(crate) resource: Option<ModelResource>,

    // Handle to node in scene of model resource from which this node
    // was instantiated from.
    #[reflect(read_only)]
    #[reflect(hidden)]
    pub(crate) original_handle_in_resource: Handle<Node>,

    #[reflect(read_only)]
    #[reflect(hidden)]
    pub(crate) instance_id: InstanceId,

    // Current script of the scene node.
    //
    // # Important notes
    //
    // WARNING: Setting a new script via reflection will break normal script destruction process!
    // Use it at your own risk only when you're completely sure what you are doing.
    #[reflect(setter = "set_script_internal")]
    pub(crate) script: Option<Script>,

    enabled: InheritableVariable<bool>,

    #[reflect(hidden)]
    pub(crate) global_enabled: Cell<bool>,
}

impl Drop for Base {
    fn drop(&mut self) {
        self.remove_script();
    }
}

impl Base {
    /// Sets name of node. Can be useful to mark a node to be able to find it later on.
    #[inline]
    pub fn set_name<N: AsRef<str>>(&mut self, name: N) {
        self.set_name_internal(name.as_ref().to_owned());
    }

    fn set_name_internal(&mut self, name: String) -> String {
        std::mem::replace(&mut self.name, name)
    }

    /// Returns name of node.
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns owned name of node.
    #[inline]
    pub fn name_owned(&self) -> String {
        self.name.clone()
    }

    /// Returns shared reference to local transform of a node, can be used to fetch
    /// some local spatial properties, such as position, rotation, scale, etc.
    #[inline]
    pub fn local_transform(&self) -> &Transform {
        &self.local_transform
    }

    /// Returns mutable reference to local transform of a node, can be used to set
    /// some local spatial properties, such as position, rotation, scale, etc.
    #[inline]
    pub fn local_transform_mut(&mut self) -> &mut Transform {
        self.transform_modified.set(true);
        &mut self.local_transform
    }

    /// Sets new local transform of a node.
    #[inline]
    pub fn set_local_transform(&mut self, transform: Transform) {
        self.local_transform = transform;
    }

    /// Tries to find properties by the name. The method returns an iterator because it possible
    /// to have multiple properties with the same name.
    #[inline]
    pub fn find_properties_ref<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Property> {
        self.properties.iter().filter(move |p| p.name == name)
    }

    /// Tries to find a first property with the given name.
    #[inline]
    pub fn find_first_property_ref(&self, name: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.name == name)
    }

    /// Sets a new set of properties of the node.
    #[inline]
    pub fn set_properties(&mut self, properties: Vec<Property>) -> Vec<Property> {
        std::mem::replace(
            self.properties.get_value_mut_and_mark_modified(),
            properties,
        )
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
    #[inline]
    pub fn set_lifetime(&mut self, time_seconds: Option<f32>) -> &mut Self {
        self.lifetime.set_value_and_mark_modified(time_seconds);
        self
    }

    /// Returns current lifetime of a node. Will be None if node has undefined lifetime.
    /// For more info about lifetimes see [`set_lifetime`](Self::set_lifetime).
    #[inline]
    pub fn lifetime(&self) -> Option<f32> {
        *self.lifetime
    }

    /// Returns handle of parent node.
    #[inline]
    pub fn parent(&self) -> Handle<Node> {
        self.parent
    }

    /// Returns slice of handles to children nodes. This can be used, for example, to
    /// traverse tree starting from some node.
    #[inline]
    pub fn children(&self) -> &[Handle<Node>] {
        self.children.as_slice()
    }

    /// Returns global transform matrix, such matrix contains combined transformation
    /// of transforms of parent nodes. This is the final matrix that describes real
    /// location of object in the world.
    #[inline]
    pub fn global_transform(&self) -> Matrix4<f32> {
        self.global_transform.get()
    }

    /// Returns inverse of bind pose matrix. Bind pose matrix - is special matrix
    /// for bone nodes, it stores initial transform of bone node at the moment
    /// of "binding" vertices to bones.
    #[inline]
    pub fn inv_bind_pose_transform(&self) -> Matrix4<f32> {
        self.inv_bind_pose_transform
    }

    /// Returns true if this node is model resource instance root node.
    #[inline]
    pub fn is_resource_instance_root(&self) -> bool {
        self.is_resource_instance_root
    }

    /// Returns resource from which this node was instantiated from.
    #[inline]
    pub fn resource(&self) -> Option<ModelResource> {
        self.resource.clone()
    }

    /// Sets local visibility of a node.
    #[inline]
    pub fn set_visibility(&mut self, visibility: bool) -> bool {
        self.visibility.set_value_and_mark_modified(visibility)
    }

    /// Returns local visibility of a node.
    #[inline]
    pub fn visibility(&self) -> bool {
        *self.visibility
    }

    /// Returns current **local-space** bounding box. Keep in mind that this value is just
    /// a placeholder, because there is not information to calculate actual bounding box.
    #[inline]
    pub fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::default()
    }

    /// Returns current **world-space** bounding box.
    #[inline]
    pub fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    /// Set new mobility for the node.
    ///
    /// TODO. Mobility still has no effect, it was designed to be used in combined
    /// rendering (dynamic + static lights (lightmaps))
    #[inline]
    pub fn set_mobility(&mut self, mobility: Mobility) -> Mobility {
        self.mobility.set_value_and_mark_modified(mobility)
    }

    /// Return current mobility of the node.
    #[inline]
    pub fn mobility(&self) -> Mobility {
        *self.mobility
    }

    /// Returns combined visibility of an node. This is the final visibility of a node. Global visibility calculated
    /// using visibility of all parent nodes until root one, so if some parent node upper on tree is invisible then
    /// all its children will be invisible. It defines if object will be rendered. It is *not* the same as real
    /// visibility from point of view of a camera. Use frustum-box intersection test instead.
    #[inline]
    pub fn global_visibility(&self) -> bool {
        self.global_visibility.get()
    }

    /// Handle to node in scene of model resource from which this node was instantiated from.
    ///
    /// # Notes
    ///
    /// This handle is extensively used to fetch information about the state of node in the resource
    /// to sync properties of instance with its original in the resource.
    #[inline]
    pub fn original_handle_in_resource(&self) -> Handle<Node> {
        self.original_handle_in_resource
    }

    /// Returns position of the node in absolute coordinates.
    #[inline]
    pub fn global_position(&self) -> Vector3<f32> {
        self.global_transform.get().position()
    }

    /// Returns "look" vector of global transform basis, in most cases return vector will be non-normalized.
    #[inline]
    pub fn look_vector(&self) -> Vector3<f32> {
        self.global_transform.get().look()
    }

    /// Returns "side" vector of global transform basis, in most cases return vector will be non-normalized.
    #[inline]
    pub fn side_vector(&self) -> Vector3<f32> {
        self.global_transform.get().side()
    }

    /// Returns "up" vector of global transform basis, in most cases return vector will be non-normalized.
    #[inline]
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
    #[inline]
    pub fn set_depth_offset_factor(&mut self, factor: f32) -> f32 {
        self.depth_offset
            .set_value_and_mark_modified(factor.abs().clamp(0.0, 1.0))
    }

    /// Returns depth offset factor.
    #[inline]
    pub fn depth_offset_factor(&self) -> f32 {
        *self.depth_offset
    }

    /// Sets new lod group.
    #[inline]
    pub fn set_lod_group(&mut self, lod_group: Option<LodGroup>) -> Option<LodGroup> {
        std::mem::replace(self.lod_group.get_value_mut_and_mark_modified(), lod_group)
    }

    /// Extracts lod group, leaving None in the node.
    #[inline]
    pub fn take_lod_group(&mut self) -> Option<LodGroup> {
        std::mem::take(self.lod_group.get_value_mut_and_mark_modified())
    }

    /// Returns shared reference to current lod group.
    #[inline]
    pub fn lod_group(&self) -> Option<&LodGroup> {
        self.lod_group.as_ref()
    }

    /// Returns mutable reference to current lod group.
    #[inline]
    pub fn lod_group_mut(&mut self) -> Option<&mut LodGroup> {
        self.lod_group.get_value_mut_and_mark_modified().as_mut()
    }

    /// Returns node tag.
    #[inline]
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Returns a copy of node tag.
    #[inline]
    pub fn tag_owned(&self) -> String {
        (*self.tag).clone()
    }

    /// Sets new tag.
    #[inline]
    pub fn set_tag(&mut self, tag: String) -> String {
        self.tag.set_value_and_mark_modified(tag)
    }

    /// Return the frustum_culling flag
    #[inline]
    pub fn frustum_culling(&self) -> bool {
        *self.frustum_culling
    }

    /// Sets whether to use frustum culling or not
    #[inline]
    pub fn set_frustum_culling(&mut self, frustum_culling: bool) -> bool {
        self.frustum_culling
            .set_value_and_mark_modified(frustum_culling)
    }

    /// Returns true if the node should cast shadows, false - otherwise.
    #[inline]
    pub fn cast_shadows(&self) -> bool {
        *self.cast_shadows
    }

    /// Sets whether the mesh should cast shadows or not.
    #[inline]
    pub fn set_cast_shadows(&mut self, cast_shadows: bool) -> bool {
        self.cast_shadows.set_value_and_mark_modified(cast_shadows)
    }

    /// Sets instance id of the node. See [`InstanceId`] for more info.
    ///
    /// ## Important notes
    ///
    /// Do not change instance id unless you're know for sure what you're doing! Changing the id could result in
    /// broken parent-child relation between prefabs. This method has very limited usage when you need to override
    /// the id with some hand-made value.
    pub fn set_instance_id(&mut self, id: InstanceId) {
        self.instance_id = id;
    }

    /// Returns current instance id.
    pub fn instance_id(&self) -> InstanceId {
        self.instance_id
    }

    fn remove_script(&mut self) {
        // Send script to the graph to destroy script instances correctly.
        if let Some(script) = self.script.take() {
            if let Some(sender) = self.script_message_sender.as_ref() {
                Log::verify(sender.send(NodeScriptMessage::DestroyScript {
                    script,
                    handle: self.self_handle,
                }));
            } else {
                Log::warn(format!(
                    "There is a script instance on a node {}, but no message sender. \
                    The script won't be correctly destroyed!",
                    self.name(),
                ))
            }
        }
    }

    /// Sets new script for the scene node.
    #[inline]
    pub fn set_script(&mut self, script: Option<Script>) {
        self.remove_script();
        self.script = script;
        if let Some(sender) = self.script_message_sender.as_ref() {
            if self.script.is_some() {
                Log::verify(sender.send(NodeScriptMessage::InitializeScript {
                    handle: self.self_handle,
                }));
            }
        }
    }

    fn set_script_internal(&mut self, script: Option<Script>) -> Option<Script> {
        std::mem::replace(&mut self.script, script)
    }

    /// Checks if the node has a script of a particular type. Returns `false` if there is no script
    /// at all, or if the script is not of a given type.
    #[inline]
    pub fn has_script<T: ScriptTrait>(&self) -> bool {
        self.try_get_script::<T>().is_some()
    }

    /// Tries to cast current script instance (if any) to given type and returns a shared reference
    /// to it on successful cast.
    #[inline]
    pub fn try_get_script<T: ScriptTrait>(&self) -> Option<&T> {
        self.script.as_ref().and_then(|s| s.cast::<T>())
    }

    /// Tries to fetch a reference to a component of the given type from the script of the node.
    #[inline]
    pub fn try_get_script_component<C>(&self) -> Option<&C>
    where
        C: Any,
    {
        self.script
            .as_ref()
            .and_then(|s| s.query_component_ref::<C>())
    }

    /// Tries to fetch a reference to a component of the given type from the script of the node.
    #[inline]
    pub fn try_get_script_component_mut<C>(&mut self) -> Option<&mut C>
    where
        C: Any,
    {
        self.script
            .as_mut()
            .and_then(|s| s.query_component_mut::<C>())
    }

    /// Tries to cast current script instance (if any) to given type and returns a mutable reference
    /// to it on successful cast.
    #[inline]
    pub fn try_get_script_mut<T: ScriptTrait>(&mut self) -> Option<&mut T> {
        self.script.as_mut().and_then(|s| s.cast_mut::<T>())
    }

    /// Returns shared reference to current script instance.
    #[inline]
    pub fn script(&self) -> Option<&Script> {
        self.script.as_ref()
    }

    /// Returns mutable reference to current script instance.
    ///
    /// # Important notes
    ///
    /// Do **not** replace script instance using mutable reference given to you by this method.
    /// This will prevent correct script de-initialization! Use `Self::set_script` if you need
    /// to replace the script.
    #[inline]
    pub fn script_mut(&mut self) -> Option<&mut Script> {
        self.script.as_mut()
    }

    /// Returns a copy of the current script.
    #[inline]
    pub fn script_cloned(&self) -> Option<Script> {
        self.script.clone()
    }

    /// Internal. Do not use.
    #[inline]
    pub fn script_inner(&mut self) -> &mut Option<Script> {
        &mut self.script
    }

    /// Enables or disables scene node. Disabled scene nodes won't be updated (including scripts) or rendered.
    ///
    /// # Important notes
    ///
    /// Enabled/disabled state will affect children nodes. It means that if you have a node with children nodes,
    /// and you disable the node, all children nodes will be disabled too even if their [`Self::is_enabled`] method
    /// returns `true`.
    #[inline]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled.set_value_and_mark_modified(enabled);
    }

    /// Returns `true` if the node is enabled, `false` - otherwise. The return value does **not** include the state
    /// of parent nodes. It should be considered as "local" enabled flag. To get actual enabled state, that includes
    /// the state of parent nodes, use [`Self::is_globally_enabled`] method.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        *self.enabled
    }

    /// Returns `true` if the node and every parent up in hierarchy is enabled, `false` - otherwise. This method
    /// returns "true" `enabled` flag. Its value could be different from the value returned by [`Self::is_enabled`].
    #[inline]
    pub fn is_globally_enabled(&self) -> bool {
        self.global_enabled.get()
    }

    /// Returns a root resource of the scene node. This method crawls up on dependency tree until it finds that
    /// the ancestor node does not have any dependencies and returns this resource as the root resource. For
    /// example, in case of simple scene node instance, this method will return the resource from which the node
    /// was instantiated from. In case of 2 or more levels of dependency, it will always return the "top"
    /// dependency in the dependency graph.
    #[inline]
    pub fn root_resource(&self) -> Option<ModelResource> {
        if let Some(resource) = self.resource.as_ref() {
            let mut state = resource.state();
            if let Some(model) = state.data() {
                if let Some(ancestor_node) = model
                    .get_scene()
                    .graph
                    .try_get(self.original_handle_in_resource)
                {
                    return if ancestor_node.resource.is_none() {
                        Some(resource.clone())
                    } else {
                        ancestor_node.root_resource()
                    };
                }
            }
        }
        None
    }
}

impl Default for Base {
    fn default() -> Self {
        BaseBuilder::new().build_base()
    }
}

// Serializes Option<Script> using given serializer.
fn visit_opt_script(name: &str, script: &mut Option<Script>, visitor: &mut Visitor) -> VisitResult {
    let mut region = visitor.enter_region(name)?;

    let mut script_type_uuid = script.as_ref().map(|s| s.id()).unwrap_or_default();
    script_type_uuid.visit("TypeUuid", &mut region)?;

    if region.is_reading() {
        *script = if script_type_uuid.is_nil() {
            None
        } else {
            let serialization_context = region
                .blackboard
                .get::<SerializationContext>()
                .expect("Visitor blackboard must contain serialization context!");

            Some(
                serialization_context
                    .script_constructors
                    .try_create(&script_type_uuid)
                    .ok_or_else(|| {
                        VisitError::User(format!(
                            "There is no corresponding script constructor for {} type!",
                            script_type_uuid
                        ))
                    })?,
            )
        };
    }

    if let Some(script) = script {
        script.visit("ScriptData", &mut region)?;
    }

    Ok(())
}

impl Visit for Base {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        if self.name.visit("Name", &mut region).is_err() {
            // Name was wrapped into `InheritableVariable` previously, so we must maintain
            // backward compatibility here.
            let mut region = region.enter_region("Name")?;
            let mut value = String::default();
            value.visit("Value", &mut region)?;
            self.name = value;
        }
        self.local_transform.visit("Transform", &mut region)?;
        self.visibility.visit("Visibility", &mut region)?;
        self.parent.visit("Parent", &mut region)?;
        self.children.visit("Children", &mut region)?;
        self.resource.visit("Resource", &mut region)?;
        self.is_resource_instance_root
            .visit("IsResourceInstance", &mut region)?;
        self.lifetime.visit("Lifetime", &mut region)?;
        self.depth_offset.visit("DepthOffset", &mut region)?;
        self.lod_group.visit("LodGroup", &mut region)?;
        self.mobility.visit("Mobility", &mut region)?;
        self.original_handle_in_resource
            .visit("Original", &mut region)?;
        self.tag.visit("Tag", &mut region)?;
        let _ = self.properties.visit("Properties", &mut region);
        let _ = self.frustum_culling.visit("FrustumCulling", &mut region);
        let _ = self.cast_shadows.visit("CastShadows", &mut region);
        let _ = self.instance_id.visit("InstanceId", &mut region);
        let _ = self.enabled.visit("Enabled", &mut region);

        // Script visiting may fail for various reasons:
        //
        // 1) Data inside a script is not compatible with latest code (there is no backward
        //    compatibility for the data)
        // 2) Script was removed in the game.
        //
        // None of the reasons are fatal and we should still give an ability to load such node
        // to edit or remove it.
        if let Err(e) = visit_opt_script("Script", &mut self.script, &mut region) {
            // Do not spam with error messages if there is missing `Script` field. It is ok
            // for old scenes not to have script at all.
            if !matches!(e, VisitError::RegionDoesNotExist(_)) {
                Log::err(format!("Unable to visit script. Reason: {:?}", e))
            }
        }

        Ok(())
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
    cast_shadows: bool,
    script: Option<Script>,
    instance_id: InstanceId,
    enabled: bool,
}

impl Default for BaseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseBuilder {
    /// Creates new builder instance.
    #[inline]
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
            cast_shadows: true,
            script: None,
            instance_id: InstanceId(Uuid::new_v4()),
            enabled: true,
        }
    }

    /// Sets desired mobility.
    #[inline]
    pub fn with_mobility(mut self, mobility: Mobility) -> Self {
        self.mobility = mobility;
        self
    }

    /// Sets desired name.
    #[inline]
    pub fn with_name<P: AsRef<str>>(mut self, name: P) -> Self {
        self.name = name.as_ref().to_owned();
        self
    }

    /// Sets desired visibility.
    #[inline]
    pub fn with_visibility(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    /// Sets desired local transform.
    #[inline]
    pub fn with_local_transform(mut self, transform: Transform) -> Self {
        self.local_transform = transform;
        self
    }

    /// Sets desired inverse bind pose transform.
    #[inline]
    pub fn with_inv_bind_pose_transform(mut self, inv_bind_pose: Matrix4<f32>) -> Self {
        self.inv_bind_pose_transform = inv_bind_pose;
        self
    }

    /// Enables or disables the scene node.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets desired list of children nodes.
    #[inline]
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
    #[inline]
    pub fn with_lifetime(mut self, time_seconds: f32) -> Self {
        self.lifetime = Some(time_seconds);
        self
    }

    /// Sets desired depth offset.
    #[inline]
    pub fn with_depth_offset(mut self, offset: f32) -> Self {
        self.depth_offset = offset;
        self
    }

    /// Sets desired lod group.
    #[inline]
    pub fn with_lod_group(mut self, lod_group: LodGroup) -> Self {
        self.lod_group = Some(lod_group);
        self
    }

    /// Sets desired tag.
    #[inline]
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tag = tag;
        self
    }

    /// Sets desired frustum_culling flag.
    #[inline]
    pub fn with_frustum_culling(mut self, frustum_culling: bool) -> Self {
        self.frustum_culling = frustum_culling;
        self
    }

    /// Sets whether mesh should cast shadows or not.
    #[inline]
    pub fn with_cast_shadows(mut self, cast_shadows: bool) -> Self {
        self.cast_shadows = cast_shadows;
        self
    }

    /// Sets desired script of the node.
    #[inline]
    pub fn with_script(mut self, script: Script) -> Self {
        self.script = Some(script);
        self
    }

    /// Sets new instance id.
    pub fn with_instance_id(mut self, id: InstanceId) -> Self {
        self.instance_id = id;
        self
    }

    /// Creates an instance of [`Base`].
    #[inline]
    pub fn build_base(self) -> Base {
        Base {
            self_handle: Default::default(),
            script_message_sender: None,
            name: self.name,
            children: self.children,
            local_transform: self.local_transform,
            lifetime: self.lifetime.into(),
            visibility: self.visibility.into(),
            global_visibility: Cell::new(true),
            parent: Handle::NONE,
            global_transform: Cell::new(Matrix4::identity()),
            inv_bind_pose_transform: self.inv_bind_pose_transform,
            resource: None,
            original_handle_in_resource: Handle::NONE,
            is_resource_instance_root: false,
            depth_offset: self.depth_offset.into(),
            lod_group: self.lod_group.into(),
            mobility: self.mobility.into(),
            tag: self.tag.into(),
            properties: Default::default(),
            transform_modified: Cell::new(false),
            frustum_culling: self.frustum_culling.into(),
            cast_shadows: self.cast_shadows.into(),
            script: self.script,
            instance_id: InstanceId(Uuid::new_v4()),
            enabled: self.enabled.into(),
            global_enabled: Cell::new(true),
        }
    }
}
