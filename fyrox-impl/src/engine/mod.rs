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

//! Engine is container for all subsystems (renderer, ui, sound, resource manager). It also
//! creates a window and an OpenGL context.

#![warn(missing_docs)]

pub mod error;
pub mod executor;
pub mod task;

mod hotreload;

use crate::{
    asset::{
        event::ResourceEvent,
        manager::{ResourceManager, ResourceWaitContext},
        state::ResourceState,
        untyped::{ResourceKind, UntypedResource},
        Resource,
    },
    core::{
        algebra::Vector2,
        futures::{executor::block_on, future::join_all},
        instant,
        log::Log,
        pool::Handle,
        reflect::Reflect,
        task::TaskPool,
        variable::try_inherit_properties,
        visitor::VisitError,
    },
    engine::{error::EngineError, task::TaskPoolHandler},
    event::Event,
    graph::{BaseSceneGraph, NodeMapping, SceneGraph},
    gui::{
        constructor::WidgetConstructorContainer, font::loader::FontLoader, font::Font,
        font::BUILT_IN_FONT, loader::UserInterfaceLoader, UiContainer, UiUpdateSwitches,
        UserInterface,
    },
    material::{
        self,
        loader::MaterialLoader,
        shader::{loader::ShaderLoader, Shader, ShaderResource, ShaderResourceExtension},
        Material,
    },
    plugin::{
        AbstractDynamicPlugin, Plugin, PluginContainer, PluginContext,
        PluginRegistrationContext,
    },
    renderer::{framework::error::FrameworkError, Renderer},
    resource::{
        curve::{loader::CurveLoader, CurveResourceState},
        model::{loader::ModelLoader, Model, ModelResource},
        texture::{self, loader::TextureLoader, Texture, TextureKind},
    },
    scene::{
        base::NodeScriptMessage,
        camera::SkyBoxKind,
        graph::{GraphUpdateSwitches, NodePool},
        mesh::surface::{self, SurfaceData, SurfaceDataLoader},
        navmesh,
        node::{constructor::NodeConstructorContainer, Node},
        sound::SoundEngine,
        tilemap::{
            brush::{TileMapBrush, TileMapBrushLoader},
            tileset::{TileSet, TileSetLoader},
        },
        Scene, SceneContainer, SceneLoader,
    },
    script::{
        constructor::ScriptConstructorContainer, RoutingStrategy, Script, ScriptContext,
        ScriptDeinitContext, ScriptMessage, ScriptMessageContext, ScriptMessageKind,
        ScriptMessageSender,
    },
    script::{PluginsRefMut, UniversalScriptContext},
    window::{Window, WindowBuilder},
};
use fxhash::{FxHashMap, FxHashSet};
use fyrox_sound::{
    buffer::{loader::SoundBufferLoader, SoundBuffer},
    renderer::hrtf::{HrirSphereLoader, HrirSphereResourceData},
};
use std::{
    any::TypeId,
    collections::{HashSet, VecDeque},
    fmt::{Display, Formatter},
    io::Cursor,
    ops::{Deref, DerefMut},
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
/// use fyrox_impl::{
///     core::{color::Color, visitor::prelude::*, reflect::prelude::*, log::Log, pool::Handle},
///     plugin::{Plugin, PluginContext},
///     scene::Scene,
/// };
/// use std::path::Path;
///
/// #[derive(Visit, Reflect, Debug)]
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

    /// Current resource manager. Resource manager can be cloned (it does clone only ref) to be able to
    /// use resource manager from any thread, this is useful to load resources from multiple
    /// threads to decrease loading times of your game by utilizing all available power of
    /// your CPU.
    pub resource_manager: ResourceManager,

    /// All available user interfaces in the engine.
    pub user_interfaces: UiContainer,

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

    // A set of plugins used by the engine.
    plugins: Vec<PluginContainer>,

    plugins_enabled: bool,

    // Amount of time (in seconds) that passed from creation of the engine.
    elapsed_time: f32,

    /// A special container that is able to create nodes by their type UUID. Use a copy of this
    /// value whenever you need it as a parameter in other parts of the engine.
    pub serialization_context: Arc<SerializationContext>,

    /// A container with widget constructors.
    pub widget_constructors: Arc<WidgetConstructorContainer>,

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
        scene_handle: Handle<Scene>,
        plugins: &mut [PluginContainer],
        resource_manager: &ResourceManager,
        dt: f32,
        elapsed_time: f32,
        message_sender: &ScriptMessageSender,
        user_interfaces: &mut UiContainer,
        graphics_context: &mut GraphicsContext,
        task_pool: &mut TaskPoolHandler,
    ) {
        while let Ok(message) = self.message_receiver.try_recv() {
            let receivers = self.type_groups.get(&message.payload.deref().type_id());

            if receivers.map_or(true, |r| r.is_empty()) {
                Log::warn(format!(
                    "Script message {message:?} was sent, but there's no receivers. \
                    Did you forgot to subscribe your script to the message?"
                ));
            }

            if let Some(receivers) = receivers {
                let mut payload = message.payload;

                match message.kind {
                    ScriptMessageKind::Targeted(target) => {
                        if receivers.contains(&target) {
                            let mut context = ScriptMessageContext {
                                dt,
                                elapsed_time,
                                plugins: PluginsRefMut(plugins),
                                handle: target,
                                scene,
                                scene_handle,
                                resource_manager,
                                message_sender,
                                task_pool,
                                graphics_context,
                                user_interfaces,
                                script_index: 0,
                            };

                            process_node_scripts(&mut context, &mut |s, ctx| {
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
                                    plugins: PluginsRefMut(plugins),
                                    handle: node,
                                    scene,
                                    scene_handle,
                                    resource_manager,
                                    message_sender,
                                    task_pool,
                                    graphics_context,
                                    user_interfaces,
                                    script_index: 0,
                                };

                                if receivers.contains(&node) {
                                    process_node_scripts(&mut context, &mut |s, ctx| {
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
                                    plugins: PluginsRefMut(plugins),
                                    handle: node,
                                    scene,
                                    scene_handle,
                                    resource_manager,
                                    message_sender,
                                    task_pool,
                                    graphics_context,
                                    user_interfaces,
                                    script_index: 0,
                                };

                                if receivers.contains(&node) {
                                    process_node_scripts(&mut context, &mut |s, ctx| {
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
                                plugins: PluginsRefMut(plugins),
                                handle: node,
                                scene,
                                scene_handle,
                                resource_manager,
                                message_sender,
                                task_pool,
                                graphics_context,
                                user_interfaces,
                                script_index: 0,
                            };

                            process_node_scripts(&mut context, &mut |s, ctx| {
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
        plugins: &mut [PluginContainer],
        resource_manager: &ResourceManager,
        task_pool: &mut TaskPoolHandler,
        graphics_context: &mut GraphicsContext,
        user_interfaces: &mut UiContainer,
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
            let script_message_sender = scene.graph.script_message_sender.clone();
            for (handle, node) in scene.graph.pair_iter_mut() {
                // Remove unused script entries.
                node.scripts
                    .retain(|e| e.script.is_some() && !e.should_be_deleted);

                if node.is_globally_enabled() {
                    for (i, entry) in node.scripts.iter().enumerate() {
                        if let Some(script) = entry.script.as_ref() {
                            if script.initialized {
                                if script.started {
                                    update_queue.push_back((handle, i));
                                } else {
                                    start_queue.push_back((handle, i));
                                }
                            } else {
                                script_message_sender
                                    .send(NodeScriptMessage::InitializeScript {
                                        handle,
                                        script_index: i,
                                    })
                                    .unwrap();
                            }
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
                    plugins: PluginsRefMut(plugins),
                    handle: Default::default(),
                    scene,
                    scene_handle: scripted_scene.handle,
                    resource_manager,
                    message_sender: &scripted_scene.message_sender,
                    message_dispatcher: &mut scripted_scene.message_dispatcher,
                    task_pool,
                    graphics_context,
                    user_interfaces,
                    script_index: 0,
                };

                'init_loop: for init_loop_iteration in 0..max_iterations {
                    // Process events first. `on_init` of a script can also create some other instances
                    // and these will be correctly initialized on current frame.
                    while let Ok(event) = context.scene.graph.script_message_receiver.try_recv() {
                        match event {
                            NodeScriptMessage::InitializeScript {
                                handle,
                                script_index,
                            } => {
                                context.handle = handle;
                                context.script_index = script_index;

                                process_node_script(
                                    script_index,
                                    &mut context,
                                    &mut |script, context| {
                                        if !script.initialized {
                                            script.on_init(context);
                                            script.initialized = true;
                                        }

                                        // `on_start` must be called even if the script was initialized.
                                        start_queue.push_back((handle, script_index));
                                    },
                                );
                            }
                            NodeScriptMessage::DestroyScript {
                                handle,
                                script,
                                script_index,
                            } => {
                                // Destruction is delayed to the end of the frame.
                                destruction_queue.push_back((handle, script, script_index));
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
                        while let Some((handle, script_index)) = start_queue.pop_front() {
                            context.handle = handle;
                            context.script_index = script_index;

                            process_node_script(
                                script_index,
                                &mut context,
                                &mut |script, context| {
                                    if script.initialized && !script.started {
                                        script.on_start(context);
                                        script.started = true;

                                        update_queue.push_back((handle, script_index));
                                    }
                                },
                            );
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
                    while let Some((handle, script_index)) = update_queue.pop_front() {
                        context.handle = handle;
                        context.script_index = script_index;

                        process_node_script(script_index, &mut context, &mut |script, context| {
                            script.on_update(context);
                        });
                    }
                }

                if update_loop_iteration == max_iterations - 1 {
                    Log::warn(
                        "Infinite update loop detected! Most likely some of \
                    your scripts causing infinite prefab instantiation!",
                    )
                }
            }

            // Dispatch script messages only when everything is initialized and updated. This has to
            // be done this way, because all those methods could spawn new messages. However, if a new
            // message is spawned directly in `on_message` the dispatcher will correctly handle it
            // on this frame, since it will be placed in the common queue anyway.
            scripted_scene.message_dispatcher.dispatch_messages(
                scene,
                scripted_scene.handle,
                plugins,
                resource_manager,
                dt,
                elapsed_time,
                &scripted_scene.message_sender,
                user_interfaces,
                graphics_context,
                task_pool,
            );

            // As the last step, destroy queued scripts.
            let mut context = ScriptDeinitContext {
                elapsed_time,
                plugins: PluginsRefMut(plugins),
                resource_manager,
                scene,
                scene_handle: scripted_scene.handle,
                node_handle: Default::default(),
                message_sender: &scripted_scene.message_sender,
                user_interfaces,
                graphics_context,
                task_pool,
                script_index: 0,
            };
            while let Some((handle, mut script, index)) = destruction_queue.pop_front() {
                context.node_handle = handle;
                context.script_index = index;

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
                    plugins: PluginsRefMut(plugins),
                    resource_manager,
                    scene: &mut detached_scene,
                    scene_handle: scripted_scene.handle,
                    node_handle: Default::default(),
                    message_sender: &scripted_scene.message_sender,
                    task_pool,
                    graphics_context,
                    user_interfaces,
                    script_index: 0,
                };

                // Destroy every script instance from nodes that were still alive.
                for node_index in 0..context.scene.graph.capacity() {
                    let handle_node = context.scene.graph.handle_from_index(node_index);
                    context.node_handle = handle_node;

                    process_node_scripts(&mut context, &mut |script, context| {
                        if script.initialized {
                            script.on_deinit(context)
                        }
                    });
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

    /// Amount of samples for MSAA. Must be a power of two (1, 2, 4, 8). `None` means disabled.
    /// MSAA works only for forward rendering and does not work for deferred rendering.
    pub msaa_sample_count: Option<u8>,
}

impl Default for GraphicsContextParams {
    fn default() -> Self {
        Self {
            window_attributes: Default::default(),
            vsync: true,
            msaa_sample_count: None,
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
    /// A container with widget constructors.
    pub widget_constructors: Arc<WidgetConstructorContainer>,
    /// A resource manager.
    pub resource_manager: ResourceManager,
    /// Task pool for asynchronous task management.
    pub task_pool: Arc<TaskPool>,
}

fn process_node_script<T, C>(index: usize, context: &mut C, func: &mut T) -> bool
where
    T: FnMut(&mut Script, &mut C),
    C: UniversalScriptContext,
{
    let Some(node) = context.node() else {
        // A node was destroyed.
        return false;
    };

    if !node.is_globally_enabled() {
        return false;
    }

    let Some(entry) = node.scripts.get_mut(index) else {
        // All scripts were visited.
        return false;
    };

    let Some(mut script) = entry.take() else {
        return false;
    };

    func(&mut script, context);

    match context.node() {
        Some(node) => {
            let entry = node
                .scripts
                .get_mut(index)
                .expect("Scripts array cannot be modified!");

            if entry.should_be_deleted {
                context.destroy_script_deferred(script, index);
            } else {
                // Put the script back at its place.
                entry.script = Some(script);
            }
        }
        None => {
            // If the node was deleted by the `func` call, we must send the script to destruction
            // queue, not silently drop it.
            context.destroy_script_deferred(script, index);
        }
    }

    true
}

fn process_node_scripts<T, C>(context: &mut C, func: &mut T)
where
    T: FnMut(&mut Script, &mut C),
    C: UniversalScriptContext,
{
    let mut index = 0;
    loop {
        context.set_script_index(index);

        if !process_node_script(index, context, func) {
            return;
        }

        // Advance to the next script.
        index += 1;
    }
}

pub(crate) fn process_scripts<T>(
    scene: &mut Scene,
    scene_handle: Handle<Scene>,
    plugins: &mut [PluginContainer],
    resource_manager: &ResourceManager,
    message_sender: &ScriptMessageSender,
    message_dispatcher: &mut ScriptMessageDispatcher,
    task_pool: &mut TaskPoolHandler,
    graphics_context: &mut GraphicsContext,
    user_interfaces: &mut UiContainer,
    dt: f32,
    elapsed_time: f32,
    mut func: T,
) where
    T: FnMut(&mut Script, &mut ScriptContext),
{
    let mut context = ScriptContext {
        dt,
        elapsed_time,
        plugins: PluginsRefMut(plugins),
        handle: Default::default(),
        scene,
        scene_handle,
        resource_manager,
        message_sender,
        message_dispatcher,
        task_pool,
        graphics_context,
        user_interfaces,
        script_index: 0,
    };

    for node_index in 0..context.scene.graph.capacity() {
        context.handle = context.scene.graph.handle_from_index(node_index);

        process_node_scripts(&mut context, &mut func);
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

    #[cfg(feature = "gltf")]
    {
        let gltf_loader = super::resource::gltf::GltfLoader {
            resource_manager: resource_manager.clone(),
            default_import_options: Default::default(),
        };
        state.loaders.set(gltf_loader);
    }

    for shader in ShaderResource::standard_shaders() {
        state.built_in_resources.add((*shader).clone());
    }

    for texture in SkyBoxKind::built_in_skybox_textures() {
        state.built_in_resources.add(texture.clone());
    }

    state.built_in_resources.add(BUILT_IN_FONT.clone());

    state.built_in_resources.add(texture::PLACEHOLDER.clone());

    for material in [
        &*material::STANDARD,
        &*material::STANDARD_2D,
        &*material::STANDARD_SPRITE,
        &*material::STANDARD_TERRAIN,
        &*material::STANDARD_TWOSIDES,
        &*material::STANDARD_PARTICLE_SYSTEM,
    ] {
        state.built_in_resources.add(material.clone());
    }

    for surface in [
        &*surface::CUBE,
        &*surface::QUAD,
        &*surface::CYLINDER,
        &*surface::SPHERE,
        &*surface::CONE,
        &*surface::TORUS,
    ] {
        state.built_in_resources.add(surface.clone());
    }

    state.constructors_container.add::<Texture>();
    state.constructors_container.add::<Shader>();
    state.constructors_container.add::<Model>();
    state.constructors_container.add::<CurveResourceState>();
    state.constructors_container.add::<SoundBuffer>();
    state.constructors_container.add::<HrirSphereResourceData>();
    state.constructors_container.add::<Material>();
    state.constructors_container.add::<Font>();
    state.constructors_container.add::<UserInterface>();
    state.constructors_container.add::<SurfaceData>();
    state.constructors_container.add::<TileSet>();
    state.constructors_container.add::<TileMapBrush>();

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
    loaders.set(SurfaceDataLoader {});
    loaders.set(TileSetLoader {
        resource_manager: resource_manager.clone(),
    });
    state.loaders.set(TileMapBrushLoader {});
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
    /// # use fyrox_impl::{
    /// #     asset::manager::ResourceManager,
    /// #     engine::{
    /// #         Engine, EngineInitParams, GraphicsContextParams,
    /// #         SerializationContext,
    /// #     },
    /// #     event_loop::EventLoop,
    /// #     window::WindowAttributes,
    /// # };
    /// # use std::sync::Arc;
    /// # use fyrox_core::task::TaskPool;
    ///
    /// let mut window_attributes = WindowAttributes::default();
    /// window_attributes.title = "Some title".to_string();
    /// let graphics_context_params = GraphicsContextParams {
    ///     window_attributes,
    ///     vsync: true,
    ///     msaa_sample_count: None
    /// };
    /// let task_pool = Arc::new(TaskPool::new());
    ///
    /// Engine::new(EngineInitParams {
    ///     graphics_context_params,
    ///     resource_manager: ResourceManager::new(task_pool.clone()),
    ///     serialization_context: Arc::new(SerializationContext::new()),
    ///     task_pool,
    ///     widget_constructors: Arc::new(Default::default()),
    /// })
    /// .unwrap();
    /// ```
    #[inline]
    #[allow(unused_variables)]
    pub fn new(params: EngineInitParams) -> Result<Self, EngineError> {
        let EngineInitParams {
            graphics_context_params,
            serialization_context,
            widget_constructors,
            resource_manager,
            task_pool,
        } = params;

        initialize_resource_manager_loaders(&resource_manager, serialization_context.clone());

        let (rx, tx) = channel();
        resource_manager.state().event_broadcaster.add(rx);

        let sound_engine = SoundEngine::without_device();

        let user_interfaces =
            UiContainer::new_with_ui(UserInterface::new(Vector2::new(100.0, 100.0)));

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
            user_interfaces,
            performance_statistics: Default::default(),
            plugins: Default::default(),
            serialization_context,
            widget_constructors,
            script_processor: Default::default(),
            plugins_enabled: false,
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

            let (window, renderer) = Renderer::new(
                &self.resource_manager,
                params,
                window_target,
                window_builder,
            )?;

            for ui in self.user_interfaces.iter_mut() {
                ui.set_screen_size(Vector2::new(
                    window.inner_size().width as f32,
                    window.inner_size().height as f32,
                ));
            }

            self.graphics_context = GraphicsContext::Initialized(InitializedGraphicsContext {
                renderer,
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
            window_attributes.preferred_theme = params.window_attributes.preferred_theme;
            window_attributes.resize_increments = window.resize_increments().map(Size::Physical);
            window_attributes.content_protected = params.window_attributes.content_protected;
            window_attributes.window_level = params.window_attributes.window_level;
            window_attributes.active = params.window_attributes.active;
            window_attributes
                .window_icon
                .clone_from(&params.window_attributes.window_icon);

            self.graphics_context = GraphicsContext::Uninitialized(GraphicsContextParams {
                window_attributes,
                vsync: params.vsync,
                msaa_sample_count: params.msaa_sample_count,
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
        self.post_update(dt, &Default::default());
        self.handle_plugins_hot_reloading(dt, window_target, lag, |_| {});
    }

    /// Tries to hot-reload dynamic plugins marked for reloading.
    ///
    /// ## Platform-specific
    ///
    /// - Windows, Unix-like systems (Linux, macOS, FreeBSD, etc) - fully supported.
    /// - WebAssembly - not supported
    /// - Android - not supported
    pub fn handle_plugins_hot_reloading<F>(
        &mut self,
        #[allow(unused_variables)] dt: f32,
        #[allow(unused_variables)] window_target: &EventLoopWindowTarget<()>,
        #[allow(unused_variables)] lag: &mut f32,
        #[allow(unused_variables)] on_reloaded: F,
    ) where
        F: FnMut(&dyn Plugin),
    {
        #[cfg(any(unix, windows))]
        {
            if let Err(message) = self.reload_dynamic_plugins(dt, window_target, lag, on_reloaded) {
                Log::err(format!(
                    "Unable to reload dynamic plugins. Reason: {message}"
                ))
            }
        }
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
                            user_interfaces: &mut self.user_interfaces,
                            serialization_context: &self.serialization_context,
                            widget_constructors: &self.widget_constructors,
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
                    user_interfaces: &mut self.user_interfaces,
                    serialization_context: &self.serialization_context,
                    widget_constructors: &self.widget_constructors,
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
                            let model = Resource::new_ok(
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
                                            &mut |_, _| {},
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
                                &[
                                    TypeId::of::<UntypedResource>(),
                                    TypeId::of::<navmesh::Container>(),
                                ],
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
                                    &[
                                        TypeId::of::<NodePool>(),
                                        TypeId::of::<UntypedResource>(),
                                        TypeId::of::<navmesh::Container>(),
                                    ],
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
        self.resource_manager.state().update(dt);
        self.handle_model_events();

        let window_size = if let GraphicsContext::Initialized(ctx) = &mut self.graphics_context {
            let inner_size = ctx.window.inner_size();
            let window_size = Vector2::new(inner_size.width as f32, inner_size.height as f32);
            ctx.renderer.update_caches(dt);
            window_size
        } else {
            Vector2::new(1.0, 1.0)
        };

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

    /// Performs post update for the engine.
    ///
    /// Normally, this is called from `Engine::update()`.
    /// You should only call this manually if you don't use that method.
    pub fn post_update(&mut self, dt: f32, ui_update_switches: &UiUpdateSwitches) {
        if let GraphicsContext::Initialized(ref ctx) = self.graphics_context {
            let inner_size = ctx.window.inner_size();
            let window_size = Vector2::new(inner_size.width as f32, inner_size.height as f32);

            let time = instant::Instant::now();
            for ui in self.user_interfaces.iter_mut() {
                ui.update(window_size, dt, ui_update_switches);
            }
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

        self.script_processor.handle_scripts(
            &mut self.scenes,
            &mut self.plugins,
            &self.resource_manager,
            &mut self.task_pool,
            &mut self.graphics_context,
            &mut self.user_interfaces,
            dt,
            self.elapsed_time,
        );

        self.performance_statistics.scripts_time = instant::Instant::now() - time;
    }

    fn handle_async_tasks(
        &mut self,
        dt: f32,
        window_target: &EventLoopWindowTarget<()>,
        lag: &mut f32,
    ) {
        while let Some(result) = self.task_pool.inner().next_task_result() {
            if let Some(plugin_task_handler) = self.task_pool.pop_plugin_task_handler(result.id) {
                // Handle plugin task.
                (plugin_task_handler)(
                    result.payload,
                    &mut self.plugins,
                    &mut PluginContext {
                        scenes: &mut self.scenes,
                        resource_manager: &self.resource_manager,
                        graphics_context: &mut self.graphics_context,
                        dt,
                        lag,
                        user_interfaces: &mut self.user_interfaces,
                        serialization_context: &self.serialization_context,
                        widget_constructors: &self.widget_constructors,
                        performance_statistics: &self.performance_statistics,
                        elapsed_time: self.elapsed_time,
                        script_processor: &self.script_processor,
                        async_scene_loader: &mut self.async_scene_loader,
                        window_target: Some(window_target),
                        task_pool: &mut self.task_pool,
                    },
                )
            } else if let Some(node_task_handler) = self.task_pool.pop_node_task_handler(result.id)
            {
                // Handle script task.
                if let Some(scripted_scene) = self
                    .script_processor
                    .scripted_scenes
                    .iter_mut()
                    .find(|e| e.handle == node_task_handler.scene_handle)
                {
                    let payload = result.payload;
                    if let Some(scene) = self.scenes.try_get_mut(node_task_handler.scene_handle) {
                        if let Some(node) = scene.graph.try_get_mut(node_task_handler.node_handle) {
                            if let Some(mut script) = node
                                .scripts
                                .get_mut(node_task_handler.script_index)
                                .and_then(|e| e.script.take())
                            {
                                (node_task_handler.closure)(
                                    payload,
                                    script.deref_mut(),
                                    &mut ScriptContext {
                                        dt,
                                        elapsed_time: self.elapsed_time,
                                        plugins: PluginsRefMut(&mut self.plugins),
                                        handle: node_task_handler.node_handle,
                                        scene,
                                        scene_handle: scripted_scene.handle,
                                        resource_manager: &self.resource_manager,
                                        message_sender: &scripted_scene.message_sender,
                                        message_dispatcher: &mut scripted_scene.message_dispatcher,
                                        task_pool: &mut self.task_pool,
                                        graphics_context: &mut self.graphics_context,
                                        user_interfaces: &mut self.user_interfaces,
                                        script_index: node_task_handler.script_index,
                                    },
                                );

                                if let Some(node) =
                                    scene.graph.try_get_mut(node_task_handler.node_handle)
                                {
                                    if let Some(entry) =
                                        node.scripts.get_mut(node_task_handler.script_index)
                                    {
                                        if entry.should_be_deleted {
                                            Log::verify(scene.graph.script_message_sender.send(
                                                NodeScriptMessage::DestroyScript {
                                                    script,
                                                    handle: node_task_handler.node_handle,
                                                    script_index: node_task_handler.script_index,
                                                },
                                            ));
                                        } else {
                                            entry.script = Some(script);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
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
            self.handle_async_tasks(dt, window_target, lag);

            // Then update all the plugins.
            let mut context = PluginContext {
                scenes: &mut self.scenes,
                resource_manager: &self.resource_manager,
                graphics_context: &mut self.graphics_context,
                dt,
                lag,
                user_interfaces: &mut self.user_interfaces,
                serialization_context: &self.serialization_context,
                widget_constructors: &self.widget_constructors,
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

            let mut uis = self
                .user_interfaces
                .pair_iter()
                .map(|(h, _)| h)
                .collect::<VecDeque<_>>();

            while let Some(ui) = uis.pop_front() {
                while let Some(message) = self
                    .user_interfaces
                    .try_get_mut(ui)
                    .and_then(|ui| ui.poll_message())
                {
                    let mut context = PluginContext {
                        scenes: &mut self.scenes,
                        resource_manager: &self.resource_manager,
                        graphics_context: &mut self.graphics_context,
                        dt,
                        lag,
                        user_interfaces: &mut self.user_interfaces,
                        serialization_context: &self.serialization_context,
                        widget_constructors: &self.widget_constructors,
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
                        user_interfaces: &mut self.user_interfaces,
                        serialization_context: &self.serialization_context,
                        widget_constructors: &self.widget_constructors,
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
                    user_interfaces: &mut self.user_interfaces,
                    serialization_context: &self.serialization_context,
                    widget_constructors: &self.widget_constructors,
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
                    user_interfaces: &mut self.user_interfaces,
                    serialization_context: &self.serialization_context,
                    widget_constructors: &self.widget_constructors,
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
                    user_interfaces: &mut self.user_interfaces,
                    serialization_context: &self.serialization_context,
                    widget_constructors: &self.widget_constructors,
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
                    &mut self.user_interfaces,
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
        for ui in self.user_interfaces.iter_mut() {
            ui.draw();
        }

        if let GraphicsContext::Initialized(ref mut ctx) = self.graphics_context {
            ctx.renderer.render_and_swap_buffers(
                &self.scenes,
                self.user_interfaces
                    .iter()
                    .map(|ui| ui.get_drawing_context()),
                &ctx.window,
            )?;
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
                for plugin in self.plugins.iter_mut() {
                    plugin.init(
                        scene_path,
                        PluginContext {
                            scenes: &mut self.scenes,
                            resource_manager: &self.resource_manager,
                            graphics_context: &mut self.graphics_context,
                            dt: 0.0,
                            lag: &mut 0.0,
                            user_interfaces: &mut self.user_interfaces,
                            serialization_context: &self.serialization_context,
                            widget_constructors: &self.widget_constructors,
                            performance_statistics: &self.performance_statistics,
                            elapsed_time: self.elapsed_time,
                            script_processor: &self.script_processor,
                            async_scene_loader: &mut self.async_scene_loader,
                            window_target,
                            task_pool: &mut self.task_pool,
                        },
                    );
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
                        user_interfaces: &mut self.user_interfaces,
                        serialization_context: &self.serialization_context,
                        widget_constructors: &self.widget_constructors,
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

    fn register_plugin_internal(
        serialization_context: &Arc<SerializationContext>,
        widget_constructors: &Arc<WidgetConstructorContainer>,
        resource_manager: &ResourceManager,
        plugin: &dyn Plugin,
    ) {
        *widget_constructors.context_type_id.lock() = plugin.type_id();
        plugin.register(PluginRegistrationContext {
            serialization_context,
            widget_constructors,
            resource_manager,
        });
    }

    fn register_plugin(&self, plugin: &dyn Plugin) {
        Self::register_plugin_internal(
            &self.serialization_context,
            &self.widget_constructors,
            &self.resource_manager,
            plugin,
        )
    }

    /// Adds a new static plugin.
    pub fn add_plugin<P>(&mut self, plugin: P)
    where
        P: Plugin + 'static,
    {
        self.register_plugin(&plugin);

        self.plugins.push(PluginContainer::Static(Box::new(plugin)));
    }

    /// Adds a new abstract dynamic plugin
    pub fn add_dynamic_plugin<P>(
        &mut self,
        plugin: P,
    ) -> &dyn Plugin
    where
        P: AbstractDynamicPlugin + 'static,
    {
        let display_name = plugin.display_name();

        let plugin_container = PluginContainer::Dynamic(Box::new(plugin));

        self.register_plugin(plugin_container.deref());
        self.plugins.push(plugin_container);

        Log::info(format!(
            "Plugin {:?} was loaded successfully",
            display_name
        ));

        &**self.plugins.last().unwrap()
    }

    /// Tries to reload a specified plugin. This method tries to perform least invasive reloading, by
    /// only detaching parts from the scenes and engine internals, that belongs to reloadable plugin.
    pub fn reload_plugin(
        &mut self,
        plugin_index: usize,
        dt: f32,
        window_target: &EventLoopWindowTarget<()>,
        lag: &mut f32,
    ) -> Result<(), String> {
        let plugin_container = &mut self.plugins[plugin_index];
        let PluginContainer::Dynamic(plugin) = plugin_container
        else {
            return Err(format!(
                "Plugin {plugin_index} is static and cannot be reloaded!",
            ));
        };

        if !plugin.is_loaded() {
            // TODO: this means that something bad happened during plugin reloading.
            // don't we want to recover from this situation by trying to load it again 
            // (maybe with clearing  `need_reload` flag, to perform new attempt only when something is changed)
            return Err(format!("Cannot reload unloaded plugin {plugin_index}!"));
        }

        let plugin_type_id = plugin.as_loaded_ref().type_id();
        let plugin_assembly_name = plugin.as_loaded_ref().assembly_name();

        // Collect all the data that belongs to the plugin
        let mut scenes_state = Vec::new();
        for (scene_handle, scene) in self.scenes.pair_iter_mut() {
            if let Some(data) = hotreload::SceneState::try_create_from_plugin(
                scene_handle,
                scene,
                &self.serialization_context,
                plugin.as_loaded_ref(),
            )? {
                scenes_state.push(data);
            }
        }

        // Check every prefab for plugin content.
        let mut prefab_scenes = Vec::new();
        let rm_state = self.resource_manager.state();
        for resource in rm_state.resources().iter() {
            if let Some(model) = resource.try_cast::<Model>() {
                let mut model_state = model.state();
                if let Some(data) = model_state.data() {
                    if let Some(scene_state) = hotreload::SceneState::try_create_from_plugin(
                        Handle::NONE,
                        &mut data.scene,
                        &self.serialization_context,
                        plugin.as_loaded_ref(),
                    )? {
                        prefab_scenes.push((model.clone(), scene_state));
                    }
                }
            }
        }
        drop(rm_state);

        // Search for script constructors, that belongs to dynamic plugins and remove them.
        let mut constructors = FxHashSet::default();
        for (type_uuid, constructor) in self.serialization_context.script_constructors.map().iter()
        {
            if constructor.assembly_name == plugin_assembly_name {
                constructors.insert(*type_uuid);
            }
        }
        for type_uuid in constructors.iter() {
            self.serialization_context
                .script_constructors
                .remove(*type_uuid);
        }

        // Search for node constructors, that belongs to dynamic plugins and remove them.
        let mut constructors = FxHashSet::default();
        for (type_uuid, constructor) in self.serialization_context.node_constructors.map().iter() {
            if constructor.assembly_name == plugin_assembly_name {
                constructors.insert(*type_uuid);
            }
        }
        for type_uuid in constructors.iter() {
            self.serialization_context
                .node_constructors
                .remove(*type_uuid);
        }

        // Search for widget constructors, that belongs to dynamic plugins and remove them.
        let mut constructors = FxHashSet::default();
        for (type_uuid, constructor) in self.widget_constructors.map().iter() {
            if constructor.assembly_name == plugin_assembly_name {
                constructors.insert(*type_uuid);
            }
        }
        for type_uuid in constructors.iter() {
            self.widget_constructors.remove(*type_uuid);
        }

        // Reload resources, that belongs to the plugin.
        {
            let mut resources_to_reload = FxHashSet::default();
            let mut state = self.resource_manager.state();
            for resource in state.resources().iter() {
                let data = resource.0.lock();
                if let ResourceState::Ok(ref data) = data.state {
                    data.as_reflect(&mut |reflect| {
                        if reflect.assembly_name() == plugin_assembly_name {
                            resources_to_reload.insert(resource.clone());
                        }
                    })
                }
            }

            for resource_to_reload in resources_to_reload.iter() {
                Log::info(format!(
                    "Reloading {} resource, because it is used in plugin {plugin_assembly_name}",
                    resource_to_reload.kind()
                ));

                state.reload_resource(resource_to_reload.clone());
            }

            drop(state);

            block_on(join_all(resources_to_reload));
        }

        // Unload custom render passes (if any).
        if let GraphicsContext::Initialized(ref mut graphics_context) = self.graphics_context {
            let render_passes = graphics_context.renderer.render_passes().to_vec();
            for render_pass in render_passes {
                if render_pass.borrow().source_type_id() == plugin_type_id {
                    graphics_context.renderer.remove_render_pass(render_pass);
                }
            }
        }

        let mut visitor = hotreload::make_writing_visitor();
        plugin
            .as_loaded_mut()
            .visit("Plugin", &mut visitor)
            .map_err(|e| e.to_string())?;
        let mut binary_blob = Cursor::new(Vec::<u8>::new());
        visitor
            .save_binary_to_memory(&mut binary_blob)
            .map_err(|e| e.to_string())?;

        Log::info(format!(
            "Plugin {plugin_index} was serialized successfully!"
        ));

        // Explicitly drop the visitor to prevent any destructors from the previous version of
        // the plugin to run at the end of the scope. This could happen, because the visitor
        // manages serialized smart pointers and if they'll be kept alive longer than the plugin
        // there's a very high chance of hard crash.
        drop(visitor);

        let binary_blob = binary_blob.into_inner();

        plugin.reload(&mut |plugin| {
            // Re-register the plugin. This is needed, because it might contain new script/node/widget
            // types (or removed ones too). This is done right before deserialization, because plugin
            // might contain some entities, that have dynamic registration.
            Self::register_plugin_internal(
                &self.serialization_context,
                &self.widget_constructors,
                &self.resource_manager,
                plugin,
            );

            let mut visitor = hotreload::make_reading_visitor(
                &binary_blob,
                &self.serialization_context,
                &self.resource_manager,
                &self.widget_constructors,
            )
            .map_err(|e| e.to_string())?;

            plugin
                .visit("Plugin", &mut visitor)
                .map_err(|e| e.to_string())?;
            Ok(())
        })?;

        // Deserialize prefab scene content.
        for (model, scene_state) in prefab_scenes {
            Log::info(format!("Deserializing {} prefab content...", model.kind()));

            scene_state.deserialize_into_prefab_scene(
                &model,
                &self.serialization_context,
                &self.resource_manager,
                &self.widget_constructors,
            )?;
        }

        // Deserialize scene content.
        for scene_state in scenes_state {
            let scene = &mut self.scenes[scene_state.scene];
            scene_state.deserialize_into_scene(
                scene,
                &self.serialization_context,
                &self.resource_manager,
                &self.widget_constructors,
            )?;
        }

        // Call `on_loaded` for plugins, so they could restore some runtime non-serializable state.
        plugin.as_loaded_mut().on_loaded(PluginContext {
            scenes: &mut self.scenes,
            resource_manager: &self.resource_manager,
            user_interfaces: &mut self.user_interfaces,
            graphics_context: &mut self.graphics_context,
            dt,
            lag,
            serialization_context: &self.serialization_context,
            widget_constructors: &self.widget_constructors,
            performance_statistics: &Default::default(),
            elapsed_time: self.elapsed_time,
            script_processor: &self.script_processor,
            async_scene_loader: &mut self.async_scene_loader,
            window_target: Some(window_target),
            task_pool: &mut self.task_pool,
        });

        Log::info(format!(
            "Plugin {} was successfully reloaded!",
            plugin_index
        ));

        Ok(())
    }

    /// Returns a reference to the plugins.
    pub fn plugins(&self) -> &[PluginContainer] {
        &self.plugins
    }

    /// Tries to reload all dynamic plugins registered in the engine, that needs to be reloaded.
    pub fn reload_dynamic_plugins<F>(
        &mut self,
        dt: f32,
        window_target: &EventLoopWindowTarget<()>,
        lag: &mut f32,
        mut on_reloaded: F,
    ) -> Result<(), String>
    where
        F: FnMut(&dyn Plugin),
    {
        for plugin_index in 0..self.plugins.len() {
            if let PluginContainer::Dynamic(plugin) = &self.plugins[plugin_index]
            {
                if plugin.is_reload_needed_now() {
                    self.reload_plugin(plugin_index, dt, window_target, lag)?;

                    on_reloaded(self.plugins[plugin_index].deref_mut());
                }
            }
        }

        Ok(())
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
            pool::Handle, reflect::prelude::*, task::TaskPool, type_traits::prelude::*,
            visitor::prelude::*,
        },
        engine::{task::TaskPoolHandler, GraphicsContext, ScriptProcessor},
        graph::BaseSceneGraph,
        scene::{base::BaseBuilder, node::Node, pivot::PivotBuilder, Scene, SceneContainer},
        script::{
            ScriptContext, ScriptDeinitContext, ScriptMessageContext, ScriptMessagePayload,
            ScriptTrait,
        },
    };
    use fyrox_ui::UiContainer;
    use std::sync::{
        mpsc::{self, Sender, TryRecvError},
        Arc,
    };

    #[derive(PartialEq, Eq, Copy, Clone, Debug)]
    struct Source {
        node_handle: Handle<Node>,
        script_index: usize,
    }

    impl Source {
        fn from_ctx(ctx: &ScriptContext) -> Self {
            Self {
                node_handle: ctx.handle,
                script_index: ctx.script_index,
            }
        }

        fn from_deinit_ctx(ctx: &ScriptDeinitContext) -> Self {
            Self {
                node_handle: ctx.node_handle,
                script_index: ctx.script_index,
            }
        }

        fn from_msg_ctx(ctx: &ScriptMessageContext) -> Self {
            Self {
                node_handle: ctx.handle,
                script_index: ctx.script_index,
            }
        }
    }

    #[allow(clippy::enum_variant_names)]
    #[derive(PartialEq, Eq, Clone, Debug)]
    enum Event {
        Initialized(Source),
        Started(Source),
        Updated(Source),
        Destroyed(Source),
        EventReceived(Source),
    }

    #[derive(Debug, Clone, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
    #[type_uuid(id = "2569de84-d4b2-427d-969b-d5c7b31a0ba6")]
    struct MyScript {
        #[reflect(hidden)]
        #[visit(skip)]
        sender: Sender<Event>,
        spawned: bool,
    }

    impl ScriptTrait for MyScript {
        fn on_init(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Initialized(Source::from_ctx(ctx)))
                .unwrap();

            // Spawn new entity with script.
            let handle = PivotBuilder::new(BaseBuilder::new().with_script(MySubScript {
                sender: self.sender.clone(),
            }))
            .build(&mut ctx.scene.graph);
            assert_eq!(handle, Handle::new(2, 1));
        }

        fn on_start(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Started(Source::from_ctx(ctx)))
                .unwrap();

            // Spawn new entity with script.
            let handle = PivotBuilder::new(BaseBuilder::new().with_script(MySubScript {
                sender: self.sender.clone(),
            }))
            .build(&mut ctx.scene.graph);
            assert_eq!(handle, Handle::new(3, 1));
        }

        fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
            self.sender
                .send(Event::Destroyed(Source::from_deinit_ctx(ctx)))
                .unwrap();
        }

        fn on_update(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Updated(Source::from_ctx(ctx)))
                .unwrap();

            if !self.spawned {
                // Spawn new entity with script.
                PivotBuilder::new(BaseBuilder::new().with_script(MySubScript {
                    sender: self.sender.clone(),
                }))
                .build(&mut ctx.scene.graph);

                self.spawned = true;
            }
        }
    }

    #[derive(Debug, Clone, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
    #[type_uuid(id = "1cebacd9-b500-4753-93be-39db344add21")]
    struct MySubScript {
        #[reflect(hidden)]
        #[visit(skip)]
        sender: Sender<Event>,
    }

    impl ScriptTrait for MySubScript {
        fn on_init(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Initialized(Source::from_ctx(ctx)))
                .unwrap();
        }

        fn on_start(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Started(Source::from_ctx(ctx)))
                .unwrap();
        }

        fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
            self.sender
                .send(Event::Destroyed(Source::from_deinit_ctx(ctx)))
                .unwrap();
        }

        fn on_update(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Updated(Source::from_ctx(ctx)))
                .unwrap();
        }
    }

    #[test]
    fn test_order() {
        let resource_manager = ResourceManager::new(Arc::new(Default::default()));
        let mut scene = Scene::new();

        let (tx, rx) = mpsc::channel();

        let node_handle = PivotBuilder::new(
            BaseBuilder::new()
                .with_script(MyScript {
                    sender: tx.clone(),
                    spawned: false,
                })
                .with_script(MySubScript { sender: tx }),
        )
        .build(&mut scene.graph);
        assert_eq!(node_handle, Handle::new(1, 1));

        let node_handle_0 = Source {
            node_handle,
            script_index: 0,
        };
        let node_handle_1 = Source {
            node_handle,
            script_index: 1,
        };

        let mut scene_container = SceneContainer::new(Default::default());

        let scene_handle = scene_container.add(scene);

        let mut script_processor = ScriptProcessor::default();

        script_processor.register_scripted_scene(scene_handle, &resource_manager);

        let handle_on_init = Source {
            node_handle: Handle::new(2, 1),
            script_index: 0,
        };
        let handle_on_start = Source {
            node_handle: Handle::new(3, 1),
            script_index: 0,
        };
        let handle_on_update1 = Source {
            node_handle: Handle::new(4, 1),
            script_index: 0,
        };
        let mut task_pool = TaskPoolHandler::new(Arc::new(TaskPool::new()));
        let mut gc = GraphicsContext::Uninitialized(Default::default());
        let mut user_interfaces = UiContainer::default();

        for iteration in 0..3 {
            script_processor.handle_scripts(
                &mut scene_container,
                &mut Vec::new(),
                &resource_manager,
                &mut task_pool,
                &mut gc,
                &mut user_interfaces,
                0.0,
                0.0,
            );

            match iteration {
                0 => {
                    assert_eq!(rx.try_recv(), Ok(Event::Initialized(node_handle_0)));
                    assert_eq!(rx.try_recv(), Ok(Event::Initialized(node_handle_1)));
                    assert_eq!(rx.try_recv(), Ok(Event::Initialized(handle_on_init)));
                    assert_eq!(rx.try_recv(), Ok(Event::Started(node_handle_0)));
                    assert_eq!(rx.try_recv(), Ok(Event::Started(node_handle_1)));
                    assert_eq!(rx.try_recv(), Ok(Event::Started(handle_on_init)));
                    assert_eq!(rx.try_recv(), Ok(Event::Initialized(handle_on_start)));
                    assert_eq!(rx.try_recv(), Ok(Event::Started(handle_on_start)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(node_handle_0)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(node_handle_1)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(handle_on_init)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(handle_on_start)));
                    assert_eq!(rx.try_recv(), Ok(Event::Initialized(handle_on_update1)));
                    assert_eq!(rx.try_recv(), Ok(Event::Started(handle_on_update1)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(handle_on_update1)));
                }
                1 => {
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(node_handle_0)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(node_handle_1)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(handle_on_init)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(handle_on_start)));
                    assert_eq!(rx.try_recv(), Ok(Event::Updated(handle_on_update1)));
                    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));

                    // Now destroy every node with script, next iteration should correctly destroy attached scripts.
                    let graph = &mut scene_container[scene_handle].graph;
                    graph.remove_node(node_handle);
                    graph.remove_node(handle_on_init.node_handle);
                    graph.remove_node(handle_on_start.node_handle);
                    graph.remove_node(handle_on_update1.node_handle);
                }
                2 => {
                    assert_eq!(rx.try_recv(), Ok(Event::Destroyed(node_handle_0)));
                    assert_eq!(rx.try_recv(), Ok(Event::Destroyed(node_handle_1)));
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

    #[derive(Debug)]
    enum MyMessage {
        Foo(usize),
        Bar(String),
    }

    #[derive(Debug, Clone, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
    #[type_uuid(id = "bf2976ad-f41d-4de6-9a32-b1a293956058")]
    struct ScriptListeningToMessages {
        index: u32,
        #[reflect(hidden)]
        #[visit(skip)]
        sender: Sender<Event>,
    }

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
                        self.sender
                            .send(Event::EventReceived(Source::from_msg_ctx(ctx)))
                            .unwrap();
                    } else {
                        unreachable!()
                    }
                }
                1 => {
                    if let MyMessage::Bar(string) = typed_message {
                        assert_eq!(string, "Foobar");
                        self.sender
                            .send(Event::EventReceived(Source::from_msg_ctx(ctx)))
                            .unwrap();
                    } else {
                        unreachable!()
                    }
                }
                _ => (),
            }

            self.index += 1;
        }
    }

    #[derive(Debug, Clone, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
    #[type_uuid(id = "6bcbf9b4-9546-42d3-965a-de055ab85475")]
    struct ScriptSendingMessages {
        index: u32,
    }

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

        PivotBuilder::new(BaseBuilder::new().with_script(ScriptSendingMessages { index: 0 }))
            .build(&mut scene.graph);

        let receiver_messages =
            PivotBuilder::new(BaseBuilder::new().with_script(ScriptListeningToMessages {
                sender: tx,
                index: 0,
            }))
            .build(&mut scene.graph);
        let receiver_messages_source = Source {
            node_handle: receiver_messages,
            script_index: 0,
        };

        let mut scene_container = SceneContainer::new(Default::default());

        let scene_handle = scene_container.add(scene);

        let mut script_processor = ScriptProcessor::default();
        let mut task_pool = TaskPoolHandler::new(Arc::new(TaskPool::new()));
        let mut gc = GraphicsContext::Uninitialized(Default::default());
        let mut user_interfaces = UiContainer::default();

        script_processor.register_scripted_scene(scene_handle, &resource_manager);

        for iteration in 0..2 {
            script_processor.handle_scripts(
                &mut scene_container,
                &mut Vec::new(),
                &resource_manager,
                &mut task_pool,
                &mut gc,
                &mut user_interfaces,
                0.0,
                0.0,
            );

            match iteration {
                0 => {
                    assert_eq!(
                        rx.try_recv(),
                        Ok(Event::EventReceived(receiver_messages_source))
                    );
                    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
                }
                1 => {
                    assert_eq!(
                        rx.try_recv(),
                        Ok(Event::EventReceived(receiver_messages_source))
                    );
                    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
                }
                _ => (),
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
    #[type_uuid(id = "7bcbf9b4-9546-42d3-965a-de055ab85475")]
    pub struct ScriptSpawningAsyncTasks {
        num: Option<u32>,
    }

    impl ScriptTrait for ScriptSpawningAsyncTasks {
        fn on_start(&mut self, ctx: &mut ScriptContext) {
            ctx.task_pool.spawn_script_task(
                ctx.scene_handle,
                ctx.handle,
                ctx.script_index,
                async move { 123u32 },
                |result, script: &mut ScriptSpawningAsyncTasks, _ctx| {
                    assert_eq!(result, 123u32);
                    script.num = Some(result);
                },
            )
        }
    }

    #[derive(Clone, Debug, PartialEq, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
    #[type_uuid(id = "8bcbf9b4-9546-42d3-965a-de055ab85475")]
    pub struct ScriptWithoutAsyncTasks {}

    impl ScriptTrait for ScriptWithoutAsyncTasks {}

    #[test]
    #[cfg(not(target_os = "macos"))] // This fails on macOS for some reason.
    fn test_async_script_tasks() {
        use crate::engine::{Engine, EngineInitParams};
        use std::mem::{ManuallyDrop, MaybeUninit};
        use winit::event_loop::EventLoop;
        // This hack is needed, because tests run in random threads and EventLoop cannot be created
        // from non-main thread. Since we don't create any windows and don't run an event loop, this
        // should be safe.
        #[allow(invalid_value)]
        #[allow(clippy::uninit_assumed_init)]
        let event_loop =
            unsafe { ManuallyDrop::new(MaybeUninit::<EventLoop<()>>::uninit().assume_init()) };

        let task_pool = Arc::new(TaskPool::default());
        let mut engine = Engine::new(EngineInitParams {
            graphics_context_params: Default::default(),
            serialization_context: Arc::new(Default::default()),
            widget_constructors: Arc::new(Default::default()),
            resource_manager: ResourceManager::new(task_pool.clone()),
            task_pool,
        })
        .unwrap();
        engine.enable_plugins(None, true, None);

        let mut scene = Scene::new();

        let handle = PivotBuilder::new(
            BaseBuilder::new()
                .with_script(ScriptSpawningAsyncTasks { num: None })
                .with_script(ScriptWithoutAsyncTasks {}),
        )
        .build(&mut scene.graph);

        let scene_handle = engine.scenes.add(scene);

        engine.register_scripted_scene(scene_handle);

        // Spin for some time.
        let mut time = 0.0;
        let dt = 1.0 / 60.0;
        let mut lag = 0.0;
        while time <= 10.0 {
            engine.update(dt, &event_loop, &mut lag, Default::default());
            time += dt;
        }

        // Ensure that the tasks are finished and correctly handled.
        let mut scripts = engine.scenes[scene_handle].graph[handle].scripts();
        assert_eq!(
            scripts
                .next()
                .and_then(|s| s.cast::<ScriptSpawningAsyncTasks>()),
            Some(&ScriptSpawningAsyncTasks { num: Some(123) })
        );
        assert_eq!(
            scripts
                .next()
                .and_then(|s| s.cast::<ScriptWithoutAsyncTasks>()),
            Some(&ScriptWithoutAsyncTasks {})
        );
    }

    #[derive(Clone, Debug, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
    #[type_uuid(id = "9bcbf9b4-9546-42d3-965a-de055ab85475")]
    pub struct ScriptThatDeletesItself {
        #[reflect(hidden)]
        #[visit(skip)]
        sender: Sender<Event>,
    }

    impl ScriptTrait for ScriptThatDeletesItself {
        fn on_init(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Initialized(Source::from_ctx(ctx)))
                .unwrap();
        }

        fn on_start(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Started(Source::from_ctx(ctx)))
                .unwrap();
        }

        fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
            self.sender
                .send(Event::Destroyed(Source::from_deinit_ctx(ctx)))
                .unwrap();
        }

        fn on_update(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Updated(Source::from_ctx(ctx)))
                .unwrap();

            let node = &mut ctx.scene.graph[ctx.handle];
            node.remove_script(ctx.script_index);
        }
    }

    #[derive(Clone, Debug, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
    #[type_uuid(id = "9bcbf9b4-9546-42d3-965a-de055ab85475")]
    pub struct ScriptThatAddsScripts {
        num: usize,
        #[reflect(hidden)]
        #[visit(skip)]
        sender: Sender<Event>,
    }

    impl ScriptTrait for ScriptThatAddsScripts {
        fn on_init(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Initialized(Source::from_ctx(ctx)))
                .unwrap();
        }

        fn on_start(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Started(Source::from_ctx(ctx)))
                .unwrap();

            for i in 0..self.num {
                ctx.scene.graph[ctx.handle].add_script(SimpleScript {
                    stuff: i,
                    sender: self.sender.clone(),
                });
            }
        }

        fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
            self.sender
                .send(Event::Destroyed(Source::from_deinit_ctx(ctx)))
                .unwrap();
        }

        fn on_update(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Updated(Source::from_ctx(ctx)))
                .unwrap();
        }
    }

    #[derive(Clone, Debug, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
    #[type_uuid(id = "9bcbf9b4-9546-42d3-965a-de055ab85475")]
    pub struct SimpleScript {
        stuff: usize,
        #[reflect(hidden)]
        #[visit(skip)]
        sender: Sender<Event>,
    }

    impl ScriptTrait for SimpleScript {
        fn on_init(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Initialized(Source::from_ctx(ctx)))
                .unwrap();
        }

        fn on_start(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Started(Source::from_ctx(ctx)))
                .unwrap();
        }

        fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
            self.sender
                .send(Event::Destroyed(Source::from_deinit_ctx(ctx)))
                .unwrap();
        }

        fn on_update(&mut self, ctx: &mut ScriptContext) {
            self.sender
                .send(Event::Updated(Source::from_ctx(ctx)))
                .unwrap();
        }
    }

    #[test]
    fn test_script_adding_removing() {
        let resource_manager = ResourceManager::new(Arc::new(Default::default()));
        let mut scene = Scene::new();

        let (tx, rx) = mpsc::channel();

        let node_handle = PivotBuilder::new(
            BaseBuilder::new()
                .with_script(ScriptThatDeletesItself { sender: tx.clone() })
                .with_script(ScriptThatAddsScripts { num: 2, sender: tx }),
        )
        .build(&mut scene.graph);
        assert_eq!(node_handle, Handle::new(1, 1));

        let mut scene_container = SceneContainer::new(Default::default());

        let scene_handle = scene_container.add(scene);

        let mut script_processor = ScriptProcessor::default();

        script_processor.register_scripted_scene(scene_handle, &resource_manager);

        let mut task_pool = TaskPoolHandler::new(Arc::new(TaskPool::new()));
        let mut gc = GraphicsContext::Uninitialized(Default::default());
        let mut user_interfaces = UiContainer::default();

        for iteration in 0..2 {
            script_processor.handle_scripts(
                &mut scene_container,
                &mut Vec::new(),
                &resource_manager,
                &mut task_pool,
                &mut gc,
                &mut user_interfaces,
                0.0,
                0.0,
            );

            match iteration {
                0 => {
                    for i in 0..2 {
                        assert_eq!(
                            rx.try_recv(),
                            Ok(Event::Initialized(Source {
                                node_handle,
                                script_index: i,
                            }))
                        );
                    }
                    for i in 0..2 {
                        assert_eq!(
                            rx.try_recv(),
                            Ok(Event::Started(Source {
                                node_handle,
                                script_index: i,
                            }))
                        );
                    }
                    for i in 2..4 {
                        assert_eq!(
                            rx.try_recv(),
                            Ok(Event::Initialized(Source {
                                node_handle,
                                script_index: i,
                            }))
                        );
                    }
                    for i in 2..4 {
                        assert_eq!(
                            rx.try_recv(),
                            Ok(Event::Started(Source {
                                node_handle,
                                script_index: i,
                            }))
                        );
                    }
                    for i in 0..4 {
                        assert_eq!(
                            rx.try_recv(),
                            Ok(Event::Updated(Source {
                                node_handle,
                                script_index: i,
                            }))
                        );
                    }
                    assert_eq!(
                        rx.try_recv(),
                        Ok(Event::Destroyed(Source {
                            node_handle,
                            script_index: 0,
                        }))
                    );

                    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
                }
                1 => {
                    for i in 0..3 {
                        assert_eq!(
                            rx.try_recv(),
                            Ok(Event::Updated(Source {
                                node_handle,
                                script_index: i,
                            }))
                        );
                    }

                    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
                }
                _ => (),
            }
        }
    }
}
