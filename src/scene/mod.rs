#![warn(missing_docs)]

//! Contains all structures and methods to create and manage 3D scenes.
//!
//! A `Scene` is a container for graph nodes, animations and physics.

pub mod accel;
pub mod base;
pub mod camera;
pub mod collider;
pub mod debug;
pub mod decal;
pub mod dim2;
pub mod graph;
pub mod joint;
pub mod light;
pub mod mesh;
pub mod node;
pub mod particle_system;
pub mod pivot;
pub mod rigidbody;
pub mod sound;
pub mod sprite;
pub mod terrain;
pub mod transform;
pub mod visibility;

use crate::{
    animation::{machine::container::AnimationMachineContainer, AnimationContainer},
    core::{
        algebra::Vector2,
        color::Color,
        futures::future::join_all,
        inspect::{Inspect, PropertyInfo},
        instant,
        pool::{Handle, Pool, Ticket},
        reflect::Reflect,
        sstorage::ImmutableString,
        visitor::{Visit, VisitError, VisitResult, Visitor},
    },
    engine::{resource_manager::ResourceManager, SerializationContext},
    material::{shader::SamplerFallback, PropertyValue},
    resource::texture::Texture,
    scene::{
        camera::Camera,
        debug::SceneDrawingContext,
        graph::{map::NodeHandleMap, Graph, GraphPerformanceStatistics},
        mesh::buffer::{
            VertexAttributeDataType, VertexAttributeDescriptor, VertexAttributeUsage,
            VertexWriteTrait,
        },
        mesh::Mesh,
        node::Node,
        sound::SoundEngine,
    },
    utils::{lightmap::Lightmap, log::Log, log::MessageKind, navmesh::Navmesh},
};
use fxhash::FxHashMap;
use std::{
    fmt::{Display, Formatter},
    ops::{Index, IndexMut},
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

/// A container for navigational meshes.
#[derive(Default, Clone, Debug, Visit)]
pub struct NavMeshContainer {
    pool: Pool<Navmesh>,
}

impl NavMeshContainer {
    /// Adds new navigational mesh to the container and returns its handle.
    pub fn add(&mut self, navmesh: Navmesh) -> Handle<Navmesh> {
        self.pool.spawn(navmesh)
    }

    /// Removes navigational mesh by its handle.
    pub fn remove(&mut self, handle: Handle<Navmesh>) -> Navmesh {
        self.pool.free(handle)
    }

    /// Creates new immutable iterator.
    pub fn iter(&self) -> impl Iterator<Item = &Navmesh> {
        self.pool.iter()
    }

    /// Creates new immutable iterator.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Navmesh> {
        self.pool.iter_mut()
    }

    /// Creates a handle to navmesh from its index.
    pub fn handle_from_index(&self, i: u32) -> Handle<Navmesh> {
        self.pool.handle_from_index(i)
    }

    /// Destroys all navmeshes. All handles will become invalid.
    pub fn clear(&mut self) {
        self.pool.clear()
    }

    /// Checks if given handle is valid.
    pub fn is_valid_handle(&self, handle: Handle<Navmesh>) -> bool {
        self.pool.is_valid_handle(handle)
    }

    /// Tries to borrow a navmesh by its index.
    pub fn at(&self, i: u32) -> Option<&Navmesh> {
        self.pool.at(i)
    }

    /// Tries to borrow a navmesh by its handle.
    pub fn try_get(&self, handle: Handle<Navmesh>) -> Option<&Navmesh> {
        self.pool.try_borrow(handle)
    }

    /// Tries to borrow a navmesh by its index.
    pub fn at_mut(&mut self, i: u32) -> Option<&mut Navmesh> {
        self.pool.at_mut(i)
    }

    /// Tries to borrow a navmesh by its handle.
    pub fn try_get_mut(&mut self, handle: Handle<Navmesh>) -> Option<&mut Navmesh> {
        self.pool.try_borrow_mut(handle)
    }
}

impl Index<Handle<Navmesh>> for NavMeshContainer {
    type Output = Navmesh;

    fn index(&self, index: Handle<Navmesh>) -> &Self::Output {
        &self.pool[index]
    }
}

