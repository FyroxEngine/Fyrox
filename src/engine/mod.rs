//! Engine is container for all subsystems (renderer, ui, sound, resource manager). It also
//! creates a window and an OpenGL context.

#![warn(missing_docs)]

pub mod error;
pub mod executor;
pub mod task;

use crate::{
    asset::{
        event::ResourceEvent,
        manager::{ResourceManager, ResourceWaitContext},
    },
    core::{
        algebra::Vector2, futures::executor::block_on, instant, log::Log, pool::Handle,
        reflect::Reflect, variable::try_inherit_properties, visitor::VisitError,
    },
    engine::error::EngineError,
    event::Event,
    gui::UserInterface,
    material::{
        loader::MaterialLoader,
        shader::{loader::ShaderLoader, Shader, ShaderResource, ShaderResourceExtension},
        Material,
    },
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    renderer::{framework::error::FrameworkError, framework::state::GlKind, Renderer},
    resource::{
        curve::{loader::CurveLoader, CurveResourceState},
        model::{loader::ModelLoader, Model, ModelResource, NodeMapping},
        texture::{loader::TextureLoader, Texture, TextureKind},
    },
    scene::{
        base::NodeScriptMessage,
        camera::SkyBoxKind,
        graph::{GraphUpdateSwitches, NodePool},
        node::{constructor::NodeConstructorContainer, Node},
        sound::SoundEngine,
        Scene, SceneContainer, SceneLoader,
    },
    script::{
        constructor::ScriptConstructorContainer, RoutingStrategy, Script, ScriptContext,
        ScriptDeinitContext, ScriptMessage, ScriptMessageContext, ScriptMessageKind,
        ScriptMessageSender,
    },
    window::{Window, WindowBuilder},
};
use fxhash::{FxHashMap, FxHashSet};
use fyrox_resource::untyped::{ResourceKind, UntypedResource};
use fyrox_sound::{
    buffer::{loader::SoundBufferLoader, SoundBuffer},
    renderer::hrtf::{HrirSphereLoader, HrirSphereResourceData},
};
#[cfg(not(target_arch = "wasm32"))]
use glutin::{
    config::ConfigTemplateBuilder,
    context::{
        ContextApi, ContextAttributesBuilder, GlProfile, NotCurrentGlContext,
        PossiblyCurrentContext, Version,
    },
    display::{GetGlDisplay, GlDisplay},
    surface::{GlSurface, Surface, SwapInterval, WindowSurface},
};
#[cfg(not(target_arch = "wasm32"))]
use glutin_winit::{DisplayBuilder, GlWindow};
#[cfg(not(target_arch = "wasm32"))]
use raw_window_handle::HasRawWindowHandle;

#[cfg(not(target_arch = "wasm32"))]
use std::{ffi::CString, num::NonZeroU32};

use crate::engine::task::TaskPoolHandler;
use fyrox_core::task::TaskPool;
use fyrox_ui::font::BUILT_IN_FONT;
use fyrox_ui::loader::UserInterfaceLoader;
use fyrox_ui::{font::loader::FontLoader, font::Font};
use std::ops::DerefMut;
use std::{
    any::TypeId,
    collections::{HashSet, VecDeque},
    fmt::{Display, Formatter},
    ops::Deref,
    path::{Path, PathBuf},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    time::Duration,
};
use winit::{
    dpi::{Position, Size},
    event_loop::EventLoopWindowTarget,
    window::WindowAttributes,
};

/// Serialization context holds runtime type information that allows to create unknown types using
/// their UUIDs and a respective constructors.
pub struct SerializationContext {
    /// A node constructor container.
    pub node_constructors: NodeConstructorContainer,
    /// A script constructor container.
    pub script_constructors: ScriptConstructorContainer,
}

impl Default for SerializationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl SerializationContext {
    /// Creates default serialization context.
    pub fn new() -> Self {
        Self {
            node_constructors: NodeConstructorContainer::new(),
            script_constructors: ScriptConstructorContainer::new(),
        }
    }
}

/// Performance statistics.
#[derive(Debug, Default)]
pub struct PerformanceStatistics {
    /// Amount of time spent in the UI system.
    pub ui_time: Duration,

    /// Amount of time spent in updating/initializing/destructing scripts of all scenes.
    pub scripts_time: Duration,

    /// Amount of time spent in plugins updating.
    pub plugins_time: Duration,
}

impl Display for PerformanceStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Performance Statistics:\n\tUI: {:?}\n\tScripts: {:?}\n\tPlugins: {:?}",
            self.ui_time, self.scripts_time, self.plugins_time
        )
    }
}

/// An initialized graphics context. It contains the main application window and the renderer instance.
pub struct InitializedGraphicsContext {
    /// Main application window.
    pub window: Window,

    /// Current renderer.
    pub renderer: Renderer,

    params: GraphicsContextParams,
    #[cfg(not(target_arch = "wasm32"))]
    gl_context: PossiblyCurrentContext,
    #[cfg(not(target_arch = "wasm32"))]
    gl_surface: Surface<WindowSurface>,
}

/// Graphics context of the engine, it could be in two main states:
///
/// - [`GraphicsContext::Initialized`] - active graphics context, that is fully initialized and ready for use.
/// - [`GraphicsContext::Uninitialized`] - suspended graphics context, that contains a set of params that could
/// be used for further initialization.
///
/// By default, when you creating an engine, there's no graphics context initialized. It must be initialized
/// manually (if you need it) on [`Event::Resumed`]. On most operating systems, it is possible to initialize
/// graphics context right after the engine was created. However Android won't allow you to do this, also on
/// some versions of macOS immediate initialization could lead to panic.
///
/// You can switch between these states whenever you need, for example if your application does not need a
/// window and a renderer at all you can just not create graphics context. This could be useful for game
/// servers or background applications. When you destroy a graphics context, the engine will remember the options
/// with which it was created and some of the main window parameters (position, size, etc.) and will re-use these
/// parameters on a next initialization attempt.
#[allow(clippy::large_enum_variant)]
pub enum GraphicsContext {
    /// Fully initialized graphics context. See [`InitializedGraphicsContext`] docs for more info.
    Initialized(InitializedGraphicsContext),

    /// Uninitialized (suspended) graphics context. See [`GraphicsContextParams`] docs for more info.
    Uninitialized(GraphicsContextParams),
}

impl GraphicsContext {
    /// Attempts to cast a graphics context to its initialized version. The method will panic if the context
    /// is not initialized.
    pub fn as_initialized_ref(&self) -> &InitializedGraphicsContext {
        if let GraphicsContext::Initialized(ctx) = self {
            ctx
        } else {
            panic!("Graphics context is uninitialized!")
        }
    }

    /// Attempts to cast a graphics context to its initialized version. The method will panic if the context
    /// is not initialized.
    pub fn as_initialized_mut(&mut self) -> &mut InitializedGraphicsContext {
        if let GraphicsContext::Initialized(ctx) = self {
            ctx
        } else {
            panic!("Graphics context is uninitialized!")
        }
    }
}

struct SceneLoadingOptions {
    derived: bool,
}

/// A helper that is used to load scenes asynchronously.
///
/// ## Examples
///
/// ```rust
/// use fyrox::{
///     core::{color::Color, log::Log, pool::Handle},
///     plugin::{Plugin, PluginConstructor, PluginContext},
///     scene::Scene,
/// };
/// use std::path::Path;
///
/// struct GameConstructor;
///
/// impl PluginConstructor for GameConstructor {
///     fn create_instance(
///         &self,
///         scene_path: Option<&str>,
///         context: PluginContext,
///     ) -> Box<dyn Plugin> {
///         Box::new(MyGame::new(scene_path, context))
///     }
/// }
///
/// struct MyGame {
///     scene: Handle<Scene>,
/// }
///
/// impl MyGame {
///     pub fn new(scene_path: Option<&str>, context: PluginContext) -> Self {
///         context
///             .async_scene_loader
///             .request(scene_path.unwrap_or("data/scene.rgs"));
///
///         Self {
///             scene: Handle::NONE,
///         }
///     }
/// }
///
/// impl Plugin for MyGame {
///     fn on_scene_begin_loading(&mut self, path: &Path, _context: &mut PluginContext) {
///         Log::info(format!("{} scene has started loading.", path.display()));
///
///         // Use this method if you need to so something when a scene started loading.
///     }
///
///     fn on_scene_loaded(
///         &mut self,
///         path: &Path,
///         scene: Handle<Scene>,
///         data: &[u8],
///         context: &mut PluginContext,
///     ) {
///         // Optionally remove previous scene.
///         if self.scene.is_some() {
///             context.scenes.remove(self.scene);
///         }
///
///         // Remember new scene handle.
///         self.scene = scene;
///
///         Log::info(format!("{} scene was loaded!", path.display()));
///
///         // Do something with a newly loaded scene.
///         let scene_ref = &mut context.scenes[scene];
///
///         scene_ref.rendering_options.ambient_lighting_color = Color::opaque(20, 20, 20);
///     }
/// }
/// ```
///
/// This example shows a typical usage of the loader, an instance of which is available in the
/// plugin context. `Game::new` requests a new scene, which internally asks a resource manager to
/// load the scene. Then, when the scene is fully loaded, the engine calls `Plugin::on_scene_loaded`
/// method which allows you to do something with the newly loaded scene by taking a reference of it.
pub struct AsyncSceneLoader {
    resource_manager: ResourceManager,
    serialization_context: Arc<SerializationContext>,
    receiver: Receiver<SceneLoadingResult>,
    sender: Sender<SceneLoadingResult>,
    loading_scenes: FxHashMap<PathBuf, LoadingScene>,
}

struct LoadingScene {
    reported: bool,
    path: PathBuf,
    options: SceneLoadingOptions,
}

struct SceneLoadingResult {
    path: PathBuf,
    result: Result<(Scene, Vec<u8>), VisitError>,
}

