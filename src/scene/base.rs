//! Contains all structures and methods to create and manage base scene graph nodes.
//!
//! For more info see [`Base`]

use crate::{
    core::{
        algebra::{Matrix4, Vector3},
        inspect::{Inspect, PropertyInfo},
        math::{aabb::AxisAlignedBoundingBox, Matrix4Ext},
        pool::{ErasedHandle, Handle},
        reflect::Reflect,
        variable::TemplateVariable,
        visitor::{Visit, VisitError, VisitResult, Visitor},
        VecExtensions,
    },
    engine::{resource_manager::ResourceManager, SerializationContext},
    resource::model::Model,
    scene::{graph::map::NodeHandleMap, node::Node, transform::Transform},
    script::Script,
    utils::log::Log,
};
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

/// A handle to scene node that will be controlled by LOD system.
#[derive(Inspect, Reflect, Default, Debug, Clone, Copy, PartialEq, Hash, Eq)]
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
#[derive(Debug, Default, Clone, Visit, Inspect, Reflect, PartialEq)]
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
#[derive(Debug, Default, Clone, Visit, Inspect, Reflect, PartialEq)]
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
#[derive(
    Debug, Visit, Inspect, Reflect, PartialEq, Clone, AsRefStr, EnumString, EnumVariantNames,
)]
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
#[derive(Debug, Visit, Inspect, Reflect, Default, Clone, PartialEq)]
pub struct Property {
    /// Name of the property.
    pub name: String,
    /// A value of the property.
    pub value: PropertyValue,
}

/// A script message from scene node. It is used for deferred initialization/deinitialization.
pub enum ScriptMessage {
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
#[derive(Debug, Inspect, Reflect)]
pub struct Base {
    #[inspect(skip)]
    #[reflect(hidden)]
    pub(crate) self_handle: Handle<Node>,

    #[inspect(skip)]
    #[reflect(hidden)]
    pub(crate) script_message_sender: Option<Sender<ScriptMessage>>,

    #[inspect(deref)]
    #[reflect(setter = "set_name_internal")]
    pub(crate) name: TemplateVariable<String>,

    pub(crate) local_transform: Transform,

    #[inspect(deref)]
    #[reflect(setter = "set_visibility")]
    visibility: TemplateVariable<bool>,

    // Maximum amount of Some(time) that node will "live" or None
    // if node has undefined lifetime.
    #[inspect(skip)] // TEMPORARILY HIDDEN. It causes crashes when set from the editor.
    pub(crate) lifetime: TemplateVariable<Option<f32>>,

    #[inspect(min_value = 0.0, max_value = 1.0, step = 0.1, deref)]
    #[reflect(setter = "set_depth_offset_factor")]
    depth_offset: TemplateVariable<f32>,

    #[inspect(deref)]
    #[reflect(setter = "set_lod_group")]
    lod_group: TemplateVariable<Option<LodGroup>>,

    #[inspect(deref)]
    #[reflect(setter = "set_mobility")]
    mobility: TemplateVariable<Mobility>,

    #[inspect(deref)]
    #[reflect(setter = "set_tag")]
    tag: TemplateVariable<String>,

    #[inspect(deref)]
    #[reflect(setter = "set_cast_shadows")]
    cast_shadows: TemplateVariable<bool>,

    /// A set of custom properties that can hold almost any data. It can be used to set additional
    /// properties to scene nodes.
    #[inspect(deref)]
    #[reflect(setter = "set_properties")]
    pub properties: TemplateVariable<Vec<Property>>,

    #[inspect(deref)]
    #[reflect(setter = "set_frustum_culling")]
    frustum_culling: TemplateVariable<bool>,

    #[inspect(skip)]
    #[reflect(hidden)]
    pub(crate) transform_modified: Cell<bool>,

    // When `true` it means that this node is instance of `resource`.
    // More precisely - this node is root of whole descendant nodes
    // hierarchy which was instantiated from resource.
    #[inspect(read_only)]
    pub(crate) is_resource_instance_root: bool,

    #[inspect(skip)]
    #[reflect(hidden)]
    pub(crate) global_visibility: Cell<bool>,

    #[inspect(skip)]
    #[reflect(hidden)]
    pub(crate) parent: Handle<Node>,

    #[inspect(skip)]
    #[reflect(hidden)]
    pub(crate) children: Vec<Handle<Node>>,

    #[inspect(skip)]
    #[reflect(hidden)]
    pub(crate) global_transform: Cell<Matrix4<f32>>,

    // Bone-specific matrix. Non-serializable.
    #[inspect(skip)]
    #[reflect(hidden)]
    pub(crate) inv_bind_pose_transform: Matrix4<f32>,

    // A resource from which this node was instantiated from, can work in pair
    // with `original` handle to get corresponding node from resource.
    #[inspect(read_only)]
    #[reflect(hidden)]
    pub(crate) resource: Option<Model>,

    // Handle to node in scene of model resource from which this node
    // was instantiated from.
    #[inspect(read_only)]
    #[reflect(hidden)]
    pub(crate) original_handle_in_resource: Handle<Node>,