impl IndexMut<Handle<Navmesh>> for NavMeshContainer {
    fn index_mut(&mut self, index: Handle<Navmesh>) -> &mut Self::Output {
        &mut self.pool[index]
    }
}

/// See module docs.
#[derive(Debug, Inspect, Reflect)]
pub struct Scene {
    /// Graph is main container for all scene nodes. It calculates global transforms for nodes,
    /// updates them and performs all other important work. See `graph` module docs for more
    /// info.
    pub graph: Graph,

    /// Animations container controls all animation on scene. Each animation can have tracks which
    /// has handles to graph nodes. See `animation` module docs for more info.
    #[inspect(skip)]
    #[reflect(hidden)]
    pub animations: AnimationContainer,

    /// Texture to draw scene to. If empty, scene will be drawn on screen directly.
    /// It is useful to "embed" some scene into other by drawing a quad with this
    /// texture. This can be used to make in-game video conference - you can make
    /// separate scene with your characters and draw scene into texture, then in
    /// main scene you can attach this texture to some quad which will be used as
    /// monitor. Other usage could be previewer of models, like pictogram of character
    /// in real-time strategies, in other words there are plenty of possible uses.
    #[inspect(skip)]
    #[reflect(hidden)]
    pub render_target: Option<Texture>,

    /// Drawing context for simple graphics.
    #[inspect(skip)]
    #[reflect(hidden)]
    pub drawing_context: SceneDrawingContext,

    /// A container for navigational meshes.
    #[inspect(skip)]
    #[reflect(hidden)]
    pub navmeshes: NavMeshContainer,

    /// Current lightmap.
    #[inspect(skip)]
    #[reflect(hidden)]
    lightmap: Option<Lightmap>,

    /// Performance statistics from last `update` call.
    #[inspect(skip)]
    #[reflect(hidden)]
    pub performance_statistics: PerformanceStatistics,

    /// Color of ambient lighting.
    pub ambient_lighting_color: Color,

    /// Whether the scene will be updated and rendered or not. Default is true.
    /// This flag allowing you to build a scene manager for your game. For example,
    /// you may have a scene for menu and one per level. Menu's scene is persistent,
    /// however you don't want it to be updated and renderer while you have a level
    /// loaded and playing a game. When you're start playing, just set `enabled` flag
    /// to false for menu's scene and when you need to open a menu - set it to true and
    /// set `enabled` flag to false for level's scene.
    pub enabled: bool,

    /// A container for animation blending state machines.
    #[inspect(skip)]
    #[reflect(hidden)]
    pub animation_machines: AnimationMachineContainer,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            graph: Default::default(),
            animations: Default::default(),
            render_target: None,
            lightmap: None,
            drawing_context: Default::default(),
            navmeshes: Default::default(),
            performance_statistics: Default::default(),
            ambient_lighting_color: Color::opaque(100, 100, 100),
            enabled: true,
            animation_machines: Default::default(),
        }
    }
}

/// A structure that holds times that specific update step took.
#[derive(Clone, Default, Debug)]
pub struct PerformanceStatistics {
    /// Graph performance statistics.
    pub graph: GraphPerformanceStatistics,

    /// A time which was required to update animations.
    pub animations_update_time: Duration,
}

impl Display for PerformanceStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Animations: {:?}\n\
            Graph: {:?}\n\
            \tSync Time: {:?}\n\
            \tSound: {:?}\n\
            \tPhysics: {:?}\n\
            \t\tSimulation: {:?}\n\
            \t\tRay cast: {:?}\n\
            \tPhysics 2D: {:?}\n\
            \t\tSimulation: {:?}\n\
            \t\tRay cast: {:?}\n\
            \tHierarchy: {:?}",
            self.animations_update_time,
            self.graph.total(),
            self.graph.sync_time,
            self.graph.sound_update_time,
            self.graph.physics.total(),
            self.graph.physics.step_time,
            self.graph.physics.total_ray_cast_time.get(),
            self.graph.physics2d.total(),
            self.graph.physics2d.step_time,
            self.graph.physics2d.total_ray_cast_time.get(),
            self.graph.hierarchical_properties_time,
        )
    }
}

