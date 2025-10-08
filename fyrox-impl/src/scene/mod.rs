// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#![warn(missing_docs)]

//! Contains all structures and methods to create and manage 3D scenes.
//!
//! A `Scene` is a container for graph nodes, animations and physics.

pub mod accel;
pub mod animation;
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
pub mod navmesh;
pub mod node;
pub mod particle_system;
pub mod pivot;
pub mod probe;
pub mod ragdoll;
pub mod rigidbody;
pub mod skybox;
pub mod sound;
pub mod sprite;
pub mod terrain;
pub mod tilemap;
pub mod transform;

use crate::{
    asset::{
        self, io::ResourceIo, manager::ResourceManager, registry::ResourceRegistryStatus,
        untyped::UntypedResource,
    },
    core::{
        algebra::Vector2,
        color::Color,
        futures::future::join_all,
        log::{Log, MessageKind},
        pool::{Handle, Pool, Ticket},
        reflect::prelude::*,
        type_traits::prelude::*,
        variable::InheritableVariable,
        visitor::{error::VisitError, Visit, VisitResult, Visitor},
    },
    engine::SerializationContext,
    graph::NodeHandleMap,
    graphics::PolygonFillMode,
    resource::texture::TextureResource,
    scene::{
        base::BaseBuilder,
        debug::SceneDrawingContext,
        graph::{Graph, GraphPerformanceStatistics, GraphUpdateSwitches},
        navmesh::NavigationalMeshBuilder,
        node::Node,
        skybox::{SkyBox, SkyBoxKind},
        sound::SoundEngine,
    },
    utils::navmesh::Navmesh,
};
use fxhash::FxHashSet;
use fyrox_core::SafeLock;
use std::{
    fmt::{Display, Formatter},
    ops::{Index, IndexMut},
    path::Path,
    path::PathBuf,
    sync::Arc,
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

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

/// A set of options, that allows selecting the source of environment lighting for a scene. By
/// default, it is set to [`EnvironmentLightingSource::SkyBox`].
#[derive(
    Reflect,
    Visit,
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    AsRefStr,
    EnumString,
    VariantNames,
    TypeUuidProvider,
)]
#[type_uuid(id = "28f22fe7-22ed-47e1-ae43-779866a46cdf")]
pub enum EnvironmentLightingSource {
    /// Sky box of a scene will be the source of lighting.
    #[default]
    SkyBox,
    /// Ambient color of the scene will be the source of lighting.
    AmbientColor,
}

/// Rendering options of a scene. It allows you to specify a render target to render the scene to, change its clear color, etc.
#[derive(Debug, Visit, Reflect, PartialEq)]
pub struct SceneRenderingOptions {
    /// A texture to draw the scene to. If empty, then the scene will be drawn on screen directly. It is useful to "embed" some scene into other
    /// by drawing a quad with this texture. This can be used to make in-game video conference - you can make separate scene with
    /// your characters and draw scene into a texture, then in the main scene you can attach this texture to some quad which will be used
    /// as a monitor. Other usage could be a previewer of models, like a pictogram of character in real-time strategies, in other words
    /// there are plenty of possible uses.
    pub render_target: Option<TextureResource>,

    /// Default color of the render target. Default is [`None`], which forces the renderer to use clear color of the back buffer.
    /// Could be set to transparent to make background transparent.
    pub clear_color: Option<Color>,

    /// Defines how polygons of the scene will be rasterized. By default it set to [`PolygonFillMode::Fill`],
    /// [`PolygonFillMode::Line`] could be used to render the scene in wireframe mode.
    pub polygon_rasterization_mode: PolygonFillMode,

    /// Color of the ambient lighting. This color is only used if `environment_lighting_source`
    /// is set to [`EnvironmentLightingSource::AmbientColor`].
    pub ambient_lighting_color: Color,

    /// A switch, that allows selecting the source of environment lighting. By default, it is set to
    /// [`EnvironmentLightingSource::SkyBox`].
    pub environment_lighting_source: EnvironmentLightingSource,
}

impl Default for SceneRenderingOptions {
    fn default() -> Self {
        Self {
            render_target: None,
            clear_color: None,
            polygon_rasterization_mode: Default::default(),
            ambient_lighting_color: Color::opaque(100, 100, 100),
            environment_lighting_source: Default::default(),
        }
    }
}