impl AsyncSceneLoader {
    fn new(
        resource_manager: ResourceManager,
        serialization_context: Arc<SerializationContext>,
    ) -> Self {
        let (sender, receiver) = channel();
        Self {
            resource_manager,
            serialization_context,
            receiver,
            sender,
            loading_scenes: Default::default(),
        }
    }

    fn request_with_options<P: AsRef<Path>>(&mut self, path: P, opts: SceneLoadingOptions) {
        let path = path.as_ref().to_path_buf();

        if self.loading_scenes.contains_key(&path) {
            Log::warn(format!("A scene {} is already loading!", path.display()))
        } else {
            // Register a new request.
            self.loading_scenes.insert(
                path.clone(),
                LoadingScene {
                    reported: false,
                    path: path.clone(),
                    options: opts,
                },
            );

            // Start loading in a separate off-thread task.
            let sender = self.sender.clone();
            let serialization_context = self.serialization_context.clone();
            let resource_manager = self.resource_manager.clone();

            // Aquire the resource IO from the resource manager
            let io = resource_manager.resource_io();

            let future = async move {
                match SceneLoader::from_file(
                    path.clone(),
                    io.as_ref(),
                    serialization_context,
                    resource_manager.clone(),
                )
                .await
                {
                    Ok((loader, data)) => {
                        let scene = loader.finish(&resource_manager).await;
                        Log::verify(sender.send(SceneLoadingResult {
                            path,
                            result: Ok((scene, data)),
                        }));
                    }
                    Err(e) => {
                        Log::verify(sender.send(SceneLoadingResult {
                            path,
                            result: Err(e),
                        }));
                    }
                }
            };

            #[cfg(not(target_arch = "wasm32"))]
            {
                std::thread::spawn(move || block_on(future));
            }

            #[cfg(target_arch = "wasm32")]
            {
                crate::core::wasm_bindgen_futures::spawn_local(future);
            }
        }
    }

    /// Requests a scene for loading as derived scene. See [`AsyncSceneLoader`] for usage example.
    ///
    /// ## Raw vs Derived Scene
    ///
    /// Derived scene means its nodes will derive their properties from the nodes from the source
    /// scene. Derived scene is useful for saved games - you can serialize your scene as usual and
    /// it will only contain a "difference" between the original scene and yours. To load the same
    /// scene as raw scene use [`Self::request_raw`] method.
    ///
    /// Raw scene, on other hand, loads the scene as-is without any additional markings for the
    /// scene nodes. It could be useful to load saved games.
    pub fn request<P: AsRef<Path>>(&mut self, path: P) {
        self.request_with_options(path, SceneLoadingOptions { derived: true });
    }

    /// Requests a scene for loading in raw mode. See [`Self::request`] docs for more info.
    pub fn request_raw<P: AsRef<Path>>(&mut self, path: P) {
        self.request_with_options(path, SceneLoadingOptions { derived: false });
    }
}

/// See module docs.
pub struct Engine {
    /// Graphics context of the engine. See [`GraphicsContext`] docs for more info.
    pub graphics_context: GraphicsContext,

    /// User interface allows you to build interface of any kind.
    pub user_interface: UserInterface,

    /// Current resource manager. Resource manager can be cloned (it does clone only ref) to be able to
    /// use resource manager from any thread, this is useful to load resources from multiple
    /// threads to decrease loading times of your game by utilizing all available power of
    /// your CPU.
    pub resource_manager: ResourceManager,

    /// All available scenes in the engine.
    pub scenes: SceneContainer,

    /// An instance of the async scene loader. See [`AsyncSceneLoader`] docs for usage example.
    pub async_scene_loader: AsyncSceneLoader,

    /// Task pool for asynchronous task management.
    pub task_pool: TaskPoolHandler,

    performance_statistics: PerformanceStatistics,

    model_events_receiver: Receiver<ResourceEvent>,

    #[allow(dead_code)] // Keep engine instance alive.
    sound_engine: SoundEngine,

    // A set of plugin constructors.
    plugin_constructors: Vec<Box<dyn PluginConstructor>>,

    // A set of plugins used by the engine.
    plugins: Vec<Box<dyn Plugin>>,

    plugins_enabled: bool,

    // Amount of time (in seconds) that passed from creation of the engine.
    elapsed_time: f32,

    /// A special container that is able to create nodes by their type UUID. Use a copy of this
    /// value whenever you need it as a parameter in other parts of the engine.
    pub serialization_context: Arc<SerializationContext>,

    /// Script processor is used to run script methods in a strict order.
    pub script_processor: ScriptProcessor,
}

/// Performs dispatch of script messages.
pub struct ScriptMessageDispatcher {
    type_groups: FxHashMap<TypeId, FxHashSet<Handle<Node>>>,
    message_receiver: Receiver<ScriptMessage>,
}

impl ScriptMessageDispatcher {
    fn new(message_receiver: Receiver<ScriptMessage>) -> Self {
        Self {
            type_groups: Default::default(),
            message_receiver,
        }
    }