/// Scene loader.
pub struct SceneLoader {
    scene: Scene,
}

impl SceneLoader {
    /// Tries to load scene from given file. File can contain any scene in native engine format.
    /// Such scenes can be made in rusty editor.
    pub async fn from_file<P: AsRef<Path>>(
        path: P,
        serialization_context: Arc<SerializationContext>,
    ) -> Result<Self, VisitError> {
        let mut visitor = Visitor::load_binary(path).await?;
        Self::load("Scene", serialization_context, &mut visitor)
    }

    /// Tries to load a scene using specified visitor and region name.
    pub fn load(
        region_name: &str,
        serialization_context: Arc<SerializationContext>,
        visitor: &mut Visitor,
    ) -> Result<Self, VisitError> {
        if !visitor.is_reading() {
            return Err(VisitError::User(
                "Visitor must be in read mode!".to_string(),
            ));
        }

        visitor.environment = Some(serialization_context);

        let mut scene = Scene::default();
        scene.visit(region_name, visitor)?;

        Ok(Self { scene })
    }

    /// Finishes scene loading.
    pub async fn finish(self, resource_manager: ResourceManager) -> Scene {
        let mut scene = self.scene;

        // Collect all model resources and wait for them. This step is crucial, because
        // later on resolve stage we'll extensively access parent resources to inherit
        // data from them and we can't read data of a resource being loading.
        let mut resources = Vec::new();
        for node in scene.graph.linear_iter_mut() {
            if let Some(shallow_resource) = node.resource.clone() {
                let resource = resource_manager
                    .clone()
                    .request_model(&shallow_resource.state().path());
                node.resource = Some(resource.clone());
                resources.push(resource);
            }
        }

        let _ = join_all(resources).await;

        // Restore pointers to resources. Scene saves only paths to resources, here we must
        // find real resources instead.
        for node in scene.graph.linear_iter_mut() {
            node.restore_resources(resource_manager.clone());
        }

        if let Some(lightmap) = scene.lightmap.as_mut() {
            for entries in lightmap.map.values_mut() {
                for entry in entries.iter_mut() {
                    resource_manager
                        .state()
                        .containers_mut()
                        .textures
                        .try_restore_optional_resource(&mut entry.texture);
                }
            }
        }

        // TODO: Move into Camera::restore_resources?
        // We have to wait until skybox textures are all loaded, because we need to read their data
        // to re-create cube map.
        let mut skybox_textures = Vec::new();
        for node in scene.graph.linear_iter() {
            if let Some(camera) = node.cast::<Camera>() {
                if let Some(skybox) = camera.skybox_ref() {
                    skybox_textures.extend(skybox.textures().iter().filter_map(|t| t.clone()));
                }
            }
        }
        join_all(skybox_textures).await;

        let mut animation_resources = Vec::new();
        for animation in scene.animations.iter_mut() {
            animation.restore_resources(resource_manager.clone());
            if let Some(resource) = animation.resource.as_ref() {
                animation_resources.push(resource.clone());
            }
        }
        join_all(animation_resources).await;

        let mut animation_machines = Vec::new();
        for machine in scene.animation_machines.iter_mut() {
            machine.restore_resources(resource_manager.clone());
            if let Some(resource) = machine.resource.as_ref() {
                animation_machines.push(resource.clone());
            }
        }
        join_all(animation_machines).await;

        // And do resolve to extract correct graphical data and so on.
        scene.resolve(resource_manager).await;

        scene
    }
}

impl Scene {
    /// Creates new scene with single root node.
    ///
    /// # Notes
    ///
    /// This method differs from Default trait implementation! Scene::default() creates
    /// empty graph with no nodes.
    #[inline]
    pub fn new() -> Self {
        Self {
            // Graph must be created with `new` method because it differs from `default`
            graph: Graph::new(),
            animations: Default::default(),
            render_target: None,
            lightmap: None,
            drawing_context: Default::default(),
            navmeshes: Default::default(),
            performance_statistics: Default::default(),
            ambient_lighting_color: Color::opaque(100, 100, 100),
            enabled: true,
            animation_machines: Default::default(),
        }
    }

