//! Engine is container for all subsystems (renderer, ui, sound, resource manager). It also
//! creates a window and an OpenGL context.

#![allow(missing_docs)] // TODO

pub mod error;
pub mod executor;
pub mod resource_manager;

use crate::{
    asset::ResourceState,
    core::{algebra::Vector2, futures::executor::block_on, instant, pool::Handle},
    engine::{
        error::EngineError,
        resource_manager::{container::event::ResourceEvent, ResourceManager, ResourceWaitContext},
    },
    event::Event,
    event_loop::ControlFlow,
    gui::UserInterface,
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    renderer::{framework::error::FrameworkError, Renderer},
    resource::{model::Model, texture::TextureKind},
    scene::{
        base::NodeScriptMessage,
        graph::GraphUpdateSwitches,
        node::{constructor::NodeConstructorContainer, Node},
        sound::SoundEngine,
        Scene, SceneContainer,
    },
    script::{
        constructor::ScriptConstructorContainer, RoutingStrategy, Script, ScriptContext,
        ScriptDeinitContext, ScriptMessage, ScriptMessageContext, ScriptMessageKind,
        ScriptMessageSender,
    },
    utils::log::Log,
    window::{Window, WindowBuilder},
};
use fxhash::{FxHashMap, FxHashSet};
use glutin::surface::SwapInterval;
#[cfg(not(target_arch = "wasm32"))]
use glutin::{
    config::ConfigTemplateBuilder,
    context::{
        ContextApi, ContextAttributesBuilder, GlProfile, NotCurrentGlContextSurfaceAccessor,
        PossiblyCurrentContext, Version,
    },
    display::{GetGlDisplay, GlDisplay},
    surface::{GlSurface, Surface, WindowSurface},
};
#[cfg(not(target_arch = "wasm32"))]
use glutin_winit::{DisplayBuilder, GlWindow};
#[cfg(not(target_arch = "wasm32"))]
use raw_window_handle::HasRawWindowHandle;
use std::{
    any::TypeId,
    collections::{HashSet, VecDeque},
    fmt::{Display, Formatter},
    ops::Deref,
    sync::{
        mpsc::{channel, Receiver},
        Arc, Mutex,
    },
    time::Duration,
};
#[cfg(not(target_arch = "wasm32"))]
use std::{ffi::CString, num::NonZeroU32};
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowAttributes;

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

pub struct GraphicsContext {
    #[cfg(not(target_arch = "wasm32"))]
    gl_context: PossiblyCurrentContext,
    #[cfg(not(target_arch = "wasm32"))]
    gl_surface: Surface<WindowSurface>,
    pub window: Window,
    /// Current renderer. You should call at least [render](Self::render) method to see your scene on
    /// screen.
    pub renderer: Renderer,
}

/// See module docs.
pub struct Engine {
    pub graphics_context_params: GraphicsContextParams,
    pub graphics_context: Option<GraphicsContext>,
    /// User interface allows you to build interface of any kind.
    pub user_interface: UserInterface,
    /// Current resource manager. Resource manager can be cloned (it does clone only ref) to be able to
    /// use resource manager from any thread, this is useful to load resources from multiple
    /// threads to decrease loading times of your game by utilizing all available power of
    /// your CPU.
    pub resource_manager: ResourceManager,
    /// All available scenes in the engine.
    pub scenes: SceneContainer,

    performance_statistics: PerformanceStatistics,

    model_events_receiver: Receiver<ResourceEvent<Model>>,

    #[allow(dead_code)] // Keep engine instance alive.
    sound_engine: Arc<Mutex<SoundEngine>>,

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

    script_processor: ScriptProcessor,
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

pub(crate) struct ScriptedScene {
    handle: Handle<Scene>,
    message_sender: ScriptMessageSender,
    message_dispatcher: ScriptMessageDispatcher,
}

#[derive(Default)]
struct ScriptProcessor {
    wait_list: Vec<ResourceWaitContext>,
    scripted_scenes: Vec<ScriptedScene>,
}

impl ScriptProcessor {
    fn has_scripted_scene(&self, scene: Handle<Scene>) -> bool {
        self.scripted_scenes.iter().any(|s| s.handle == scene)
    }