    /// Subscribes a node to receive any message of the given type `T`. Subscription is automatically removed
    /// if the node dies.
    pub fn subscribe_to<T: 'static>(&mut self, receiver: Handle<Node>) {
        self.type_groups
            .entry(TypeId::of::<T>())
            .and_modify(|v| {
                v.insert(receiver);
            })
            .or_insert_with(|| FxHashSet::from_iter([receiver]));
    }

    /// Unsubscribes a node from receiving any messages of the given type `T`.
    pub fn unsubscribe_from<T: 'static>(&mut self, receiver: Handle<Node>) {
        if let Some(group) = self.type_groups.get_mut(&TypeId::of::<T>()) {
            group.remove(&receiver);
        }
    }

    /// Unsubscribes a node from receiving any messages.
    pub fn unsubscribe(&mut self, receiver: Handle<Node>) {
        for group in self.type_groups.values_mut() {
            group.remove(&receiver);
        }
    }

    fn dispatch_messages(
        &self,
        scene: &mut Scene,
        plugins: &mut Vec<Box<dyn Plugin>>,
        resource_manager: &ResourceManager,
        dt: f32,
        elapsed_time: f32,
        message_sender: &ScriptMessageSender,
    ) {
        while let Ok(message) = self.message_receiver.try_recv() {
            let mut payload = message.payload;
            if let Some(receivers) = self.type_groups.get(&payload.deref().type_id()) {
                match message.kind {
                    ScriptMessageKind::Targeted(target) => {
                        if receivers.contains(&target) {
                            let mut context = ScriptMessageContext {
                                dt,
                                elapsed_time,
                                plugins,
                                handle: target,
                                scene,
                                resource_manager,
                                message_sender,
                            };

                            process_node_message(&mut context, &mut |s, ctx| {
                                s.on_message(&mut *payload, ctx)
                            })
                        }
                    }
                    ScriptMessageKind::Hierarchical { root, routing } => match routing {
                        RoutingStrategy::Up => {
                            let mut node = root;
                            while let Some(node_ref) = scene.graph.try_get(node) {
                                let parent = node_ref.parent();

                                let mut context = ScriptMessageContext {
                                    dt,
                                    elapsed_time,
                                    plugins,
                                    handle: node,
                                    scene,
                                    resource_manager,
                                    message_sender,
                                };

                                if receivers.contains(&node) {
                                    process_node_message(&mut context, &mut |s, ctx| {
                                        s.on_message(&mut *payload, ctx)
                                    });
                                }

                                node = parent;
                            }
                        }
                        RoutingStrategy::Down => {
                            for node in scene.graph.traverse_handle_iter(root).collect::<Vec<_>>() {
                                let mut context = ScriptMessageContext {
                                    dt,
                                    elapsed_time,
                                    plugins,
                                    handle: node,
                                    scene,
                                    resource_manager,
                                    message_sender,
                                };

                                if receivers.contains(&node) {
                                    process_node_message(&mut context, &mut |s, ctx| {
                                        s.on_message(&mut *payload, ctx)
                                    });
                                }
                            }
                        }
                    },
                    ScriptMessageKind::Global => {
                        for &node in receivers {
                            let mut context = ScriptMessageContext {
                                dt,
                                elapsed_time,
                                plugins,
                                handle: node,
                                scene,
                                resource_manager,
                                message_sender,
                            };

                            process_node_message(&mut context, &mut |s, ctx| {
                                s.on_message(&mut *payload, ctx)
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Scripted scene is a handle to scene with some additional data associated with it.
pub struct ScriptedScene {
    /// Handle of a scene.
    pub handle: Handle<Scene>,
    /// Script message sender.
    pub message_sender: ScriptMessageSender,
    message_dispatcher: ScriptMessageDispatcher,
}

/// Script processor is used to run script methods in a strict order.
#[derive(Default)]
pub struct ScriptProcessor {
    wait_list: Vec<ResourceWaitContext>,
    /// A list of scenes.
    pub scripted_scenes: Vec<ScriptedScene>,
}

impl ScriptProcessor {
    fn has_scripted_scene(&self, scene: Handle<Scene>) -> bool {
        self.scripted_scenes.iter().any(|s| s.handle == scene)
    }

    fn register_scripted_scene(
        &mut self,
        scene: Handle<Scene>,
        resource_manager: &ResourceManager,
    ) {
        // Ensure that the scene wasn't registered previously.
        assert!(!self.has_scripted_scene(scene));

        let (tx, rx) = channel();
        self.scripted_scenes.push(ScriptedScene {
            handle: scene,
            message_sender: ScriptMessageSender { sender: tx },
            message_dispatcher: ScriptMessageDispatcher::new(rx),
        });

        self.wait_list
            .push(resource_manager.state().get_wait_context());
    }

    fn handle_scripts(
        &mut self,
        scenes: &mut SceneContainer,
        plugins: &mut Vec<Box<dyn Plugin>>,
        resource_manager: &ResourceManager,
        task_pool: &mut TaskPoolHandler,
        graphics_context: &mut GraphicsContext,
        dt: f32,
        elapsed_time: f32,
    ) {
        self.wait_list
            .retain_mut(|context| !context.is_all_loaded());

        if !self.wait_list.is_empty() {
            return;
        }

        self.scripted_scenes
            .retain(|s| scenes.is_valid_handle(s.handle));

        'scene_loop: for scripted_scene in self.scripted_scenes.iter_mut() {
            let scene = &mut scenes[scripted_scene.handle];

            // Disabled scenes should not update their scripts.
            if !*scene.enabled {
                continue 'scene_loop;
            }

            // Fill in initial handles to nodes to initialize, start, update.
            let mut update_queue = VecDeque::new();
            let mut start_queue = VecDeque::new();
            for (handle, node) in scene.graph.pair_iter() {
                if let Some(script) = node.script.as_ref() {
                    if node.is_globally_enabled() {
                        if script.initialized {
                            if script.started {
                                update_queue.push_back(handle);
                            } else {
                                start_queue.push_back(handle);
                            }
                        } else {
                            scene
                                .graph
                                .script_message_sender
                                .send(NodeScriptMessage::InitializeScript { handle })
                                .unwrap();
                        }
                    }
                }
            }

            // We'll gather all scripts queued for destruction and destroy them all at once at the
            // end of the frame.
            let mut destruction_queue = VecDeque::new();

            let max_iterations = 64;

            'update_loop: for update_loop_iteration in 0..max_iterations {
                let mut context = ScriptContext {
                    dt,
                    elapsed_time,
                    plugins,
                    handle: Default::default(),
                    scene,
                    scene_handle: scripted_scene.handle,
                    resource_manager,
                    message_sender: &scripted_scene.message_sender,
                    message_dispatcher: &mut scripted_scene.message_dispatcher,
                    task_pool,
                    graphics_context,
                };

                'init_loop: for init_loop_iteration in 0..max_iterations {
                    // Process events first. `on_init` of a script can also create some other instances
                    // and these will be correctly initialized on current frame.
                    while let Ok(event) = context.scene.graph.script_message_receiver.try_recv() {
                        match event {
                            NodeScriptMessage::InitializeScript { handle } => {
                                context.handle = handle;

                                process_node(&mut context, &mut |script, context| {
                                    if !script.initialized {
                                        script.on_init(context);
                                        script.initialized = true;
                                    }

                                    // `on_start` must be called even if the script was initialized.
                                    start_queue.push_back(handle);
                                });
                            }
                            NodeScriptMessage::DestroyScript { handle, script } => {
                                // Destruction is delayed to the end of the frame.
                                destruction_queue.push_back((handle, script));
                            }
                        }
                    }

                    if start_queue.is_empty() {
                        // There is no more new nodes, we can safely leave the init loop.
                        break 'init_loop;
                    } else {
                        // Call `on_start` for every recently initialized node and go to next
                        // iteration of init loop. This is needed because `on_start` can spawn
                        // some other nodes that must be initialized before update.
                        while let Some(node) = start_queue.pop_front() {
                            context.handle = node;

                            process_node(&mut context, &mut |script, context| {
                                if !script.started {
                                    script.on_start(context);
                                    script.started = true;

                                    update_queue.push_back(node);
                                }
                            });
                        }
                    }

                    if init_loop_iteration == max_iterations - 1 {
                        Log::warn(
                            "Infinite init loop detected! Most likely some of \
                    your scripts causing infinite prefab instantiation!",
                        )
                    }
                }

                // Update all initialized and started scripts until there is something to initialize.
                if update_queue.is_empty() {
                    break 'update_loop;
                } else {
                    while let Some(handle) = update_queue.pop_front() {
                        context.handle = handle;

                        process_node(&mut context, &mut |script, context| {
                            script.on_update(context);
                        });
                    }

                    // Dispatch messages and go to the next iteration of update loop. This is needed, because
                    // `ScriptTrait::on_message` can spawn new scripts that must be correctly updated on this
                    // frame (to prevent one-frame lag).
                    scripted_scene.message_dispatcher.dispatch_messages(
                        scene,
                        plugins,
                        resource_manager,
                        dt,
                        elapsed_time,
                        &scripted_scene.message_sender,
                    );
                }

                if update_loop_iteration == max_iterations - 1 {
                    Log::warn(
                        "Infinite update loop detected! Most likely some of \
                    your scripts causing infinite prefab instantiation!",
                    )
                }
            }

            // As the last step, destroy queued scripts.
            let mut context = ScriptDeinitContext {
                elapsed_time,
                plugins,
                resource_manager,
                scene,
                node_handle: Default::default(),
                message_sender: &scripted_scene.message_sender,
            };
            while let Some((handle, mut script)) = destruction_queue.pop_front() {
                context.node_handle = handle;

                // Unregister self in message dispatcher.
                scripted_scene.message_dispatcher.unsubscribe(handle);

                // `on_deinit` could also spawn new nodes, but we won't take those into account on
                // this frame. They'll be correctly handled on next frame.
                script.on_deinit(&mut context);
            }
        }

        // Process scripts from destroyed scenes.
        for (handle, mut detached_scene) in scenes.destruction_list.drain(..) {
            if let Some(scripted_scene) = self.scripted_scenes.iter().find(|s| s.handle == handle) {
                let mut context = ScriptDeinitContext {
                    elapsed_time,
                    plugins,
                    resource_manager,
                    scene: &mut detached_scene,
                    node_handle: Default::default(),
                    message_sender: &scripted_scene.message_sender,
                };

                // Destroy every script instance from nodes that were still alive.
                for node_index in 0..context.scene.graph.capacity() {
                    context.node_handle = context.scene.graph.handle_from_index(node_index);

                    if let Some(mut script) = context
                        .scene
                        .graph
                        .try_get_mut(context.node_handle)
                        .and_then(|node| node.script.take())
                    {
                        // A script could not be initialized in case if we added a scene, and then immediately
                        // removed it. Calling `on_deinit` in this case would be a violation of API contract.
                        if script.initialized {
                            script.on_deinit(&mut context)
                        }
                    }
                }
            }
        }
    }
}

struct ResourceGraphVertex {
    resource: ModelResource,
    children: Vec<ResourceGraphVertex>,
}

impl ResourceGraphVertex {
    pub fn new(model: ModelResource, resource_manager: ResourceManager) -> Self {
        let mut children = Vec::new();

        // Look for dependent resources.
        let mut dependent_resources = HashSet::new();
        for resource in resource_manager.state().iter() {
            if let Some(other_model) = resource.try_cast::<Model>() {
                let mut state = other_model.state();
                if let Some(model_data) = state.data() {
                    if model_data
                        .get_scene()
                        .graph
                        .linear_iter()
                        .any(|n| n.resource.as_ref().map_or(false, |r| r == &model))
                    {
                        dependent_resources.insert(other_model.clone());
                    }
                }
            }
        }

        children.extend(
            dependent_resources
                .into_iter()
                .map(|r| ResourceGraphVertex::new(r, resource_manager.clone())),
        );

        Self {
            resource: model,
            children,
        }
    }

    pub fn resolve(&self, resource_manager: &ResourceManager) {
        Log::info(format!(
            "Resolving {} resource from dependency graph...",
            self.resource.kind()
        ));

        // Wait until resource is fully loaded, then resolve.
        if block_on(self.resource.clone()).is_ok() {
            self.resource
                .data_ref()
                .get_scene_mut()
                .resolve(resource_manager);

            for child in self.children.iter() {
                child.resolve(resource_manager);
            }
        }
    }
}

struct ResourceDependencyGraph {
    root: ResourceGraphVertex,
}

impl ResourceDependencyGraph {
    pub fn new(model: ModelResource, resource_manager: ResourceManager) -> Self {
        Self {
            root: ResourceGraphVertex::new(model, resource_manager),
        }
    }

    pub fn resolve(&self, resource_manager: &ResourceManager) {
        self.root.resolve(resource_manager)
    }
}

/// A set of parameters that could be used to initialize graphics context.
#[derive(Clone)]
pub struct GraphicsContextParams {
    /// Attributes of the main application window.
    pub window_attributes: WindowAttributes,

    /// Whether to use vertical synchronization or not. V-sync will force your game to render frames with the synchronization
    /// rate of your monitor (which is ~60 FPS). Keep in mind that vertical synchronization might not be available on your OS.
    pub vsync: bool,
}

impl Default for GraphicsContextParams {
    fn default() -> Self {
        Self {
            window_attributes: Default::default(),
            vsync: true,
        }
    }
}

/// Engine initialization parameters.
pub struct EngineInitParams {
    /// A set of parameters for graphics context initialization. Keep in mind that the engine **will not** initialize
    /// graphics context for you. Instead, you need to call [`Engine::initialize_graphics_context`] on [`Event::Resumed`]
    /// event and [`Engine::destroy_graphics_context`] on [`Event::Suspended`] event. If you don't need a graphics context
    /// (for example for game servers), then you can pass [`Default::default`] here and do not call any methods.
    pub graphics_context_params: GraphicsContextParams,
    /// A special container that is able to create nodes by their type UUID.
    pub serialization_context: Arc<SerializationContext>,
    /// A resource manager.
    pub resource_manager: ResourceManager,
    /// Task pool for asynchronous task management.
    pub task_pool: Arc<TaskPool>,
}

macro_rules! define_process_node {
    ($name:ident, $ctx_type:ty) => {
        fn $name<T>(context: &mut $ctx_type, func: &mut T)
        where
            T: FnMut(&mut Script, &mut $ctx_type),
        {
            // Take a script from node. We're temporarily taking ownership over script
            // instance.
            let mut script = match context.scene.graph.try_get_mut(context.handle) {
                Some(node) => {
                    if !node.is_globally_enabled() {
                        return;
                    }

                    if let Some(script) = node.script.take() {
                        script
                    } else {
                        // No script.
                        return;
                    }
                }
                None => {
                    // Invalid handle.
                    return;
                }
            };

            func(&mut script, context);

            // Put the script back to the node. We must do a checked borrow, because it is possible
            // that the node is already destroyed by script logic.
            if let Some(node) = context.scene.graph.try_get_mut(context.handle) {
                node.script = Some(script);
            }
        }
    };
}

define_process_node!(process_node, ScriptContext);
define_process_node!(process_node_message, ScriptMessageContext);

pub(crate) fn process_scripts<T>(
    scene: &mut Scene,
    scene_handle: Handle<Scene>,
    plugins: &mut [Box<dyn Plugin>],
    resource_manager: &ResourceManager,
    message_sender: &ScriptMessageSender,
    message_dispatcher: &mut ScriptMessageDispatcher,
    task_pool: &mut TaskPoolHandler,
    graphics_context: &mut GraphicsContext,
    dt: f32,
    elapsed_time: f32,
    mut func: T,
) where
    T: FnMut(&mut Script, &mut ScriptContext),
{
    let mut context = ScriptContext {
        dt,
        elapsed_time,
        plugins,
        handle: Default::default(),
        scene,
        scene_handle,
        resource_manager,
        message_sender,
        message_dispatcher,
        task_pool,
        graphics_context,
    };

    for node_index in 0..context.scene.graph.capacity() {
        context.handle = context.scene.graph.handle_from_index(node_index);

        process_node(&mut context, &mut func);
    }
}

pub(crate) fn initialize_resource_manager_loaders(
    resource_manager: &ResourceManager,
    serialization_context: Arc<SerializationContext>,
) {
    let model_loader = ModelLoader {
        resource_manager: resource_manager.clone(),
        serialization_context,
        default_import_options: Default::default(),
    };

    let mut state = resource_manager.state();

    for shader in ShaderResource::standard_shaders() {
        state
            .built_in_resources
            .insert(shader.kind().path_owned().unwrap(), shader.into_untyped());
    }

    for texture in SkyBoxKind::built_in_skybox_textures() {
        state.built_in_resources.insert(
            texture.kind().path_owned().unwrap(),
            texture.clone().into_untyped(),
        );
    }

    state.built_in_resources.insert(
        BUILT_IN_FONT.kind().path_owned().unwrap(),
        BUILT_IN_FONT.clone().into_untyped(),
    );

    state.constructors_container.add::<Texture>();
    state.constructors_container.add::<Shader>();
    state.constructors_container.add::<Model>();
    state.constructors_container.add::<CurveResourceState>();
    state.constructors_container.add::<SoundBuffer>();
    state.constructors_container.add::<HrirSphereResourceData>();
    state.constructors_container.add::<Material>();
    state.constructors_container.add::<Font>();
    state.constructors_container.add::<UserInterface>();

    let loaders = &mut state.loaders;
    loaders.set(model_loader);
    loaders.set(TextureLoader {
        default_import_options: Default::default(),
    });
    loaders.set(SoundBufferLoader {
        default_import_options: Default::default(),
    });
    loaders.set(ShaderLoader);
    loaders.set(CurveLoader);
    loaders.set(HrirSphereLoader);
    loaders.set(MaterialLoader {
        resource_manager: resource_manager.clone(),
    });
    loaders.set(FontLoader::default());
    loaders.set(UserInterfaceLoader {
        resource_manager: resource_manager.clone(),
    });
}

impl Engine {
    /// Creates new instance of engine from given initialization parameters. Automatically creates all sub-systems
    /// (sound, ui, resource manager, etc.) **except** graphics context. Graphics context should be created manually
    /// only on [`Event::Resumed`] by calling [`Engine::initialize_graphics_context`] and destroyed on [`Event::Suspended`]
    /// by calling [`Engine::destroy_graphics_context`]. If you don't need a graphics context (for example if you're
    /// making a game server), then you can ignore these methods.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fyrox::{
    ///     asset::manager::ResourceManager,
    ///     engine::{
    ///         Engine, EngineInitParams, GraphicsContextParams,
    ///         SerializationContext,
    ///     },
    ///     event_loop::EventLoop,
    ///     window::WindowAttributes,
    /// };
    /// use std::sync::Arc;
    /// use fyrox_core::task::TaskPool;
    ///
    /// let mut window_attributes = WindowAttributes::default();
    /// window_attributes.title = "Some title".to_string();
    /// let graphics_context_params = GraphicsContextParams {
    ///     window_attributes,
    ///     vsync: true,
    /// };
    /// let task_pool = Arc::new(TaskPool::new());
    ///
    /// Engine::new(EngineInitParams {
    ///     graphics_context_params,
    ///     resource_manager: ResourceManager::new(task_pool.clone()),
    ///     serialization_context: Arc::new(SerializationContext::new()),
    ///     task_pool
    /// })
    /// .unwrap();
    /// ```
    #[inline]
    #[allow(unused_variables)]
    pub fn new(params: EngineInitParams) -> Result<Self, EngineError> {
        let EngineInitParams {
            graphics_context_params,
            serialization_context,
            resource_manager,
            task_pool,
        } = params;

        initialize_resource_manager_loaders(&resource_manager, serialization_context.clone());

        let (rx, tx) = channel();
        resource_manager.state().event_broadcaster.add(rx);

        let sound_engine = SoundEngine::without_device();

        Ok(Self {
            graphics_context: GraphicsContext::Uninitialized(graphics_context_params),
            model_events_receiver: tx,
            async_scene_loader: AsyncSceneLoader::new(
                resource_manager.clone(),
                serialization_context.clone(),
            ),
            resource_manager,
            scenes: SceneContainer::new(sound_engine.clone()),
            sound_engine,
            user_interface: UserInterface::new(Vector2::new(100.0, 100.0)),
            performance_statistics: Default::default(),
            plugins: Default::default(),
            serialization_context,
            script_processor: Default::default(),
            plugins_enabled: false,
            plugin_constructors: Default::default(),
            elapsed_time: 0.0,
            task_pool: TaskPoolHandler::new(task_pool),
        })
    }

    /// Tries to initialize the graphics context. The method will attempt to use the info stored in `graphics_context`
    /// variable of the engine to attempt to initialize the graphics context. It will fail if the graphics context is
    /// already initialized as well as if there any platform-dependent error (for example your hardware does not support
    /// OpenGL 3.3 Core or OpenGL ES 3.0).
    ///
    /// This method should be called on [`Event::Resumed`] of your game loop, however you can ignore it if you don't need
    /// graphics context at all (for example - if you're making game server).
    pub fn initialize_graphics_context(
        &mut self,
        window_target: &EventLoopWindowTarget<()>,
    ) -> Result<(), EngineError> {
        if let GraphicsContext::Uninitialized(params) = &self.graphics_context {
            let mut window_builder = WindowBuilder::new();
            if let Some(inner_size) = params.window_attributes.inner_size {
                window_builder = window_builder.with_inner_size(inner_size);
            }
            if let Some(min_inner_size) = params.window_attributes.min_inner_size {
                window_builder = window_builder.with_min_inner_size(min_inner_size);
            }
            if let Some(max_inner_size) = params.window_attributes.max_inner_size {
                window_builder = window_builder.with_min_inner_size(max_inner_size);
            }
            if let Some(position) = params.window_attributes.position {
                window_builder = window_builder.with_position(position);
            }
            if let Some(resize_increments) = params.window_attributes.resize_increments {
                window_builder = window_builder.with_resize_increments(resize_increments);
            }
            unsafe {
                window_builder = window_builder
                    .with_parent_window(params.window_attributes.parent_window().cloned());
            }
            window_builder = window_builder
                .with_resizable(params.window_attributes.resizable)
                .with_enabled_buttons(params.window_attributes.enabled_buttons)
                .with_title(params.window_attributes.title.clone())
                .with_fullscreen(params.window_attributes.fullscreen().cloned())
                .with_maximized(params.window_attributes.maximized)
                .with_visible(params.window_attributes.visible)
                .with_transparent(params.window_attributes.transparent)
                .with_decorations(params.window_attributes.decorations)
                .with_window_icon(params.window_attributes.window_icon.clone())
                .with_theme(params.window_attributes.preferred_theme)
                .with_content_protected(params.window_attributes.content_protected)
                .with_window_level(params.window_attributes.window_level)
                .with_active(params.window_attributes.active);

            #[cfg(not(target_arch = "wasm32"))]
            let (window, gl_context, gl_surface, glow_context, gl_kind) = {
                let template = ConfigTemplateBuilder::new()
                    .prefer_hardware_accelerated(Some(true))
                    .with_stencil_size(8)
                    .with_depth_size(24);

                let (opt_window, gl_config) = DisplayBuilder::new()
                    .with_window_builder(Some(window_builder))
                    .build(window_target, template, |mut configs| {
                        configs.next().unwrap()
                    })?;

                let window = opt_window.unwrap();

                let raw_window_handle = window.raw_window_handle();

                let gl_display = gl_config.display();

                #[cfg(debug_assertions)]
                let debug = true;

                #[cfg(not(debug_assertions))]
                let debug = true;

                let gl3_3_core_context_attributes = ContextAttributesBuilder::new()
                    .with_debug(debug)
                    .with_profile(GlProfile::Core)
                    .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
                    .build(Some(raw_window_handle));

                let gles3_context_attributes = ContextAttributesBuilder::new()
                    .with_debug(debug)
                    .with_profile(GlProfile::Core)
                    .with_context_api(ContextApi::Gles(Some(Version::new(3, 0))))
                    .build(Some(raw_window_handle));

                unsafe {
                    let attrs = window.build_surface_attributes(Default::default());

                    let gl_surface = gl_config
                        .display()
                        .create_window_surface(&gl_config, &attrs)?;

                    let (non_current_gl_context, gl_kind) = if let Ok(gl3_3_core_context) =
                        gl_display.create_context(&gl_config, &gl3_3_core_context_attributes)
                    {
                        (gl3_3_core_context, GlKind::OpenGL)
                    } else {
                        (
                            gl_display.create_context(&gl_config, &gles3_context_attributes)?,
                            GlKind::OpenGLES,
                        )
                    };

                    let gl_context = non_current_gl_context.make_current(&gl_surface)?;

                    if params.vsync {
                        Log::verify(gl_surface.set_swap_interval(
                            &gl_context,
                            SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
                        ));
                    }

                    (
                        window,
                        gl_context,
                        gl_surface,
                        glow::Context::from_loader_function(|s| {
                            gl_display.get_proc_address(&CString::new(s).unwrap())
                        }),
                        gl_kind,
                    )
                }
            };

            #[cfg(target_arch = "wasm32")]
            let (window, glow_context, gl_kind) = {
                use crate::{
                    core::wasm_bindgen::JsCast,
                    dpi::{LogicalSize, PhysicalSize},
                    platform::web::WindowExtWebSys,
                };

                let inner_size = window_builder.window_attributes().inner_size;
                let window = window_builder.build(window_target).unwrap();

                let web_window = crate::core::web_sys::window().unwrap();
                let scale_factor = web_window.device_pixel_ratio();

                let canvas = window.canvas().unwrap();

                // For some reason winit completely ignores the requested inner size. This is a quick-n-dirty fix
                // that also handles HiDPI monitors. It has one issue - if user changes DPI, it won't be handled
                // correctly.
                if let Some(inner_size) = inner_size {
                    let physical_inner_size: PhysicalSize<u32> =
                        inner_size.to_physical(scale_factor);

                    canvas.set_width(physical_inner_size.width);
                    canvas.set_height(physical_inner_size.height);

                    let logical_inner_size: LogicalSize<f64> = inner_size.to_logical(scale_factor);
                    Log::verify(
                        canvas
                            .style()
                            .set_property("width", &format!("{}px", logical_inner_size.width)),
                    );
                    Log::verify(
                        canvas
                            .style()
                            .set_property("height", &format!("{}px", logical_inner_size.height)),
                    );
                }

                let document = web_window.document().unwrap();
                let body = document.body().unwrap();

                body.append_child(&canvas)
                    .expect("Append canvas to HTML body");

                let webgl2_context = canvas
                    .get_context("webgl2")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<crate::core::web_sys::WebGl2RenderingContext>()
                    .unwrap();
                (
                    window,
                    glow::Context::from_webgl2_context(webgl2_context),
                    GlKind::OpenGLES,
                )
            };

            self.user_interface.set_screen_size(Vector2::new(
                window.inner_size().width as f32,
                window.inner_size().height as f32,
            ));

            #[cfg(not(target_arch = "wasm32"))]
            gl_surface.resize(
                &gl_context,
                NonZeroU32::new(window.inner_size().width)
                    .unwrap_or_else(|| NonZeroU32::new(1).unwrap()),
                NonZeroU32::new(window.inner_size().height)
                    .unwrap_or_else(|| NonZeroU32::new(1).unwrap()),
            );

            self.graphics_context = GraphicsContext::Initialized(InitializedGraphicsContext {
                #[cfg(not(target_arch = "wasm32"))]
                gl_context,
                #[cfg(not(target_arch = "wasm32"))]
                gl_surface,
                renderer: Renderer::new(
                    glow_context,
                    (window.inner_size().width, window.inner_size().height),
                    &self.resource_manager,
                    gl_kind,
                )?,
                window,
                params: params.clone(),
            });

            self.sound_engine.initialize_audio_output_device()?;

            Ok(())
        } else {
            Err(EngineError::Custom(
                "Graphics context is already initialized!".to_string(),
            ))
        }
    }

    /// Tries to destroy current graphics context. It will succeed only if the `graphics_context` is fully initialized.
    /// The method will try to save all possible runtime changes of the window, so the next [`Engine::initialize_graphics_context`]
    /// will result in the almost exact copy of the context that was made before destruction.
    ///
    /// This method should be called on [`Event::Suspended`] of your game loop, however if you do not use any graphics context
    /// (for example - if you're making a game server), then you can ignore this method completely.
    pub fn destroy_graphics_context(&mut self) -> Result<(), EngineError> {
        if let GraphicsContext::Initialized(ref ctx) = self.graphics_context {
            let params = &ctx.params;
            let window = &ctx.window;

            let mut window_attributes = WindowAttributes::default();

            window_attributes.inner_size = Some(Size::Physical(window.inner_size()));
            window_attributes.min_inner_size = params.window_attributes.min_inner_size;
            window_attributes.max_inner_size = params.window_attributes.max_inner_size;
            window_attributes.position = window.outer_position().ok().map(Position::Physical);
            window_attributes.resizable = window.is_resizable();
            window_attributes.enabled_buttons = window.enabled_buttons();
            window_attributes.title = window.title();
            window_attributes.maximized = window.is_maximized();
            window_attributes.visible = window.is_visible().unwrap_or(true);
            window_attributes.transparent = params.window_attributes.transparent;
            window_attributes.decorations = window.is_decorated();
            window_attributes.window_icon = params.window_attributes.window_icon.clone();
            window_attributes.preferred_theme = params.window_attributes.preferred_theme;
            window_attributes.resize_increments = window.resize_increments().map(Size::Physical);
            window_attributes.content_protected = params.window_attributes.content_protected;
            window_attributes.window_level = params.window_attributes.window_level;
            window_attributes.active = params.window_attributes.active;

            self.graphics_context = GraphicsContext::Uninitialized(GraphicsContextParams {
                window_attributes,
                vsync: params.vsync,
            });

            self.sound_engine.destroy_audio_output_device();

            Ok(())
        } else {
            Err(EngineError::Custom(
                "Graphics context is already destroyed!".to_string(),
            ))
        }
    }

    /// Adjust size of the frame to be rendered. Must be called after the window size changes.
    /// Will update the renderer and GL context frame size.
    pub fn set_frame_size(&mut self, new_size: (u32, u32)) -> Result<(), FrameworkError> {
        if let GraphicsContext::Initialized(ctx) = &mut self.graphics_context {
            ctx.renderer.set_frame_size(new_size)?;

            #[cfg(not(target_arch = "wasm32"))]
            ctx.gl_surface.resize(
                &ctx.gl_context,
                NonZeroU32::new(new_size.0).unwrap_or_else(|| NonZeroU32::new(1).unwrap()),
                NonZeroU32::new(new_size.1).unwrap_or_else(|| NonZeroU32::new(1).unwrap()),
            );
        }

        Ok(())
    }

    /// Amount of time (in seconds) that passed from creation of the engine. Keep in mind, that
    /// this value is **not** guaranteed to match real time. A user can change delta time with
    /// which the engine "ticks" and this delta time affects elapsed time.
    pub fn elapsed_time(&self) -> f32 {
        self.elapsed_time
    }

    /// Performs single update tick with given time delta. Engine internally will perform update
    /// of all scenes, sub-systems, user interface, etc. Must be called in order to get engine
    /// functioning.
    ///
    /// ## Parameters
    ///
    /// `lag` - is a reference to time accumulator, that holds remaining amount of time that should be used
    /// to update a plugin. A caller splits `lag` into multiple sub-steps using `dt` and thus stabilizes
    /// update rate. The main use of this variable, is to be able to reset `lag` when you doing some heavy
    /// calculations in a your game loop (i.e. loading a new level) so the engine won't try to "catch up" with
    /// all the time that was spent in heavy calculation. The engine does **not** use this variable itself,
    /// but the plugins attach may use it, that's why you need to provide it. If you don't use plugins, then
    /// put `&mut 0.0` here.
    pub fn update(
        &mut self,
        dt: f32,
        window_target: &EventLoopWindowTarget<()>,
        lag: &mut f32,
        switches: FxHashMap<Handle<Scene>, GraphUpdateSwitches>,
    ) {
        self.handle_async_scene_loading(dt, lag, window_target);
        self.pre_update(dt, window_target, lag, switches);
        self.post_update(dt);
    }

    fn handle_async_scene_loading(
        &mut self,
        dt: f32,
        lag: &mut f32,
        window_target: &EventLoopWindowTarget<()>,
    ) {
        let len = self.async_scene_loader.loading_scenes.len();
        let mut n = 0;
        while n < len {
            if let Some(request) = self.async_scene_loader.loading_scenes.values_mut().nth(n) {
                if !request.reported {
                    request.reported = true;

                    // Notify plugins about a scene, that started loading.
                    if self.plugins_enabled {
                        let path = request.path.clone();
                        let mut context = PluginContext {
                            scenes: &mut self.scenes,
                            resource_manager: &self.resource_manager,
                            graphics_context: &mut self.graphics_context,
                            dt,
                            lag,
                            user_interface: &mut self.user_interface,
                            serialization_context: &self.serialization_context,
                            performance_statistics: &self.performance_statistics,
                            elapsed_time: self.elapsed_time,
                            script_processor: &self.script_processor,
                            async_scene_loader: &mut self.async_scene_loader,
                            window_target: Some(window_target),
                            task_pool: &mut self.task_pool,
                        };

                        for plugin in self.plugins.iter_mut() {
                            plugin.on_scene_begin_loading(&path, &mut context);
                        }
                    }
                }
            }

            n += 1;
        }

        while let Ok(loading_result) = self.async_scene_loader.receiver.try_recv() {
            if let Some(request) = self
                .async_scene_loader
                .loading_scenes
                .remove(&loading_result.path)
            {
                let mut context = PluginContext {
                    scenes: &mut self.scenes,
                    resource_manager: &self.resource_manager,
                    graphics_context: &mut self.graphics_context,
                    dt,
                    lag,
                    user_interface: &mut self.user_interface,
                    serialization_context: &self.serialization_context,
                    performance_statistics: &self.performance_statistics,
                    elapsed_time: self.elapsed_time,
                    script_processor: &self.script_processor,
                    async_scene_loader: &mut self.async_scene_loader,
                    window_target: Some(window_target),
                    task_pool: &mut self.task_pool,
                };

                match loading_result.result {
                    Ok((mut scene, data)) => {
                        if request.options.derived {
                            // Create a resource, that will point to the scene we've loaded the
                            // scene from and force scene nodes to inherit data from them.
                            let model = ModelResource::new_ok(
                                ResourceKind::External(request.path.clone()),
                                Model {
                                    mapping: NodeMapping::UseHandles,
                                    // We have to create a full copy of the scene, because otherwise
                                    // some methods (`Base::root_resource` in particular) won't work
                                    // correctly.
                                    scene: scene
                                        .clone(
                                            scene.graph.get_root(),
                                            &mut |_, _| true,
                                            &mut |_, _, _| {},
                                        )
                                        .0,
                                },
                            );

                            Log::verify(self.resource_manager.register(
                                model.clone().into_untyped(),
                                request.path.clone(),
                                |_, _| true,
                            ));

                            for (handle, node) in scene.graph.pair_iter_mut() {
                                node.set_inheritance_data(handle, model.clone());
                            }

                            // Reset modified flags in every inheritable property of the scene.
                            // Except nodes, they're inherited in a separate place.
                            (&mut scene as &mut dyn Reflect).apply_recursively_mut(
                                &mut |object| {
                                    let type_id = (*object).type_id();
                                    if type_id != TypeId::of::<NodePool>() {
                                        object.as_inheritable_variable_mut(&mut |variable| {
                                            if let Some(variable) = variable {
                                                variable.reset_modified_flag();
                                            }
                                        });
                                    }
                                },
                                &[TypeId::of::<UntypedResource>()],
                            )
                        } else {
                            // Take scene data from the source scene.
                            if let Some(source_asset) =
                                scene.graph[scene.graph.get_root()].root_resource()
                            {
                                let source_asset_ref = source_asset.data_ref();
                                let source_scene_ref = &source_asset_ref.scene;
                                Log::verify(try_inherit_properties(
                                    &mut scene,
                                    source_scene_ref,
                                    &[TypeId::of::<NodePool>(), TypeId::of::<UntypedResource>()],
                                ));
                            }
                        }

                        let scene_handle = context.scenes.add(scene);

                        // Notify plugins about newly loaded scene.
                        if self.plugins_enabled {
                            for plugin in self.plugins.iter_mut() {
                                Log::info(format!(
                                    "Scene {} was loaded successfully!",
                                    loading_result.path.display()
                                ));

                                plugin.on_scene_loaded(
                                    &request.path,
                                    scene_handle,
                                    &data,
                                    &mut context,
                                );
                            }
                        }
                    }
                    Err(error) => {
                        // Notify plugins about a scene, that is failed to load.
                        if self.plugins_enabled {
                            Log::err(format!(
                                "Unable to load scene {}. Reason: {:?}",
                                loading_result.path.display(),
                                error
                            ));

                            for plugin in self.plugins.iter_mut() {
                                plugin.on_scene_loading_failed(&request.path, &error, &mut context);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Performs pre update for the engine.
    ///
    /// Normally, this is called from `Engine::update()`.
    /// You should only call this manually if you don't use that method.
    ///
    /// ## Parameters
    ///
    /// `lag` - is a reference to time accumulator, that holds remaining amount of time that should be used
    /// to update a plugin. A caller splits `lag` into multiple sub-steps using `dt` and thus stabilizes
    /// update rate. The main use of this variable, is to be able to reset `lag` when you doing some heavy
    /// calculations in a your game loop (i.e. loading a new level) so the engine won't try to "catch up" with
    /// all the time that was spent in heavy calculation. The engine does **not** use this variable itself,
    /// but the plugins attach may use it, that's why you need to provide it. If you don't use plugins, then
    /// put `&mut 0.0` here.
    pub fn pre_update(
        &mut self,
        dt: f32,
        window_target: &EventLoopWindowTarget<()>,
        lag: &mut f32,
        switches: FxHashMap<Handle<Scene>, GraphUpdateSwitches>,
    ) {
        if let GraphicsContext::Initialized(ctx) = &mut self.graphics_context {
            let inner_size = ctx.window.inner_size();
            let window_size = Vector2::new(inner_size.width as f32, inner_size.height as f32);

            self.resource_manager.state().update(dt);
            ctx.renderer.update_caches(dt);
            self.handle_model_events();

            for (handle, scene) in self.scenes.pair_iter_mut().filter(|(_, s)| *s.enabled) {
                let frame_size =
                    scene
                        .rendering_options
                        .render_target
                        .as_ref()
                        .map_or(window_size, |rt| {
                            if let TextureKind::Rectangle { width, height } = rt.data_ref().kind() {
                                Vector2::new(width as f32, height as f32)
                            } else {
                                panic!("only rectangle textures can be used as render target!");
                            }
                        });

                scene.update(
                    frame_size,
                    dt,
                    switches.get(&handle).cloned().unwrap_or_default(),
                );
            }

            self.update_plugins(dt, window_target, lag);
            self.handle_scripts(dt);
        }
    }

    /// Performs post update for the engine.
    ///
    /// Normally, this is called from `Engine::update()`.
    /// You should only call this manually if you don't use that method.
    pub fn post_update(&mut self, dt: f32) {
        if let GraphicsContext::Initialized(ref ctx) = self.graphics_context {
            let inner_size = ctx.window.inner_size();
            let window_size = Vector2::new(inner_size.width as f32, inner_size.height as f32);

            let time = instant::Instant::now();
            self.user_interface.update(window_size, dt);
            self.performance_statistics.ui_time = instant::Instant::now() - time;
            self.elapsed_time += dt;
        }
    }

    /// Returns true if the scene is registered for script processing.
    pub fn has_scripted_scene(&self, scene: Handle<Scene>) -> bool {
        self.script_processor.has_scripted_scene(scene)
    }

    /// Registers a scene for script processing.
    pub fn register_scripted_scene(&mut self, scene: Handle<Scene>) {
        self.script_processor
            .register_scripted_scene(scene, &self.resource_manager)
    }

    fn handle_scripts(&mut self, dt: f32) {
        let time = instant::Instant::now();

        while let Some((payload, handler)) = self.task_pool.pop_node_task() {
            if let Some(scripted_scene) = self
                .script_processor
                .scripted_scenes
                .iter_mut()
                .find(|e| e.handle == handler.scene_handle)
            {
                if let Some(scene) = self.scenes.try_get_mut(handler.scene_handle) {
                    if let Some(mut script) = scene
                        .graph
                        .try_get_mut(handler.node_handle)
                        .and_then(|n| n.script.take())
                    {
                        (handler.closure)(
                            payload,
                            script.deref_mut(),
                            &mut ScriptContext {
                                dt,
                                elapsed_time: self.elapsed_time,
                                plugins: &mut self.plugins,
                                handle: handler.node_handle,
                                scene,
                                scene_handle: scripted_scene.handle,
                                resource_manager: &self.resource_manager,
                                message_sender: &scripted_scene.message_sender,
                                message_dispatcher: &mut scripted_scene.message_dispatcher,
                                task_pool: &mut self.task_pool,
                                graphics_context: &mut self.graphics_context,
                            },
                        );

                        if let Some(node) = scene.graph.try_get_mut(handler.node_handle) {
                            node.script = Some(script);
                        }
                    }
                }
            }
        }

        self.script_processor.handle_scripts(
            &mut self.scenes,
            &mut self.plugins,
            &self.resource_manager,
            &mut self.task_pool,
            &mut self.graphics_context,
            dt,
            self.elapsed_time,
        );

        self.performance_statistics.scripts_time = instant::Instant::now() - time;
    }

    fn update_plugins(
        &mut self,
        dt: f32,
        window_target: &EventLoopWindowTarget<()>,
        lag: &mut f32,
    ) {
        let time = instant::Instant::now();

        if self.plugins_enabled {
            // Handle asynchronous tasks first.
            while let Some((payload, handler)) = self.task_pool.pop_plugin_task() {
                (handler)(
                    payload,
                    &mut self.plugins,
                    &mut PluginContext {
                        scenes: &mut self.scenes,
                        resource_manager: &self.resource_manager,
                        graphics_context: &mut self.graphics_context,
                        dt,
                        lag,
                        user_interface: &mut self.user_interface,
                        serialization_context: &self.serialization_context,
                        performance_statistics: &self.performance_statistics,
                        elapsed_time: self.elapsed_time,
                        script_processor: &self.script_processor,
                        async_scene_loader: &mut self.async_scene_loader,
                        window_target: Some(window_target),
                        task_pool: &mut self.task_pool,
                    },
                )
            }

            // Then update all the plugins.
            let mut context = PluginContext {
                scenes: &mut self.scenes,
                resource_manager: &self.resource_manager,
                graphics_context: &mut self.graphics_context,
                dt,
                lag,
                user_interface: &mut self.user_interface,
                serialization_context: &self.serialization_context,
                performance_statistics: &self.performance_statistics,
                elapsed_time: self.elapsed_time,
                script_processor: &self.script_processor,
                async_scene_loader: &mut self.async_scene_loader,
                window_target: Some(window_target),
                task_pool: &mut self.task_pool,
            };

            for plugin in self.plugins.iter_mut() {
                plugin.update(&mut context);
            }

            while let Some(message) = self.user_interface.poll_message() {
                let mut context = PluginContext {
                    scenes: &mut self.scenes,
                    resource_manager: &self.resource_manager,
                    graphics_context: &mut self.graphics_context,
                    dt,
                    lag,
                    user_interface: &mut self.user_interface,
                    serialization_context: &self.serialization_context,
                    performance_statistics: &self.performance_statistics,
                    elapsed_time: self.elapsed_time,
                    script_processor: &self.script_processor,
                    async_scene_loader: &mut self.async_scene_loader,
                    window_target: Some(window_target),
                    task_pool: &mut self.task_pool,
                };

                for plugin in self.plugins.iter_mut() {
                    plugin.on_ui_message(&mut context, &message);
                }
            }
        }

        self.performance_statistics.plugins_time = instant::Instant::now() - time;
    }

    pub(crate) fn handle_os_event_by_plugins(
        &mut self,
        event: &Event<()>,
        dt: f32,
        window_target: &EventLoopWindowTarget<()>,
        lag: &mut f32,
    ) {
        if self.plugins_enabled {
            for plugin in self.plugins.iter_mut() {
                plugin.on_os_event(
                    event,
                    PluginContext {
                        scenes: &mut self.scenes,
                        resource_manager: &self.resource_manager,
                        graphics_context: &mut self.graphics_context,
                        dt,
                        lag,
                        user_interface: &mut self.user_interface,
                        serialization_context: &self.serialization_context,
                        performance_statistics: &self.performance_statistics,
                        elapsed_time: self.elapsed_time,
                        script_processor: &self.script_processor,
                        async_scene_loader: &mut self.async_scene_loader,
                        window_target: Some(window_target),
                        task_pool: &mut self.task_pool,
                    },
                );
            }
        }
    }

    pub(crate) fn handle_graphics_context_created_by_plugins(
        &mut self,
        dt: f32,
        window_target: &EventLoopWindowTarget<()>,
        lag: &mut f32,
    ) {
        if self.plugins_enabled {
            for plugin in self.plugins.iter_mut() {
                plugin.on_graphics_context_initialized(PluginContext {
                    scenes: &mut self.scenes,
                    resource_manager: &self.resource_manager,
                    graphics_context: &mut self.graphics_context,
                    dt,
                    lag,
                    user_interface: &mut self.user_interface,
                    serialization_context: &self.serialization_context,
                    performance_statistics: &self.performance_statistics,
                    elapsed_time: self.elapsed_time,
                    script_processor: &self.script_processor,
                    async_scene_loader: &mut self.async_scene_loader,
                    window_target: Some(window_target),
                    task_pool: &mut self.task_pool,
                });
            }
        }
    }

    pub(crate) fn handle_graphics_context_destroyed_by_plugins(
        &mut self,
        dt: f32,
        window_target: &EventLoopWindowTarget<()>,
        lag: &mut f32,
    ) {
        if self.plugins_enabled {
            for plugin in self.plugins.iter_mut() {
                plugin.on_graphics_context_destroyed(PluginContext {
                    scenes: &mut self.scenes,
                    resource_manager: &self.resource_manager,
                    graphics_context: &mut self.graphics_context,
                    dt,
                    lag,
                    user_interface: &mut self.user_interface,
                    serialization_context: &self.serialization_context,
                    performance_statistics: &self.performance_statistics,
                    elapsed_time: self.elapsed_time,
                    script_processor: &self.script_processor,
                    async_scene_loader: &mut self.async_scene_loader,
                    window_target: Some(window_target),
                    task_pool: &mut self.task_pool,
                });
            }
        }
    }

    pub(crate) fn handle_before_rendering_by_plugins(
        &mut self,
        dt: f32,
        window_target: &EventLoopWindowTarget<()>,
        lag: &mut f32,
    ) {
        if self.plugins_enabled {
            for plugin in self.plugins.iter_mut() {
                plugin.before_rendering(PluginContext {
                    scenes: &mut self.scenes,
                    resource_manager: &self.resource_manager,
                    graphics_context: &mut self.graphics_context,
                    dt,
                    lag,
                    user_interface: &mut self.user_interface,
                    serialization_context: &self.serialization_context,
                    performance_statistics: &self.performance_statistics,
                    elapsed_time: self.elapsed_time,
                    script_processor: &self.script_processor,
                    async_scene_loader: &mut self.async_scene_loader,
                    window_target: Some(window_target),
                    task_pool: &mut self.task_pool,
                });
            }
        }
    }

    /// Passes specified OS event to every script of the specified scene.
    ///
    /// # Important notes
    ///
    /// This method is intended to be used by the editor and game runner. If you're using the
    /// engine as a framework, then you should not call this method because you'll most likely
    /// do something wrong.
    pub(crate) fn handle_os_event_by_scripts(
        &mut self,
        event: &Event<()>,
        scene_handle: Handle<Scene>,
        dt: f32,
    ) {
        if let Some(scripted_scene) = self
            .script_processor
            .scripted_scenes
            .iter_mut()
            .find(|s| s.handle == scene_handle)
        {
            let scene = &mut self.scenes[scene_handle];
            if *scene.enabled {
                process_scripts(
                    scene,
                    scene_handle,
                    &mut self.plugins,
                    &self.resource_manager,
                    &scripted_scene.message_sender,
                    &mut scripted_scene.message_dispatcher,
                    &mut self.task_pool,
                    &mut self.graphics_context,
                    dt,
                    self.elapsed_time,
                    |script, context| {
                        if script.initialized && script.started {
                            script.on_os_event(event, context);
                        }
                    },
                )
            }
        }
    }

    /// Handle hot-reloading of resources.
    ///
    /// Normally, this is called from `Engine::update()`.
    /// You should only call this manually if you don't use that method.
    pub fn handle_model_events(&mut self) {
        while let Ok(event) = self.model_events_receiver.try_recv() {
            if let ResourceEvent::Reloaded(resource) = event {
                if let Some(model) = resource.try_cast::<Model>() {
                    Log::info(format!(
                        "A model resource {} was reloaded, propagating changes...",
                        model.kind()
                    ));

                    // Build resource dependency graph and resolve it first.
                    ResourceDependencyGraph::new(model, self.resource_manager.clone())
                        .resolve(&self.resource_manager);

                    Log::info("Propagating changes to active scenes...");

                    // Resolve all scenes.
                    // TODO: This might be inefficient if there is bunch of scenes loaded,
                    // however this seems to be very rare case so it should be ok.
                    for scene in self.scenes.iter_mut() {
                        scene.resolve(&self.resource_manager);
                    }
                }
            }
        }
    }

    /// Performs rendering of single frame, must be called from your game loop, otherwise you won't
    /// see anything.
    #[inline]
    pub fn render(&mut self) -> Result<(), FrameworkError> {
        self.user_interface.draw();

        if let GraphicsContext::Initialized(ref mut ctx) = self.graphics_context {
            #[cfg(not(target_arch = "wasm32"))]
            {
                ctx.renderer.render_and_swap_buffers(
                    &self.scenes,
                    self.user_interface.get_drawing_context(),
                    &ctx.gl_surface,
                    &ctx.gl_context,
                    &ctx.window,
                )?;
            }
            #[cfg(target_arch = "wasm32")]
            {
                ctx.renderer.render_and_swap_buffers(
                    &self.scenes,
                    &self.user_interface.get_drawing_context(),
                )?;
            }
        }

        Ok(())
    }

    /// Enables or disables registered plugins.
    pub(crate) fn enable_plugins(
        &mut self,
        scene_path: Option<&str>,
        enabled: bool,
        window_target: Option<&EventLoopWindowTarget<()>>,
    ) {
        if self.plugins_enabled != enabled {
            self.plugins_enabled = enabled;

            if self.plugins_enabled {
                // Create and initialize instances.
                for constructor in self.plugin_constructors.iter() {
                    self.plugins.push(constructor.create_instance(
                        scene_path,
                        PluginContext {
                            scenes: &mut self.scenes,
                            resource_manager: &self.resource_manager,
                            graphics_context: &mut self.graphics_context,
                            dt: 0.0,
                            lag: &mut 0.0,
                            user_interface: &mut self.user_interface,
                            serialization_context: &self.serialization_context,
                            performance_statistics: &self.performance_statistics,
                            elapsed_time: self.elapsed_time,
                            script_processor: &self.script_processor,
                            async_scene_loader: &mut self.async_scene_loader,
                            window_target,
                            task_pool: &mut self.task_pool,
                        },
                    ));
                }
            } else {
                self.handle_scripts(0.0);

                for mut plugin in self.plugins.drain(..) {
                    // Deinit plugin first.
                    plugin.on_deinit(PluginContext {
                        scenes: &mut self.scenes,
                        resource_manager: &self.resource_manager,
                        graphics_context: &mut self.graphics_context,
                        dt: 0.0,
                        lag: &mut 0.0,
                        user_interface: &mut self.user_interface,
                        serialization_context: &self.serialization_context,
                        performance_statistics: &self.performance_statistics,
                        elapsed_time: self.elapsed_time,
                        script_processor: &self.script_processor,
                        async_scene_loader: &mut self.async_scene_loader,
                        window_target,
                        task_pool: &mut self.task_pool,
                    });
                }
            }
        }
    }

    /// Adds new plugin plugin constructor.
    pub fn add_plugin_constructor<P>(&mut self, constructor: P)
    where
        P: PluginConstructor + 'static,
    {
        constructor.register(PluginRegistrationContext {
            serialization_context: &self.serialization_context,
            resource_manager: &self.resource_manager,
        });

        self.plugin_constructors.push(Box::new(constructor));
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Destroy all scenes first and correctly destroy all script instances.
        // This will ensure that any `on_destroy` logic will be executed before
        // engine destroyed.
        let scenes = self
            .scenes
            .pair_iter()
            .map(|(h, _)| h)
            .collect::<Vec<Handle<Scene>>>();

        for handle in scenes {
            self.scenes.remove(handle);
        }

        // Finally disable plugins.
        self.enable_plugins(None, false, None);
    }
}

#[cfg(test)]
mod test {
    use crate::{
        asset::manager::ResourceManager,
        core::{
            impl_component_provider, pool::Handle, reflect::prelude::*, task::TaskPool,
            uuid_provider, visitor::prelude::*,
        },
        engine::{task::TaskPoolHandler, GraphicsContext, ScriptProcessor},
        scene::{base::BaseBuilder, node::Node, pivot::PivotBuilder, Scene, SceneContainer},
        script::{
            Script, ScriptContext, ScriptDeinitContext, ScriptMessageContext, ScriptMessagePayload,
            ScriptTrait,
        },
    };
    use std::sync::Arc;

    use std::sync::mpsc::{self, Sender, TryRecvError};

    #[allow(clippy::enum_variant_names)]
    #[derive(PartialEq, Eq, Clone, Debug)]
    enum Event {
        Initialized(Handle<Node>),
        Started(Handle<Node>),
        Updated(Handle<Node>),
        Destroyed(Handle<Node>),
        EventReceived(Handle<Node>),
    }

    #[derive(Debug, Clone, Reflect, Visit)]
    struct MyScript {
        #[reflect(hidden)]
        #[visit(skip)]
        sender: Sender<Event>,
        spawned: bool,
    }

    impl_component_provider!(MyScript);
    uuid_provider!(MyScript = "2569de84-d4b2-427d-969b-d5c7b31a0ba6");

    impl ScriptTrait for MyScript {
        fn on_init(&mut self, ctx: &mut ScriptContext) {
            self.sender.send(Event::Initialized(ctx.handle)).unwrap();

            // Spawn new entity with script.
            let handle =
                PivotBuilder::new(BaseBuilder::new().with_script(Script::new(MySubScript {
                    sender: self.sender.clone(),
                })))
                .build(&mut ctx.scene.graph);
            assert_eq!(handle, Handle::new(2, 1));
        }

        fn on_start(&mut self, ctx: &mut ScriptContext) {
            self.sender.send(Event::Started(ctx.handle)).unwrap();

            // Spawn new entity with script.
            let handle =
                PivotBuilder::new(BaseBuilder::new().with_script(Script::new(MySubScript {
                    sender: self.sender.clone(),
                })))
                .build(&mut ctx.scene.graph);
            assert_eq!(handle, Handle::new(3, 1));
        }

        fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
            self.sender.send(Event::Destroyed(ctx.node_handle)).unwrap();
        }

        fn on_update(&mut self, ctx: &mut ScriptContext) {
            self.sender.send(Event::Updated(ctx.handle)).unwrap();

            if !self.spawned {
                // Spawn new entity with script.
                PivotBuilder::new(BaseBuilder::new().with_script(Script::new(MySubScript {
                    sender: self.sender.clone(),
                })))
                .build(&mut ctx.scene.graph);

                self.spawned = true;
            }
        }
    }

    #[derive(Debug, Clone, Reflect, Visit)]
    struct MySubScript {
        #[reflect(hidden)]
        #[visit(skip)]
        sender: Sender<Event>,
    }

    impl_component_provider!(MySubScript);
    uuid_provider!(MySubScript = "1cebacd9-b500-4753-93be-39db344add21");

    impl ScriptTrait for MySubScript {
        fn on_init(&mut self, ctx: &mut ScriptContext) {
            self.sender.send(Event::Initialized(ctx.handle)).unwrap();
        }

        fn on_start(&mut self, ctx: &mut ScriptContext) {
            self.sender.send(Event::Started(ctx.handle)).unwrap();
        }

        fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
            self.sender.send(Event::Destroyed(ctx.node_handle)).unwrap();
        }

        fn on_update(&mut self, ctx: &mut ScriptContext) {
            self.sender.send(Event::Updated(ctx.handle)).unwrap();
        }
    }

    #[test]
    fn test_order() {
        let resource_manager = ResourceManager::new(Arc::new(Default::default()));
        let mut scene = Scene::new();

        let (tx, rx) = mpsc::channel();

        let node_handle =
            PivotBuilder::new(BaseBuilder::new().with_script(Script::new(MyScript {
                sender: tx,
                spawned: false,
            })))
            .build(&mut scene.graph);
        assert_eq!(node_handle, Handle::new(1, 1));

        let mut scene_container = SceneContainer::new(Default::default());

        let scene_handle = scene_container.add(scene);

        let mut script_processor = ScriptProcessor::default();

        script_processor.register_scripted_scene(scene_handle, &resource_manager);

        let handle_on_init = Handle::new(2, 1);
        let handle_on_start = Handle::new(3, 1);
        let handle_on_update1 = Handle::new(4, 1);
        let mut task_pool = TaskPoolHandler::new(Arc::new(TaskPool::new()));
        let mut gc = GraphicsContext::Uninitialized(Default::default());

        for iteration in 0..3 {
            script_processor.handle_scripts(
                &mut scene_container,
                &mut Default::default(),
                &resource_manager,
                &mut task_pool,
                &mut gc,
                0.0,
                0.0,
            );

            match iteration {
                0 => {
                    assert_eq!(rx.try_recv(), Ok(Event::Initialized(node_handle)));
                    assert_eq!(rx.try_recv(), Ok(Event::Initialized(handle_on_init)));

                    assert_eq!(rx.try_recv(), Ok(Event::Started(node_handle)));
                    assert_eq!(rx.try_recv(), Ok(Event::Started(handle_on_init)));

                    assert_eq!(rx.try_recv(), Ok(Event::Initialized(handle_on_start)));
                    assert_eq!(rx.try_recv(), Ok(Event::Started(handle_on_start)));

                    assert_eq!(rx.try_recv(), Ok(Event::Updated(node_handle)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(handle_on_init)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(handle_on_start)));

                    assert_eq!(rx.try_recv(), Ok(Event::Initialized(handle_on_update1)));
                    assert_eq!(rx.try_recv(), Ok(Event::Started(handle_on_update1)));

                    assert_eq!(rx.try_recv(), Ok(Event::Updated(handle_on_update1)));
                }
                1 => {
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(node_handle)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(handle_on_init)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(handle_on_start)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(handle_on_update1)));
                    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));

                    // Now destroy every node with script, next iteration should correctly destroy attached scripts.
                    let graph = &mut scene_container[scene_handle].graph;
                    graph.remove_node(node_handle);
                    graph.remove_node(handle_on_init);
                    graph.remove_node(handle_on_start);
                    graph.remove_node(handle_on_update1);
                }
                2 => {
                    assert_eq!(rx.try_recv(), Ok(Event::Destroyed(node_handle)));
                    assert_eq!(rx.try_recv(), Ok(Event::Destroyed(handle_on_init)));
                    assert_eq!(rx.try_recv(), Ok(Event::Destroyed(handle_on_start)));
                    assert_eq!(rx.try_recv(), Ok(Event::Destroyed(handle_on_update1)));

                    // Every instance holding sender died, so receiver is disconnected from sender.
                    assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
                }
                _ => (),
            }
        }
    }

    enum MyMessage {
        Foo(usize),
        Bar(String),
    }

    #[derive(Debug, Clone, Reflect, Visit)]
    struct ScriptListeningToMessages {
        index: u32,
        #[reflect(hidden)]
        #[visit(skip)]
        sender: Sender<Event>,
    }

    impl_component_provider!(ScriptListeningToMessages);
    uuid_provider!(ScriptListeningToMessages = "bf2976ad-f41d-4de6-9a32-b1a293956058");

    impl ScriptTrait for ScriptListeningToMessages {
        fn on_start(&mut self, ctx: &mut ScriptContext) {
            ctx.message_dispatcher.subscribe_to::<MyMessage>(ctx.handle);
        }

        fn on_message(
            &mut self,
            message: &mut dyn ScriptMessagePayload,
            ctx: &mut ScriptMessageContext,
        ) {
            let typed_message = message.downcast_ref::<MyMessage>().unwrap();
            match self.index {
                0 => {
                    if let MyMessage::Foo(num) = typed_message {
                        assert_eq!(*num, 123);
                        self.sender.send(Event::EventReceived(ctx.handle)).unwrap();
                    } else {
                        unreachable!()
                    }
                }
                1 => {
                    if let MyMessage::Bar(string) = typed_message {
                        assert_eq!(string, "Foobar");
                        self.sender.send(Event::EventReceived(ctx.handle)).unwrap();
                    } else {
                        unreachable!()
                    }
                }
                _ => (),
            }

            self.index += 1;
        }
    }

    #[derive(Debug, Clone, Reflect, Visit)]
    struct ScriptSendingMessages {
        index: u32,
    }

    impl_component_provider!(ScriptSendingMessages);
    uuid_provider!(ScriptSendingMessages = "6bcbf9b4-9546-42d3-965a-de055ab85475");

    impl ScriptTrait for ScriptSendingMessages {
        fn on_update(&mut self, ctx: &mut ScriptContext) {
            match self.index {
                0 => ctx.message_sender.send_global(MyMessage::Foo(123)),
                1 => ctx
                    .message_sender
                    .send_global(MyMessage::Bar("Foobar".to_string())),
                _ => (),
            }
            self.index += 1;
        }
    }

    #[test]
    fn test_messages() {
        let resource_manager = ResourceManager::new(Arc::new(Default::default()));
        let mut scene = Scene::new();

        let (tx, rx) = mpsc::channel();

        PivotBuilder::new(
            BaseBuilder::new().with_script(Script::new(ScriptSendingMessages { index: 0 })),
        )
        .build(&mut scene.graph);

        let receiver_messages = PivotBuilder::new(BaseBuilder::new().with_script(Script::new(
            ScriptListeningToMessages {
                sender: tx,
                index: 0,
            },
        )))
        .build(&mut scene.graph);

        let mut scene_container = SceneContainer::new(Default::default());

        let scene_handle = scene_container.add(scene);

        let mut script_processor = ScriptProcessor::default();
        let mut task_pool = TaskPoolHandler::new(Arc::new(TaskPool::new()));
        let mut gc = GraphicsContext::Uninitialized(Default::default());

        script_processor.register_scripted_scene(scene_handle, &resource_manager);

        for iteration in 0..2 {
            script_processor.handle_scripts(
                &mut scene_container,
                &mut Default::default(),
                &resource_manager,
                &mut task_pool,
                &mut gc,
                0.0,
                0.0,
            );

            match iteration {
                0 => {
                    assert_eq!(rx.try_recv(), Ok(Event::EventReceived(receiver_messages)));
                    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
                }
                1 => {
                    assert_eq!(rx.try_recv(), Ok(Event::EventReceived(receiver_messages)));
                    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
                }
                _ => (),
            }
        }
    }
}