    /// Removes node from scene with all associated entities, like animations etc. This method
    /// should be used all times instead of [Graph::remove_node](crate::scene::graph::Graph::remove_node).
    ///
    /// # Panics
    ///
    /// Panics if handle is invalid.
    pub fn remove_node(&mut self, handle: Handle<Node>) {
        for descendant in self.graph.traverse_handle_iter(handle) {
            // Remove all associated animations.
            self.animations.retain(|animation| {
                for track in animation.get_tracks() {
                    if track.get_node() == descendant {
                        return false;
                    }
                }
                true
            });
        }

        self.graph.remove_node(handle)
    }

    /// Synchronizes the state of the scene with external resources.
    pub async fn resolve(&mut self, resource_manager: ResourceManager) {
        Log::writeln(MessageKind::Information, "Starting resolve...");

        self.graph.resolve();
        self.animations.resolve(&self.graph);
        self.animation_machines
            .resolve(resource_manager, &mut self.graph, &mut self.animations)
            .await;
        self.graph.update_hierarchical_data();

        // Re-apply lightmap if any. This has to be done after resolve because we must patch surface
        // data at this stage, but if we'd do this before we wouldn't be able to do this because
        // meshes contains invalid surface data.
        if let Some(lightmap) = self.lightmap.as_mut() {
            // Patch surface data first. To do this we gather all surface data instances and
            // look in patch data if we have patch for data.
            let mut unique_data_set = FxHashMap::default();
            for &handle in lightmap.map.keys() {
                if let Some(mesh) = self.graph[handle].cast_mut::<Mesh>() {
                    for surface in mesh.surfaces() {
                        let data = surface.data();
                        let key = &*data as *const _ as u64;
                        unique_data_set.entry(key).or_insert(data);
                    }
                }
            }

            for (_, data) in unique_data_set.into_iter() {
                let mut data = data.lock();

                if let Some(patch) = lightmap.patches.get(&data.content_hash()) {
                    if !data
                        .vertex_buffer
                        .has_attribute(VertexAttributeUsage::TexCoord1)
                    {
                        data.vertex_buffer
                            .modify()
                            .add_attribute(
                                VertexAttributeDescriptor {
                                    usage: VertexAttributeUsage::TexCoord1,
                                    data_type: VertexAttributeDataType::F32,
                                    size: 2,
                                    divisor: 0,
                                    shader_location: 6, // HACK: GBuffer renderer expects it to be at 6
                                },
                                Vector2::<f32>::default(),
                            )
                            .unwrap();
                    }

                    data.geometry_buffer.set_triangles(patch.triangles.clone());

                    let mut vertex_buffer_mut = data.vertex_buffer.modify();
                    for &v in patch.additional_vertices.iter() {
                        vertex_buffer_mut.duplicate(v as usize);
                    }

                    assert_eq!(
                        vertex_buffer_mut.vertex_count() as usize,
                        patch.second_tex_coords.len()
                    );
                    for (mut view, &tex_coord) in vertex_buffer_mut
                        .iter_mut()
                        .zip(patch.second_tex_coords.iter())
                    {
                        view.write_2_f32(VertexAttributeUsage::TexCoord1, tex_coord)
                            .unwrap();
                    }
                } else {
                    Log::writeln(
                        MessageKind::Warning,
                        "Failed to get surface data patch while resolving lightmap!\
                    This means that surface has changed and lightmap must be regenerated!",
                    );
                }
            }

            // Apply textures.
            for (&handle, entries) in lightmap.map.iter_mut() {
                if let Some(mesh) = self.graph[handle].cast_mut::<Mesh>() {
                    for (entry, surface) in entries.iter_mut().zip(mesh.surfaces_mut()) {
                        if let Err(e) = surface.material().lock().set_property(
                            &ImmutableString::new("lightmapTexture"),
                            PropertyValue::Sampler {
                                value: entry.texture.clone(),
                                fallback: SamplerFallback::Black,
                            },
                        ) {
                            Log::writeln(
                                MessageKind::Error,
                                format!(
                                    "Failed to apply light map texture to material. Reason {:?}",
                                    e
                                ),
                            )
                        }
                    }
                }
            }
        }

        Log::writeln(MessageKind::Information, "Resolve succeeded!");
    }