impl Clone for SceneRenderingOptions {
    fn clone(&self) -> Self {
        Self {
            render_target: None, // Intentionally not copied!
            clear_color: self.clear_color,
            polygon_rasterization_mode: self.polygon_rasterization_mode,
            ambient_lighting_color: self.ambient_lighting_color,
            environment_lighting_source: self.environment_lighting_source,
        }
    }
}

/// See module docs.
#[derive(Debug, Reflect)]
pub struct Scene {
    /// Graph is main container for all scene nodes. It calculates global transforms for nodes,
    /// updates them and performs all other important work. See `graph` module docs for more
    /// info.
    pub graph: Graph,

    /// Rendering options of a scene. See [`SceneRenderingOptions`] docs for more info.
    pub rendering_options: InheritableVariable<SceneRenderingOptions>,

    /// Drawing context for simple graphics.
    #[reflect(hidden)]
    pub drawing_context: SceneDrawingContext,

    /// Performance statistics from last `update` call.
    #[reflect(hidden)]
    pub performance_statistics: PerformanceStatistics,

    #[reflect(setter = "set_skybox")]
    sky_box: InheritableVariable<Option<SkyBox>>,

    /// Whether the scene will be updated and rendered or not. Default is true.
    /// This flag allowing you to build a scene manager for your game. For example,
    /// you may have a scene for menu and one per level. Menu's scene is persistent,
    /// however you don't want it to be updated and renderer while you have a level
    /// loaded and playing a game. When you're start playing, just set `enabled` flag
    /// to false for menu's scene and when you need to open a menu - set it to true and
    /// set `enabled` flag to false for level's scene.
    pub enabled: InheritableVariable<bool>,
}

impl Clone for Scene {
    fn clone(&self) -> Self {
        self.clone_one_to_one().0
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            graph: Default::default(),
            rendering_options: Default::default(),
            drawing_context: Default::default(),
            performance_statistics: Default::default(),
            enabled: true.into(),
            sky_box: Some(SkyBoxKind::built_in_skybox().clone()).into(),
        }
    }
}

/// A structure that holds times that specific update step took.
#[derive(Clone, Default, Debug)]
pub struct PerformanceStatistics {
    /// Graph performance statistics.
    pub graph: GraphPerformanceStatistics,
}

impl Display for PerformanceStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Graph: {:?}\n\
            \tSync Time: {:?}\n\
            \tSound: {:?}\n\
            \tPhysics: {:?}\n\
            \t\tSimulation: {:?}\n\
            \t\tRay cast: {:?}\n\
            \tPhysics 2D: {:?}\n\
            \t\tSimulation: {:?}\n\
            \t\tRay cast: {:?}\n\
            \tHierarchy: {:?}",
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
    path: Option<PathBuf>,
    resource_manager: ResourceManager,
}

impl SceneLoader {
    /// Tries to load scene from given file. File can contain any scene in native engine format.
    /// Such scenes can be made in rusty editor.
    pub async fn from_file<P: AsRef<Path>>(
        path: P,
        io: &dyn ResourceIo,
        serialization_context: Arc<SerializationContext>,
        resource_manager: ResourceManager,
    ) -> Result<(Self, Vec<u8>), VisitError> {
        let registry_status = resource_manager
            .state()
            .resource_registry
            .safe_lock()
            .status_flag();
        // Wait until the registry is fully loaded.
        let registry_status = registry_status.await;
        if registry_status == ResourceRegistryStatus::Unknown {
            return Err(VisitError::User(format!(
                "Unable to load a scene from {} path, because the \
            resource registry isn't loaded!",
                path.as_ref().display()
            )));
        }

        let data = io.load_file(path.as_ref()).await?;
        let mut visitor = Visitor::load_from_memory(&data)?;
        let loader = Self::load(
            "Scene",
            serialization_context,
            resource_manager,
            &mut visitor,
            Some(path.as_ref().to_path_buf()),
        )?;
        Ok((loader, data))
    }