    // Current script of the scene node.
    //
    // # Important notes
    //
    // WARNING: Setting a new script via reflection will break normal script destruction process!
    // Use it at your own risk only when you're completely sure what you are doing.
    #[reflect(setter = "set_script_internal")]
    pub(crate) script: Option<Script>,
}

impl Drop for Base {
    fn drop(&mut self) {
        self.remove_script();
    }
}

impl Clone for Base {
    fn clone(&self) -> Self {
        Self {
            self_handle: Default::default(), // Intentionally not copied!
            script_message_sender: None,     // Intentionally not copied!
            name: self.name.clone(),
            local_transform: self.local_transform.clone(),
            global_transform: self.global_transform.clone(),
            visibility: self.visibility.clone(),
            global_visibility: self.global_visibility.clone(),
            inv_bind_pose_transform: self.inv_bind_pose_transform,
            resource: self.resource.clone(),
            original_handle_in_resource: self.original_handle_in_resource,
            is_resource_instance_root: self.is_resource_instance_root,
            lifetime: self.lifetime.clone(),
            mobility: self.mobility.clone(),
            tag: self.tag.clone(),
            lod_group: self.lod_group.clone(),
            properties: self.properties.clone(),
            frustum_culling: self.frustum_culling.clone(),
            depth_offset: self.depth_offset.clone(),
            cast_shadows: self.cast_shadows.clone(),
            script: self.script.clone(),

            // Rest of data is *not* copied!
            parent: Default::default(),
            children: Default::default(),
            transform_modified: Cell::new(false),
        }
    }
}

impl Base {
    /// Sets name of node. Can be useful to mark a node to be able to find it later on.
    pub fn set_name<N: AsRef<str>>(&mut self, name: N) {
        self.set_name_internal(name.as_ref().to_owned());
    }

    fn set_name_internal(&mut self, name: String) -> String {
        self.name.set(name)
    }