    /// Tries to set new lightmap to scene.
    pub fn set_lightmap(&mut self, lightmap: Lightmap) -> Result<Option<Lightmap>, &'static str> {
        // Assign textures to surfaces.
        for (handle, lightmaps) in lightmap.map.iter() {
            if let Some(mesh) = self.graph[*handle].cast_mut::<Mesh>() {
                if mesh.surfaces().len() != lightmaps.len() {
                    return Err("failed to set lightmap, surface count mismatch");
                }

                for (surface, entry) in mesh.surfaces_mut().iter_mut().zip(lightmaps) {
                    // This unwrap() call must never panic in normal conditions, because texture wrapped in Option
                    // only to implement Default trait to be serializable.
                    let texture = entry.texture.clone().unwrap();
                    if let Err(e) = surface.material().lock().set_property(
                        &ImmutableString::new("lightmapTexture"),
                        PropertyValue::Sampler {
                            value: Some(texture),
                            fallback: SamplerFallback::Black,
                        },
                    ) {
                        Log::writeln(
                            MessageKind::Error,
                            format!(
                                "Failed to apply light map texture to material. Reason {:?}",
                                e
                            ),
                        )
                    }
                }
            }
        }
        Ok(std::mem::replace(&mut self.lightmap, Some(lightmap)))
    }

    /// Performs single update tick with given delta time from last frame. Internally
    /// it updates physics, animations, and each graph node. In most cases there is
    /// no need to call it directly, engine automatically updates all available scenes.
    pub fn update(&mut self, frame_size: Vector2<f32>, dt: f32) {
        let last = instant::Instant::now();
        self.animations.update_animations(dt);
        self.performance_statistics.animations_update_time = instant::Instant::now() - last;

        self.graph.update(frame_size, dt);
        self.performance_statistics.graph = self.graph.performance_statistics.clone();

        for machine in self.animation_machines.iter_mut() {
            machine
                .evaluate_pose(&self.animations, dt)
                .apply(&mut self.graph);
        }
    }

    /// Creates deep copy of a scene, filter predicate allows you to filter out nodes
    /// by your criteria.
    pub fn clone<F>(&self, filter: &mut F) -> (Self, NodeHandleMap)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        let (graph, old_new_map) = self.graph.clone(filter);
        let mut animations = self.animations.clone();
        for animation in animations.iter_mut() {
            // Remove all tracks for nodes that were filtered out.
            animation.retain_tracks(|track| old_new_map.map.contains_key(&track.get_node()));
            // Remap track nodes.
            for track in animation.get_tracks_mut() {
                track.set_node(old_new_map.map[&track.get_node()]);
            }
        }

        let mut animation_machines = self.animation_machines.clone();
        for machine in animation_machines.iter_mut() {
            machine.root = old_new_map
                .map
                .get(&machine.root)
                .cloned()
                .unwrap_or_default();
        }

        (
            Self {
                graph,
                animations,
                animation_machines,
                // Render target is intentionally not copied, because it does not makes sense - a copy
                // will redraw frame completely.
                render_target: Default::default(),
                lightmap: self.lightmap.clone(),
                drawing_context: self.drawing_context.clone(),
                navmeshes: self.navmeshes.clone(),
                performance_statistics: Default::default(),
                ambient_lighting_color: self.ambient_lighting_color,
                enabled: self.enabled,
            },
            old_new_map,
        )
    }

    fn visit(&mut self, region_name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(region_name)?;

        self.graph.visit("Graph", &mut region)?;
        self.animations.visit("Animations", &mut region)?;
        self.lightmap.visit("Lightmap", &mut region)?;
        self.navmeshes.visit("NavMeshes", &mut region)?;
        self.ambient_lighting_color
            .visit("AmbientLightingColor", &mut region)?;
        self.enabled.visit("Enabled", &mut region)?;
        let _ = self
            .animation_machines
            .visit("AnimationMachines", &mut region);

        Ok(())
    }

    /// Saves scene in a specified file.
    pub fn save(&mut self, region_name: &str, visitor: &mut Visitor) -> VisitResult {
        if visitor.is_reading() {
            return Err(VisitError::User(
                "Visitor must be in write mode!".to_string(),
            ));
        }

        self.visit(region_name, visitor)
    }
}