    /// Tries to load a scene using specified visitor and region name.
    pub fn load(
        region_name: &str,
        serialization_context: Arc<SerializationContext>,
        resource_manager: ResourceManager,
        visitor: &mut Visitor,
        path: Option<PathBuf>,
    ) -> Result<Self, VisitError> {
        if !visitor.is_reading() {
            return Err(VisitError::User(
                "Visitor must be in read mode!".to_string(),
            ));
        }

        visitor.blackboard.register(serialization_context);
        visitor
            .blackboard
            .register(Arc::new(resource_manager.clone()));

        let mut scene = Scene::default();
        scene.visit(region_name, visitor)?;

        Ok(Self {
            scene,
            path,
            resource_manager,
        })
    }

    /// Finishes scene loading.
    pub async fn finish(self) -> Scene {
        let mut scene = self.scene;

        Log::info("SceneLoader::finish() - Collecting resources used by the scene...");

        let mut used_resources = scene.collect_used_resources();

        // Do not wait for self resources.
        if let Some(path) = self.path {
            let exclusion_list = used_resources
                .iter()
                .filter(|res| {
                    let uuid = res.resource_uuid();
                    let state = self.resource_manager.state();
                    let registry = state.resource_registry.lock();
                    uuid.and_then(|uuid| registry.uuid_to_path(uuid)) == Some(&path)
                })
                .cloned()
                .collect::<Vec<_>>();

            for excluded_resource in exclusion_list {
                assert!(used_resources.remove(&excluded_resource));
            }
        }

        let used_resources_count = used_resources.len();

        Log::info(format!(
            "SceneLoader::finish() - {used_resources_count} resources collected. Waiting them to load..."
        ));

        // Wait everything.
        let results = join_all(used_resources.into_iter()).await;

        for result in results {
            if let Err(err) = result {
                Log::err(format!("Scene resource loading error: {:?}", err));
            }
        }

        Log::info(format!(
            "SceneLoader::finish() - All {used_resources_count} resources have finished loading."
        ));

        // We have to wait until skybox textures are all loaded, because we need to read their data
        // to re-create cube map.
        let mut skybox_textures = Vec::new();
        if let Some(skybox) = scene.skybox_ref() {
            skybox_textures.extend(skybox.textures().iter().filter_map(|t| t.clone()));
        }
        join_all(skybox_textures).await;

        // And do resolve to extract correct graphical data and so on.
        scene.resolve();

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
            rendering_options: Default::default(),
            drawing_context: Default::default(),
            performance_statistics: Default::default(),
            enabled: true.into(),
            sky_box: Some(SkyBoxKind::built_in_skybox().clone()).into(),
        }
    }

    /// Sets new skybox. Could be None if no skybox needed.
    pub fn set_skybox(&mut self, skybox: Option<SkyBox>) -> Option<SkyBox> {
        self.sky_box.set_value_and_mark_modified(skybox)
    }

    /// Return optional mutable reference to current skybox.
    pub fn skybox_mut(&mut self) -> Option<&mut SkyBox> {
        self.sky_box.get_value_mut_and_mark_modified().as_mut()
    }

    /// Return optional shared reference to current skybox.
    pub fn skybox_ref(&self) -> Option<&SkyBox> {
        self.sky_box.as_ref()
    }

    /// Replaces the skybox.
    pub fn replace_skybox(&mut self, new: Option<SkyBox>) -> Option<SkyBox> {
        std::mem::replace(self.sky_box.get_value_mut_and_mark_modified(), new)
    }

    /// Synchronizes the state of the scene with external resources.
    pub fn resolve(&mut self) {
        Log::writeln(MessageKind::Information, "Starting resolve...");

        // Update cube maps for sky boxes.
        if let Some(skybox) = self.skybox_mut() {
            Log::verify(skybox.create_cubemap());
        }

        self.graph.resolve();

        Log::writeln(MessageKind::Information, "Resolve succeeded!");
    }

    /// Collects all resources used by the scene. It uses reflection to "scan" the contents of the scene, so
    /// if some fields marked with `#[reflect(hidden)]` attribute, then such field will be ignored!
    pub fn collect_used_resources(&self) -> FxHashSet<UntypedResource> {
        let mut collection = FxHashSet::default();
        asset::collect_used_resources(self, &mut collection);
        collection
    }

    /// Performs single update tick with given delta time from last frame. Internally
    /// it updates physics, animations, and each graph node. In most cases there is
    /// no need to call it directly, engine automatically updates all available scenes.
    pub fn update(&mut self, frame_size: Vector2<f32>, dt: f32, switches: GraphUpdateSwitches) {
        self.graph.update(frame_size, dt, switches);
        self.performance_statistics.graph = self.graph.performance_statistics.clone();
    }

    /// Creates deep copy of a scene, filter predicate allows you to filter out nodes
    /// by your criteria.
    pub fn clone_ex<F, Pre, Post>(
        &self,
        root: Handle<Node>,
        filter: &mut F,
        pre_process_callback: &mut Pre,
        post_process_callback: &mut Post,
    ) -> (Self, NodeHandleMap<Node>)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
        Pre: FnMut(Handle<Node>, &mut Node),
        Post: FnMut(Handle<Node>, Handle<Node>, &mut Node),
    {
        let (graph, old_new_map) =
            self.graph
                .clone_ex(root, filter, pre_process_callback, post_process_callback);

        (
            Self {
                graph,
                rendering_options: self.rendering_options.clone(),
                drawing_context: self.drawing_context.clone(),
                performance_statistics: Default::default(),
                enabled: self.enabled.clone(),
                sky_box: self.sky_box.clone(),
            },
            old_new_map,
        )
    }

    /// Creates deep copy of a scene. Same as [`Self::clone`], but does 1:1 cloning.
    pub fn clone_one_to_one(&self) -> (Self, NodeHandleMap<Node>) {
        self.clone_ex(
            self.graph.get_root(),
            &mut |_, _| true,
            &mut |_, _| {},
            &mut |_, _, _| {},
        )
    }

    fn visit(&mut self, region_name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(region_name)?;

        self.graph.visit("Graph", &mut region)?;

        self.enabled.visit("Enabled", &mut region)?;
        let _ = self
            .rendering_options
            .visit("RenderingOptions", &mut region);
        let _ = self.sky_box.visit("SkyBox", &mut region);

        // Backward compatibility.
        let mut navmeshes = NavMeshContainer::default();
        if navmeshes.visit("NavMeshes", &mut region).is_ok() {
            for (i, navmesh) in navmeshes.iter().enumerate() {
                NavigationalMeshBuilder::new(BaseBuilder::new().with_name(format!("Navmesh{i}")))
                    .with_navmesh(navmesh.clone())
                    .build(&mut self.graph);
            }
        }

        Ok(())
    }

    /// Tries to serialize the scene using the specified serializer. The serializer must be in write mode, otherwise
    /// serialization will fail. The `region_name` argument must be `Scene` (scene loader expects this value, you can
    /// use any other if you don't plan to load scenes using the standard mechanism).. Keep in mind, that this method
    /// does **not** write anything to a file, instead it just fills in the serializer.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// # use fyrox_impl::{
    /// #     core::visitor::Visitor,
    /// #     scene::{
    /// #         base::BaseBuilder,
    /// #         mesh::{
    /// #             surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
    /// #             MeshBuilder,
    /// #         },
    /// #         Scene,
    /// #     },
    /// # };
    /// use fyrox_resource::untyped::ResourceKind;
    /// #
    /// // Create a scene.
    /// let mut scene = Scene::new();
    ///
    /// MeshBuilder::new(BaseBuilder::new())
    ///     .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_embedded(SurfaceData::make_cube(Default::default()),
    ///     ))
    ///     .build()])
    ///     .build(&mut scene.graph);
    ///
    /// // Serialize the content.
    /// let mut visitor = Visitor::new();
    /// scene.save("Scene", &mut visitor).unwrap();
    ///
    /// // Write the data to a file.
    /// visitor.save_binary_to_file("path/to/a/scene.rgs").unwrap();
    /// ```
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
    sound_engine: SoundEngine,
    pub(crate) destruction_list: Vec<(Handle<Scene>, Scene)>,
}

impl SceneContainer {
    pub(crate) fn new(sound_engine: SoundEngine) -> Self {
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
            .state()
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
            .state()
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