    fn register_scripted_scene(
        &mut self,
        scene: Handle<Scene>,
        scenes: &mut SceneContainer,
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

        let graph = &mut scenes[scene].graph;

        // Spawn events for each node in the scene to force the engine to
        // initialize scripts.
        for (handle, _) in graph.pair_iter() {
            graph
                .script_message_sender
                .send(NodeScriptMessage::InitializeScript { handle })
                .unwrap();
        }

        self.wait_list
            .push(resource_manager.state().containers_mut().get_wait_context());
    }

    fn handle_scripts(
        &mut self,
        scenes: &mut SceneContainer,
        plugins: &mut Vec<Box<dyn Plugin>>,
        resource_manager: &ResourceManager,
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
            if !scene.enabled {
                continue 'scene_loop;
            }

            // Fill in initial handles to nodes to update.
            let mut update_queue = VecDeque::new();
            for (handle, node) in scene.graph.pair_iter() {
                if let Some(script) = node.script.as_ref() {
                    if script.initialized && script.started {
                        update_queue.push_back(handle);
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
                    resource_manager,
                    message_sender: &scripted_scene.message_sender,
                    message_dispatcher: &mut scripted_scene.message_dispatcher,
                };

                'init_loop: for init_loop_iteration in 0..max_iterations {
                    let mut start_queue = VecDeque::new();

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
    resource: Model,
    children: Vec<ResourceGraphVertex>,
}

impl ResourceGraphVertex {
    pub fn new(model: Model, resource_manager: ResourceManager) -> Self {
        let mut children = Vec::new();

        // Look for dependent resources.
        let mut dependent_resources = HashSet::new();
        for other_model in resource_manager.state().containers().models.iter() {
            let state = other_model.state();
            if let ResourceState::Ok(ref model_data) = *state {
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

    pub fn resolve(&self) {
        Log::info(format!(
            "Resolving {} resource from dependency graph...",
            self.resource.state().path().display()
        ));

        // Wait until resource is fully loaded, then resolve.
        if block_on(self.resource.clone()).is_ok() {
            self.resource.data_ref().get_scene_mut().resolve();

            for child in self.children.iter() {
                child.resolve();
            }
        }
    }
}

struct ResourceDependencyGraph {
    root: ResourceGraphVertex,
}

impl ResourceDependencyGraph {
    pub fn new(model: Model, resource_manager: ResourceManager) -> Self {
        Self {
            root: ResourceGraphVertex::new(model, resource_manager),
        }
    }

    pub fn resolve(&self) {
        self.root.resolve()
    }
}

pub struct GraphicsContextParams {
    /// Main window attributes.
    pub window_attributes: WindowAttributes,
    /// Whether to use vertical synchronization or not. V-sync will force your game to render
    /// frames with the synchronization rate of your monitor (which is ~60 FPS). Keep in mind
    /// vertical synchronization might not be available on your OS and engine might fail to
    /// initialize if v-sync is on.
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
    pub graphics_context_params: GraphicsContextParams,
    /// A special container that is able to create nodes by their type UUID.
    pub serialization_context: Arc<SerializationContext>,
    /// A resource manager.
    pub resource_manager: ResourceManager,
    /// (experimental) Run the engine without opening a window (TODO) and without sound.
    /// Useful for dedicated game servers or running on CI.
    ///
    /// Headless support is incomplete, for progress see
    /// <https://github.com/FyroxEngine/Fyrox/issues/222>.
    pub headless: bool,
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
    plugins: &mut [Box<dyn Plugin>],
    resource_manager: &ResourceManager,
    message_sender: &ScriptMessageSender,
    message_dispatcher: &mut ScriptMessageDispatcher,
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
        resource_manager,
        message_sender,
        message_dispatcher,
    };

    for node_index in 0..context.scene.graph.capacity() {
        context.handle = context.scene.graph.handle_from_index(node_index);

        process_node(&mut context, &mut func);
    }
}

mod kek {}

impl Engine {
    /// Creates new instance of engine from given initialization parameters. Automatically creates all sub-systems
    /// (sound, ui, resource manager, etc.) **except** graphics context. Graphics context **must** be created
    /// only on [`Event::Resumed`] by calling [`Self::resume`] and destroyed on [`Event::Suspended`] by calling
    /// [`Self::suspend`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fyrox::{
    ///     engine::{
    ///         resource_manager::ResourceManager, Engine, EngineInitParams, GraphicsContextParams,
    ///         SerializationContext,
    ///     },
    ///     event_loop::EventLoop,
    ///     window::WindowAttributes,
    /// };
    /// use std::sync::Arc;
    ///
    /// let graphics_context_params = GraphicsContextParams {
    ///     window_attributes: WindowAttributes {
    ///         title: "Some title".to_string(),
    ///         ..Default::default()
    ///     },
    ///     vsync: true,
    /// };
    /// let serialization_context = Arc::new(SerializationContext::new());
    /// Engine::new(EngineInitParams {
    ///     graphics_context_params,
    ///     resource_manager: ResourceManager::new(serialization_context.clone()),
    ///     serialization_context,
    ///     headless: false,
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
            headless,
        } = params;

        let sound_engine = if headless {
            SoundEngine::new_headless()
        } else {
            SoundEngine::new()
        };

        let (rx, tx) = channel();
        resource_manager
            .state()
            .containers_mut()
            .models
            .event_broadcaster
            .add(rx);

        Ok(Self {
            graphics_context_params,
            graphics_context: None,
            model_events_receiver: tx,
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
        })
    }

    pub fn resume(&mut self, window_target: &EventLoopWindowTarget<()>) -> Result<(), EngineError> {
        let mut window_builder = WindowBuilder::new();
        if let Some(inner_size) = self.graphics_context_params.window_attributes.inner_size {
            window_builder = window_builder.with_inner_size(inner_size);
        }
        if let Some(min_inner_size) = self
            .graphics_context_params
            .window_attributes
            .min_inner_size
        {
            window_builder = window_builder.with_min_inner_size(min_inner_size);
        }
        if let Some(max_inner_size) = self
            .graphics_context_params
            .window_attributes
            .max_inner_size
        {
            window_builder = window_builder.with_min_inner_size(max_inner_size);
        }
        if let Some(position) = self.graphics_context_params.window_attributes.position {
            window_builder = window_builder.with_position(position);
        }
        if let Some(resize_increments) = self
            .graphics_context_params
            .window_attributes
            .resize_increments
        {
            window_builder = window_builder.with_resize_increments(resize_increments);
        }
        unsafe {
            window_builder = window_builder
                .with_parent_window(self.graphics_context_params.window_attributes.parent_window);
        }
        window_builder = window_builder
            .with_resizable(self.graphics_context_params.window_attributes.resizable)
            .with_enabled_buttons(
                self.graphics_context_params
                    .window_attributes
                    .enabled_buttons,
            )
            .with_title(self.graphics_context_params.window_attributes.title.clone())
            .with_fullscreen(
                self.graphics_context_params
                    .window_attributes
                    .fullscreen
                    .clone(),
            )
            .with_maximized(self.graphics_context_params.window_attributes.maximized)
            .with_visible(self.graphics_context_params.window_attributes.visible)
            .with_transparent(self.graphics_context_params.window_attributes.transparent)
            .with_decorations(self.graphics_context_params.window_attributes.decorations)
            .with_window_icon(
                self.graphics_context_params
                    .window_attributes
                    .window_icon
                    .clone(),
            )
            .with_theme(
                self.graphics_context_params
                    .window_attributes
                    .preferred_theme,
            )
            .with_content_protected(
                self.graphics_context_params
                    .window_attributes
                    .content_protected,
            )
            .with_window_level(self.graphics_context_params.window_attributes.window_level)
            .with_active(self.graphics_context_params.window_attributes.active);

        #[cfg(not(target_arch = "wasm32"))]
        let (window, gl_context, gl_surface, glow_context) = {
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

            let gl3_3_core_context_attributes = ContextAttributesBuilder::new()
                .with_profile(GlProfile::Core)
                .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
                .build(Some(raw_window_handle));

            let gles3_context_attributes = ContextAttributesBuilder::new()
                .with_profile(GlProfile::Core)
                .with_context_api(ContextApi::Gles(Some(Version::new(3, 0))))
                .build(Some(raw_window_handle));

            unsafe {
                let attrs = window.build_surface_attributes(Default::default());

                let gl_surface = gl_config
                    .display()
                    .create_window_surface(&gl_config, &attrs)?;

                let non_current_gl_context = if let Ok(gl3_3_core_context) =
                    gl_display.create_context(&gl_config, &gl3_3_core_context_attributes)
                {
                    gl3_3_core_context
                } else {
                    gl_display.create_context(&gl_config, &gles3_context_attributes)?
                };

                let gl_context = non_current_gl_context.make_current(&gl_surface)?;

                if self.graphics_context_params.vsync {
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
                )
            }
        };

        #[cfg(target_arch = "wasm32")]
        let (window, glow_context) = {
            let window = window_builder.build(window_target).unwrap();

            use crate::core::wasm_bindgen::JsCast;
            use crate::platform::web::WindowExtWebSys;

            let canvas = window.canvas();

            let document = crate::core::web_sys::window().unwrap().document().unwrap();
            let body = document.body().unwrap();

            body.append_child(&canvas)
                .expect("Append canvas to HTML body");

            let webgl2_context = canvas
                .get_context("webgl2")
                .unwrap()
                .unwrap()
                .dyn_into::<crate::core::web_sys::WebGl2RenderingContext>()
                .unwrap();
            (window, glow::Context::from_webgl2_context(webgl2_context))
        };

        self.user_interface.set_screen_size(Vector2::new(
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        ));

        self.graphics_context = Some(GraphicsContext {
            #[cfg(not(target_arch = "wasm32"))]
            gl_context,
            #[cfg(not(target_arch = "wasm32"))]
            gl_surface,
            renderer: Renderer::new(
                glow_context,
                (window.inner_size().width, window.inner_size().height),
                &self.resource_manager,
            )?,
            window,
        });

        Ok(())
    }

    pub fn suspend(&mut self) {
        drop(self.graphics_context.take());
    }

    /// Adjust size of the frame to be rendered. Must be called after the window size changes.
    /// Will update the renderer and GL context frame size.
    pub fn set_frame_size(&mut self, new_size: (u32, u32)) -> Result<(), FrameworkError> {
        if let Some(graphics_context) = self.graphics_context.as_mut() {
            graphics_context.renderer.set_frame_size(new_size)?;

            #[cfg(not(target_arch = "wasm32"))]
            graphics_context.gl_surface.resize(
                &graphics_context.gl_context,
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
        control_flow: &mut ControlFlow,
        lag: &mut f32,
        switches: FxHashMap<Handle<Scene>, GraphUpdateSwitches>,
    ) {
        self.pre_update(dt, control_flow, lag, switches);
        self.post_update(dt);
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
        control_flow: &mut ControlFlow,
        lag: &mut f32,
        switches: FxHashMap<Handle<Scene>, GraphUpdateSwitches>,
    ) {
        if let Some(graphics_context) = self.graphics_context.as_mut() {
            let inner_size = graphics_context.window.inner_size();
            let window_size = Vector2::new(inner_size.width as f32, inner_size.height as f32);

            self.resource_manager.state().update(dt);
            graphics_context.renderer.update_caches(dt);
            self.handle_model_events();

            for (handle, scene) in self.scenes.pair_iter_mut().filter(|(_, s)| s.enabled) {
                let frame_size = scene.render_target.as_ref().map_or(window_size, |rt| {
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

            self.update_plugins(dt, control_flow, lag);
            self.handle_scripts(dt);
        }
    }

    /// Performs post update for the engine.
    ///
    /// Normally, this is called from `Engine::update()`.
    /// You should only call this manually if you don't use that method.
    pub fn post_update(&mut self, dt: f32) {
        if let Some(graphics_context) = self.graphics_context.as_ref() {
            let inner_size = graphics_context.window.inner_size();
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
        self.script_processor.register_scripted_scene(
            scene,
            &mut self.scenes,
            &self.resource_manager,
        )
    }

    fn handle_scripts(&mut self, dt: f32) {
        let time = instant::Instant::now();
        self.script_processor.handle_scripts(
            &mut self.scenes,
            &mut self.plugins,
            &self.resource_manager,
            dt,
            self.elapsed_time,
        );
        self.performance_statistics.scripts_time = instant::Instant::now() - time;
    }

    fn update_plugins(&mut self, dt: f32, control_flow: &mut ControlFlow, lag: &mut f32) {
        let time = instant::Instant::now();

        if self.plugins_enabled {
            let mut context = PluginContext {
                scenes: &mut self.scenes,
                resource_manager: &self.resource_manager,
                graphics_context: self.graphics_context.as_mut(),
                dt,
                lag,
                user_interface: &mut self.user_interface,
                serialization_context: &self.serialization_context,
                performance_statistics: &self.performance_statistics,
            };

            for plugin in self.plugins.iter_mut() {
                plugin.update(&mut context, control_flow);
            }

            while let Some(message) = self.user_interface.poll_message() {
                let mut context = PluginContext {
                    scenes: &mut self.scenes,
                    resource_manager: &self.resource_manager,
                    graphics_context: self.graphics_context.as_mut(),
                    dt,
                    lag,
                    user_interface: &mut self.user_interface,
                    serialization_context: &self.serialization_context,
                    performance_statistics: &self.performance_statistics,
                };

                for plugin in self.plugins.iter_mut() {
                    plugin.on_ui_message(&mut context, &message, control_flow);
                }
            }
        }

        self.performance_statistics.plugins_time = instant::Instant::now() - time;
    }

    pub(crate) fn handle_os_event_by_plugins(
        &mut self,
        event: &Event<()>,
        dt: f32,
        control_flow: &mut ControlFlow,
        lag: &mut f32,
    ) {
        if self.plugins_enabled {
            for plugin in self.plugins.iter_mut() {
                plugin.on_os_event(
                    event,
                    PluginContext {
                        scenes: &mut self.scenes,
                        resource_manager: &self.resource_manager,
                        graphics_context: self.graphics_context.as_mut(),
                        dt,
                        lag,
                        user_interface: &mut self.user_interface,
                        serialization_context: &self.serialization_context,
                        performance_statistics: &self.performance_statistics,
                    },
                    control_flow,
                );
            }
        }
    }

    pub(crate) fn handle_graphics_context_created_by_plugins(
        &mut self,
        dt: f32,
        control_flow: &mut ControlFlow,
        lag: &mut f32,
    ) {
        if self.plugins_enabled {
            for plugin in self.plugins.iter_mut() {
                plugin.on_graphics_context_created(
                    PluginContext {
                        scenes: &mut self.scenes,
                        resource_manager: &self.resource_manager,
                        graphics_context: self.graphics_context.as_mut(),
                        dt,
                        lag,
                        user_interface: &mut self.user_interface,
                        serialization_context: &self.serialization_context,
                        performance_statistics: &self.performance_statistics,
                    },
                    control_flow,
                );
            }
        }
    }

    pub(crate) fn handle_graphics_context_destroyed_by_plugins(
        &mut self,
        dt: f32,
        control_flow: &mut ControlFlow,
        lag: &mut f32,
    ) {
        if self.plugins_enabled {
            for plugin in self.plugins.iter_mut() {
                plugin.on_graphics_context_destroyed(
                    PluginContext {
                        scenes: &mut self.scenes,
                        resource_manager: &self.resource_manager,
                        graphics_context: self.graphics_context.as_mut(),
                        dt,
                        lag,
                        user_interface: &mut self.user_interface,
                        serialization_context: &self.serialization_context,
                        performance_statistics: &self.performance_statistics,
                    },
                    control_flow,
                );
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
        scene: Handle<Scene>,
        dt: f32,
    ) {
        if let Some(scripted_scene) = self
            .script_processor
            .scripted_scenes
            .iter_mut()
            .find(|s| s.handle == scene)
        {
            let scene = &mut self.scenes[scene];
            if scene.enabled {
                process_scripts(
                    scene,
                    &mut self.plugins,
                    &self.resource_manager,
                    &scripted_scene.message_sender,
                    &mut scripted_scene.message_dispatcher,
                    dt,
                    self.elapsed_time,
                    |script, context| {
                        if script.initialized {
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
            if let ResourceEvent::Reloaded(model) = event {
                Log::info(format!(
                    "A model resource {} was reloaded, propagating changes...",
                    model.state().path().display()
                ));

                // Build resource dependency graph and resolve it first.
                ResourceDependencyGraph::new(model, self.resource_manager.clone()).resolve();

                Log::info("Propagating changes to active scenes...");

                // Resolve all scenes.
                // TODO: This might be inefficient if there is bunch of scenes loaded,
                // however this seems to be very rare case so it should be ok.
                for scene in self.scenes.iter_mut() {
                    scene.resolve();
                }
            }
        }
    }

    /// Performs rendering of single frame, must be called from your game loop, otherwise you won't
    /// see anything.
    #[inline]
    pub fn render(&mut self) -> Result<(), FrameworkError> {
        self.user_interface.draw();

        if let Some(graphics_context) = self.graphics_context.as_mut() {
            #[cfg(not(target_arch = "wasm32"))]
            {
                graphics_context.renderer.render_and_swap_buffers(
                    &self.scenes,
                    self.user_interface.get_drawing_context(),
                    &graphics_context.gl_surface,
                    &graphics_context.gl_context,
                )?;
            }
            #[cfg(target_arch = "wasm32")]
            {
                graphics_context.renderer.render_and_swap_buffers(
                    &self.scenes,
                    &self.user_interface.get_drawing_context(),
                )?;
            }
        }

        Ok(())
    }

    /// Enables or disables registered plugins.
    pub(crate) fn enable_plugins(&mut self, override_scene: Handle<Scene>, enabled: bool) {
        if self.plugins_enabled != enabled {
            self.plugins_enabled = enabled;

            if self.plugins_enabled {
                // Create and initialize instances.
                for constructor in self.plugin_constructors.iter() {
                    self.plugins.push(constructor.create_instance(
                        override_scene,
                        PluginContext {
                            scenes: &mut self.scenes,
                            resource_manager: &self.resource_manager,
                            graphics_context: self.graphics_context.as_mut(),
                            dt: 0.0,
                            lag: &mut 0.0,
                            user_interface: &mut self.user_interface,
                            serialization_context: &self.serialization_context,
                            performance_statistics: &self.performance_statistics,
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
                        graphics_context: self.graphics_context.as_mut(),
                        dt: 0.0,
                        lag: &mut 0.0,
                        user_interface: &mut self.user_interface,
                        serialization_context: &self.serialization_context,
                        performance_statistics: &self.performance_statistics,
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
        self.enable_plugins(Default::default(), false);
    }
}

#[cfg(test)]
mod test {
    use crate::script::{ScriptMessageContext, ScriptMessagePayload};
    use crate::{
        core::{pool::Handle, reflect::prelude::*, uuid::Uuid, visitor::prelude::*},
        engine::{resource_manager::ResourceManager, ScriptProcessor},
        impl_component_provider,
        scene::{base::BaseBuilder, node::Node, pivot::PivotBuilder, Scene, SceneContainer},
        script::{Script, ScriptContext, ScriptDeinitContext, ScriptTrait},
    };
    use std::sync::mpsc::{self, Sender, TryRecvError};

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

        fn id(&self) -> Uuid {
            Uuid::new_v4()
        }
    }

    #[derive(Debug, Clone, Reflect, Visit)]
    struct MySubScript {
        #[reflect(hidden)]
        #[visit(skip)]
        sender: Sender<Event>,
    }

    impl_component_provider!(MySubScript);

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

        fn id(&self) -> Uuid {
            Uuid::new_v4()
        }
    }

    #[test]
    fn test_order() {
        let resource_manager = ResourceManager::new(Default::default());
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

        script_processor.register_scripted_scene(
            scene_handle,
            &mut scene_container,
            &resource_manager,
        );

        let handle_on_init = Handle::new(2, 1);
        let handle_on_start = Handle::new(3, 1);
        let handle_on_update1 = Handle::new(4, 1);

        for iteration in 0..3 {
            script_processor.handle_scripts(
                &mut scene_container,
                &mut Default::default(),
                &resource_manager,
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

        fn id(&self) -> Uuid {
            Uuid::new_v4()
        }
    }

    #[derive(Debug, Clone, Reflect, Visit)]
    struct ScriptSendingMessages {
        index: u32,
    }

    impl_component_provider!(ScriptSendingMessages);

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

        fn id(&self) -> Uuid {
            Uuid::new_v4()
        }
    }

    #[test]
    fn test_messages() {
        let resource_manager = ResourceManager::new(Default::default());
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

        script_processor.register_scripted_scene(
            scene_handle,
            &mut scene_container,
            &resource_manager,
        );

        for iteration in 0..2 {
            script_processor.handle_scripts(
                &mut scene_container,
                &mut Default::default(),
                &resource_manager,
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