/// Container for scenes in the engine.
pub struct SceneContainer {
    pool: Pool<Scene>,
    sound_engine: Arc<Mutex<SoundEngine>>,
    pub(crate) destruction_list: Vec<(Handle<Scene>, Scene)>,
}

impl SceneContainer {
    pub(crate) fn new(sound_engine: Arc<Mutex<SoundEngine>>) -> Self {
        Self {
            pool: Pool::new(),
            sound_engine,
            destruction_list: Default::default(),
        }
    }

    /// Return true if given handle is valid and "points" to "alive" scene.
    pub fn is_valid_handle(&self, handle: Handle<Scene>) -> bool {
        self.pool.is_valid_handle(handle)
    }

    /// Returns pair iterator which yields (handle, scene_ref) pairs.
    pub fn pair_iter(&self) -> impl Iterator<Item = (Handle<Scene>, &Scene)> {
        self.pool.pair_iter()
    }

    /// Returns pair iterator which yields (handle, scene_ref) pairs.
    pub fn pair_iter_mut(&mut self) -> impl Iterator<Item = (Handle<Scene>, &mut Scene)> {
        self.pool.pair_iter_mut()
    }

    /// Tries to borrow a scene using its handle.
    pub fn try_get(&self, handle: Handle<Scene>) -> Option<&Scene> {
        self.pool.try_borrow(handle)
    }

    /// Tries to borrow a scene using its handle.
    pub fn try_get_mut(&mut self, handle: Handle<Scene>) -> Option<&mut Scene> {
        self.pool.try_borrow_mut(handle)
    }

    /// Creates new iterator over scenes in container.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Scene> {
        self.pool.iter()
    }

    /// Creates new mutable iterator over scenes in container.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Scene> {
        self.pool.iter_mut()
    }

    /// Adds new scene into container.
    #[inline]
    pub fn add(&mut self, scene: Scene) -> Handle<Scene> {
        self.sound_engine
            .lock()
            .unwrap()
            .add_context(scene.graph.sound_context.native.clone());
        self.pool.spawn(scene)
    }

    /// Removes all scenes from container.
    #[inline]
    pub fn clear(&mut self) {
        self.pool.clear()
    }

    /// Removes given scene from container. The scene will be destroyed on a next update call.
    #[inline]
    pub fn remove(&mut self, handle: Handle<Scene>) {
        self.sound_engine
            .lock()
            .unwrap()
            .remove_context(self.pool[handle].graph.sound_context.native.clone());
        self.destruction_list.push((handle, self.pool.free(handle)));
    }

    /// Takes scene from the container and transfers ownership to caller. You must either
    /// put scene back using ticket or call `forget_ticket` to make memory used by scene
    /// vacant again.
    pub fn take_reserve(&mut self, handle: Handle<Scene>) -> (Ticket<Scene>, Scene) {
        self.pool.take_reserve(handle)
    }

    /// Puts scene back using its ticket.
    pub fn put_back(&mut self, ticket: Ticket<Scene>, scene: Scene) -> Handle<Scene> {
        self.pool.put_back(ticket, scene)
    }

    /// Forgets ticket of a scene, making place at which ticket points, vacant again.
    pub fn forget_ticket(&mut self, ticket: Ticket<Scene>) {
        self.pool.forget_ticket(ticket)
    }
}

impl Index<Handle<Scene>> for SceneContainer {
    type Output = Scene;

    #[inline]
    fn index(&self, index: Handle<Scene>) -> &Self::Output {
        &self.pool[index]
    }
}

impl IndexMut<Handle<Scene>> for SceneContainer {
    #[inline]
    fn index_mut(&mut self, index: Handle<Scene>) -> &mut Self::Output {
        &mut self.pool[index]
    }
}