    /// Returns name of node.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns owned name of node.
    pub fn name_owned(&self) -> String {
        (*self.name).clone()
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
    pub fn set_local_transform(&mut self, transform: Transform) {
        self.local_transform = transform;
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

    /// Sets a new set of properties of the node.
    pub fn set_properties(&mut self, properties: Vec<Property>) -> Vec<Property> {
        std::mem::replace(self.properties.get_mut(), properties)
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
        self.lifetime.set(time_seconds);
        self
    }

    /// Returns current lifetime of a node. Will be None if node has undefined lifetime.
    /// For more info about lifetimes see [`set_lifetime`](Self::set_lifetime).
    pub fn lifetime(&self) -> Option<f32> {
        *self.lifetime
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
    pub fn set_visibility(&mut self, visibility: bool) -> bool {
        self.visibility.set(visibility)
    }

    /// Returns local visibility of a node.
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
    pub fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    /// Set new mobility for the node.
    ///
    /// TODO. Mobility still has no effect, it was designed to be used in combined
    /// rendering (dynamic + static lights (lightmaps))
    pub fn set_mobility(&mut self, mobility: Mobility) -> Mobility {
        self.mobility.set(mobility)
    }

    /// Return current mobility of the node.
    pub fn mobility(&self) -> Mobility {
        *self.mobility
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
    pub fn set_depth_offset_factor(&mut self, factor: f32) -> f32 {
        self.depth_offset.set(factor.abs().min(1.0).max(0.0))
    }

    /// Returns depth offset factor.
    pub fn depth_offset_factor(&self) -> f32 {
        *self.depth_offset
    }

    /// Sets new lod group.
    pub fn set_lod_group(&mut self, lod_group: Option<LodGroup>) -> Option<LodGroup> {
        std::mem::replace(self.lod_group.get_mut(), lod_group)
    }

    /// Extracts lod group, leaving None in the node.
    pub fn take_lod_group(&mut self) -> Option<LodGroup> {
        std::mem::take(self.lod_group.get_mut())
    }

    /// Returns shared reference to current lod group.
    pub fn lod_group(&self) -> Option<&LodGroup> {
        self.lod_group.as_ref()
    }

    /// Returns mutable reference to current lod group.
    pub fn lod_group_mut(&mut self) -> Option<&mut LodGroup> {
        self.lod_group.get_mut().as_mut()
    }

    /// Returns node tag.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Returns a copy of node tag.
    pub fn tag_owned(&self) -> String {
        (*self.tag).clone()
    }

    /// Sets new tag.
    pub fn set_tag(&mut self, tag: String) -> String {
        self.tag.set(tag)
    }

    /// Return the frustum_culling flag
    pub fn frustum_culling(&self) -> bool {
        *self.frustum_culling
    }

    /// Sets whether to use frustum culling or not
    pub fn set_frustum_culling(&mut self, frustum_culling: bool) -> bool {
        self.frustum_culling.set(frustum_culling)
    }

    /// Returns true if the node should cast shadows, false - otherwise.
    #[inline]
    pub fn cast_shadows(&self) -> bool {
        *self.cast_shadows
    }

    /// Sets whether the mesh should cast shadows or not.
    #[inline]
    pub fn set_cast_shadows(&mut self, cast_shadows: bool) -> bool {
        self.cast_shadows.set(cast_shadows)
    }

    fn remove_script(&mut self) {
        // Send script to the graph to destroy script instances correctly.
        if let Some(script) = self.script.take() {
            if let Some(sender) = self.script_message_sender.as_ref() {
                Log::verify(sender.send(ScriptMessage::DestroyScript {
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
    pub fn set_script(&mut self, script: Option<Script>) {
        self.remove_script();
        self.script = script;
        if let Some(sender) = self.script_message_sender.as_ref() {
            if self.script.is_some() {
                Log::verify(sender.send(ScriptMessage::InitializeScript {
                    handle: self.self_handle,
                }));
            }
        }
    }

    fn set_script_internal(&mut self, script: Option<Script>) -> Option<Script> {
        std::mem::replace(&mut self.script, script)
    }

    /// Returns shared reference to current script instance.
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
    pub fn script_mut(&mut self) -> Option<&mut Script> {
        self.script.as_mut()
    }

    /// Returns a copy of the current script.
    pub fn script_cloned(&self) -> Option<Script> {
        self.script.clone()
    }

    /// Internal. Do not use.
    pub fn script_inner(&mut self) -> &mut Option<Script> {
        &mut self.script
    }

    /// Updates node lifetime and returns true if the node is still alive, false - otherwise.
    pub(crate) fn update_lifetime(&mut self, dt: f32) -> bool {
        if let Some(lifetime) = self.lifetime.get_mut_silent().as_mut() {
            *lifetime -= dt;
            *lifetime >= 0.0
        } else {
            true
        }
    }

    pub(crate) fn restore_resources(&mut self, resource_manager: ResourceManager) {
        if let Some(script) = self.script.as_mut() {
            script.restore_resources(resource_manager);
        }
    }

    pub(crate) fn remap_handles(&mut self, old_new_mapping: &NodeHandleMap) {
        for property in self.properties.get_mut_silent().iter_mut() {
            if let PropertyValue::NodeHandle(ref mut handle) = property.value {
                if !old_new_mapping.try_map(handle) {
                    Log::warn(format!(
                        "Unable to remap node handle property {} of a node {}. Handle is {}!",
                        property.name, *self.name, handle
                    ))
                }
            }
        }

        // LODs also have handles that must be remapped too.
        if let Some(lod_group) = self.lod_group.get_mut_silent() {
            for level in lod_group.levels.iter_mut() {
                level.objects.retain_mut_ext(|object| {
                    if old_new_mapping.try_map(object) {
                        true
                    } else {
                        Log::warn(format!(
                            "Unable to remap LOD object handle of a node {}. Handle is {}!",
                            *self.name, object.0
                        ));

                        // Discard invalid handles.
                        false
                    }
                });
            }
        }
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
                .environment
                .as_ref()
                .and_then(|e| e.downcast_ref::<SerializationContext>())
                .expect("Visitor environment must contain serialization context!");

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

        self.name.visit("Name", &mut region)?;
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
            cast_shadows: true,
            script: None,
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

    /// Sets whether mesh should cast shadows or not.
    pub fn with_cast_shadows(mut self, cast_shadows: bool) -> Self {
        self.cast_shadows = cast_shadows;
        self
    }

    /// Sets desired script of the node.
    pub fn with_script(mut self, script: Script) -> Self {
        self.script = Some(script);
        self
    }

    /// Creates an instance of [`Base`].
    pub fn build_base(self) -> Base {
        Base {
            self_handle: Default::default(),
            script_message_sender: None,
            name: self.name.into(),
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
        }
    }
}

#[cfg(test)]
pub mod test {
    use crate::{
        core::{reflect::Reflect, variable::try_inherit_properties},
        scene::base::{BaseBuilder, LevelOfDetail, LodGroup, Mobility},
    };

    pub fn check_inheritable_properties_equality(entity_a: &dyn Reflect, entity_b: &dyn Reflect) {
        for (a, b) in entity_a.fields().iter().zip(entity_b.fields()) {
            if let (Some(ta), Some(tb)) = ((*a).as_template_variable(), b.as_template_variable()) {
                if !ta.value_equals(tb) {
                    panic!("Value of property {:?} is not equal to {:?}", ta, tb)
                }
            }

            check_inheritable_properties_equality(*a, b);
        }
    }

    #[test]
    fn test_base_inheritance() {
        let parent = BaseBuilder::new()
            .with_visibility(false)
            .with_depth_offset(1.0)
            .with_tag("Tag".to_string())
            .with_name("Name")
            .with_lifetime(1.0)
            .with_frustum_culling(false)
            .with_mobility(Mobility::Static)
            .with_lod_group(LodGroup {
                levels: vec![LevelOfDetail {
                    begin: 0.0,
                    end: 1.0,
                    objects: vec![],
                }],
            })
            .build_base();

        let mut child = BaseBuilder::new().build_base();

        try_inherit_properties(&mut child, &parent).unwrap();

        check_inheritable_properties_equality(&child.local_transform, &parent.local_transform);
        check_inheritable_properties_equality(&child, &parent)
    }
}
