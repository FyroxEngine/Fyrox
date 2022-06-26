//! Engine is container for all subsystems (renderer, ui, sound, resource manager). It also
//! creates a window and an OpenGL context.

#![warn(missing_docs)]

pub mod error;
pub mod executor;
pub mod framework;
pub mod resource_manager;

use crate::{
    asset::ResourceState,
    core::{algebra::Vector2, futures::executor::block_on, instant, pool::Handle},
    engine::{
        error::EngineError,
        resource_manager::{container::event::ResourceEvent, ResourceManager},
    },
    event::Event,
    event_loop::EventLoop,
    gui::UserInterface,
    plugin::{Plugin, PluginContext, PluginRegistrationContext},
    renderer::{framework::error::FrameworkError, Renderer},
    resource::{model::Model, texture::TextureKind},
    scene::{
        graph::event::GraphEvent,
        node::{constructor::NodeConstructorContainer, Node, TypeUuidProvider},
        sound::SoundEngine,
        Scene, SceneContainer,
    },
    script::{constructor::ScriptConstructorContainer, Script, ScriptContext, ScriptDeinitContext},
    utils::log::Log,
    window::{Window, WindowBuilder},
};
use std::{
    collections::HashSet,
    sync::{
        mpsc::{self, channel, Receiver},
        Arc, Mutex,
    },
    time::Duration,
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

/// See module docs.
pub struct Engine {
    #[cfg(not(target_arch = "wasm32"))]
    context: glutin::WindowedContext<glutin::PossiblyCurrent>,
    #[cfg(target_arch = "wasm32")]
    window: winit::window::Window,
    /// Current renderer. You should call at least [render](Self::render) method to see your scene on
    /// screen.
    pub renderer: Renderer,
    /// User interface allows you to build interface of any kind.
    pub user_interface: UserInterface,
    /// Current resource manager. Resource manager can be cloned (it does clone only ref) to be able to
    /// use resource manager from any thread, this is useful to load resources from multiple
    /// threads to decrease loading times of your game by utilizing all available power of
    /// your CPU.
    pub resource_manager: ResourceManager,
    /// All available scenes in the engine.
    pub scenes: SceneContainer,
    /// The time user interface took for internal needs. TODO: This is not the right place
    /// for such statistics, probably it is best to make separate structure to hold all
    /// such data.
    pub ui_time: Duration,

    model_events_receiver: Receiver<ResourceEvent<Model>>,

    // Sound context control all sound sources in the engine. It is wrapped into Arc<Mutex<>>
    // because internally sound engine spawns separate thread to mix and send data to sound
    // device. For more info see docs for Context.
    sound_engine: Arc<Mutex<SoundEngine>>,

    // A set of plugins used by the engine.
    plugins: Vec<Box<dyn Plugin>>,

    plugins_enabled: bool,

    /// A special container that is able to create nodes by their type UUID. Use a copy of this
    /// value whenever you need it as a parameter in other parts of the engine.
    pub serialization_context: Arc<SerializationContext>,

    /// Defines a set of scenes whose scripts can be processed by the engine.
    pub scripted_scenes: HashSet<Handle<Scene>>,
}

struct ResourceGraphVertex {
    resource: Model,
    children: Vec<ResourceGraphVertex>,
    resource_manager: ResourceManager,
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
            resource_manager,
        }
    }

    pub fn resolve(&self) {
        Log::info(format!(
            "Resolving {} resource from dependency graph...",
            self.resource.state().path().display()
        ));

        block_on(
            self.resource
                .data_ref()
                .get_scene_mut()
                .resolve(self.resource_manager.clone()),
        );

        for child in self.children.iter() {
            child.resolve();
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

/// Engine initialization parameters.
pub struct EngineInitParams<'a> {
    /// A window builder.
    pub window_builder: WindowBuilder,
    /// A special container that is able to create nodes by their type UUID.
    pub serialization_context: Arc<SerializationContext>,
    /// A resource manager.
    pub resource_manager: ResourceManager,
    /// OS event loop.
    pub events_loop: &'a EventLoop<()>,
    /// Whether to use vertical synchronization or not. V-sync will force your game to render
    /// frames with the synchronization rate of your monitor (which is ~60 FPS). Keep in mind
    /// vertical synchronization could not be available on your OS and engine might fail to
    /// initialize if v-sync is on.
    pub vsync: bool,
}

fn process_node<T>(
    scene: &mut Scene,
    dt: f32,
    handle: Handle<Node>,
    plugins: &mut [Box<dyn Plugin>],
    resource_manager: &ResourceManager,
    func: &mut T,
) where
    T: FnMut(&mut Script, ScriptContext),
{
    // Take a script from node. We're temporarily taking ownership over script
    // instance.
    let mut script = match scene.graph.try_get_mut(handle) {
        Some(node) => {
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

    // Find respective plugin.
    if let Some(plugin) = plugins.iter_mut().find(|p| p.id() == script.plugin_uuid()) {
        // Form the context with all available data.
        let context = ScriptContext {
            dt,
            plugin: &mut **plugin,
            handle,
            scene,
            resource_manager,
        };

        func(&mut script, context);
    }

    // Put the script back to the node. We must do a checked borrow, because it is possible
    // that the node is already destroyed by script logic.
    if let Some(node) = scene.graph.try_get_mut(handle) {
        node.script = Some(script);
    }
}

pub(crate) fn process_scripts<T>(
    scene: &mut Scene,
    plugins: &mut [Box<dyn Plugin>],
    resource_manager: &ResourceManager,
    dt: f32,
    mut func: T,
) where
    T: FnMut(&mut Script, ScriptContext),
{
    // Start processing by going through the graph and processing nodes one-by-one.
    for node_index in 0..scene.graph.capacity() {
        let handle = scene.graph.handle_from_index(node_index);

        process_node(scene, dt, handle, plugins, resource_manager, &mut func);
    }
}

macro_rules! get_window {
    ($self:ident) => {{
        #[cfg(not(target_arch = "wasm32"))]
        {
            $self.context.window()
        }
        #[cfg(target_arch = "wasm32")]
        {
            &$self.window
        }
    }};
}

impl Engine {
    /// Creates new instance of engine from given initialization parameters.
    ///
    /// Automatically creates all sub-systems (renderer, sound, ui, etc.).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fyrox::engine::{Engine, EngineInitParams};
    /// use fyrox::window::WindowBuilder;
    /// use fyrox::engine::resource_manager::ResourceManager;
    /// use fyrox::event_loop::EventLoop;
    /// use std::sync::Arc;
    /// use fyrox::engine::SerializationContext;
    ///
    /// let evt = EventLoop::new();
    /// let window_builder = WindowBuilder::new()
    ///     .with_title("Test")
    ///     .with_fullscreen(None);
    /// let serialization_context = Arc::new(SerializationContext::new());
    /// let mut engine = Engine::new(EngineInitParams {
    ///     window_builder,
    ///     resource_manager: ResourceManager::new(serialization_context.clone()),
    ///     serialization_context,
    ///     events_loop: &evt,
    ///     vsync: false,
    /// })
    /// .unwrap();
    /// ```
    #[inline]
    #[allow(unused_variables)]
    pub fn new(params: EngineInitParams) -> Result<Self, EngineError> {
        let EngineInitParams {
            window_builder,
            serialization_context: node_constructors,
            resource_manager,
            events_loop,
            vsync,
        } = params;

        #[cfg(not(target_arch = "wasm32"))]
        let (context, client_size) = {
            let context_wrapper: glutin::WindowedContext<glutin::NotCurrent> =
                glutin::ContextBuilder::new()
                    .with_vsync(vsync)
                    .with_gl_profile(glutin::GlProfile::Core)
                    .with_gl(glutin::GlRequest::GlThenGles {
                        opengl_version: (3, 3),
                        opengles_version: (3, 0),
                    })
                    .build_windowed(window_builder, events_loop)?;

            let ctx = match unsafe { context_wrapper.make_current() } {
                Ok(context) => context,
                Err((_, e)) => return Err(EngineError::from(e)),
            };
            let inner_size = ctx.window().inner_size();
            (
                ctx,
                Vector2::new(inner_size.width as f32, inner_size.height as f32),
            )
        };

        #[cfg(target_arch = "wasm32")]
        let (window, client_size, glow_context) = {
            let winit_window = window_builder.build(events_loop).unwrap();

            use crate::core::wasm_bindgen::JsCast;
            use crate::platform::web::WindowExtWebSys;

            let canvas = winit_window.canvas();

            let window = crate::core::web_sys::window().unwrap();
            let document = window.document().unwrap();
            let body = document.body().unwrap();

            body.append_child(&canvas)
                .expect("Append canvas to HTML body");

            let webgl2_context = canvas
                .get_context("webgl2")
                .unwrap()
                .unwrap()
                .dyn_into::<crate::core::web_sys::WebGl2RenderingContext>()
                .unwrap();
            let glow_context = glow::Context::from_webgl2_context(webgl2_context);

            let inner_size = winit_window.inner_size();
            (
                winit_window,
                Vector2::new(inner_size.width as f32, inner_size.height as f32),
                glow_context,
            )
        };

        #[cfg(not(target_arch = "wasm32"))]
        let glow_context =
            { unsafe { glow::Context::from_loader_function(|s| context.get_proc_address(s)) } };

        let sound_engine = SoundEngine::new();

        let renderer = Renderer::new(
            glow_context,
            (client_size.x as u32, client_size.y as u32),
            &resource_manager,
        )?;

        let (rx, tx) = channel();
        resource_manager
            .state()
            .containers_mut()
            .models
            .event_broadcaster
            .add(rx);

        Ok(Self {
            model_events_receiver: tx,
            resource_manager,
            renderer,
            scenes: SceneContainer::new(sound_engine.clone()),
            sound_engine,
            user_interface: UserInterface::new(client_size),
            ui_time: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            context,
            #[cfg(target_arch = "wasm32")]
            window,
            plugins: Default::default(),
            serialization_context: node_constructors,
            scripted_scenes: Default::default(),
            plugins_enabled: false,
        })
    }

    /// Adjust size of the frame to be rendered. Must be called after the window size changes.
    /// Will update the renderer and GL context frame size.
    /// When using the [`framework::Framework`], you don't need to call this yourself.
    pub fn set_frame_size(&mut self, new_size: (u32, u32)) -> Result<(), FrameworkError> {
        self.renderer.set_frame_size(new_size)?;

        #[cfg(not(target_arch = "wasm32"))]
        self.context.resize(new_size.into());

        Ok(())
    }

    /// Returns reference to main window. Could be useful to set fullscreen mode, change
    /// size of window, its title, etc.
    #[inline]
    pub fn get_window(&self) -> &Window {
        get_window!(self)
    }

    /// Performs single update tick with given time delta. Engine internally will perform update
    /// of all scenes, sub-systems, user interface, etc. Must be called in order to get engine
    /// functioning.
    pub fn update(&mut self, dt: f32) {
        self.pre_update(dt);
        self.post_update(dt);
    }

    /// Performs pre update for the engine.
    ///
    /// Normally, this is called from `Engine::update()`.
    /// You should only call this manually if you don't use that method.
    pub fn pre_update(&mut self, dt: f32) {
        let inner_size = self.get_window().inner_size();
        let window_size = Vector2::new(inner_size.width as f32, inner_size.height as f32);

        self.resource_manager.state().update(dt);
        self.renderer.update_caches(dt);
        self.handle_model_events();

        for scene in self.scenes.iter_mut().filter(|s| s.enabled) {
            let frame_size = scene.render_target.as_ref().map_or(window_size, |rt| {
                if let TextureKind::Rectangle { width, height } = rt.data_ref().kind() {
                    Vector2::new(width as f32, height as f32)
                } else {
                    panic!("only rectangle textures can be used as render target!");
                }
            });

            scene.update(frame_size, dt);
        }

        self.update_plugins(dt);
        self.update_scripted_scene_scripts(dt);
    }

    /// Performs post update for the engine.
    ///
    /// Normally, this is called from `Engine::update()`.
    /// You should only call this manually if you don't use that method.
    pub fn post_update(&mut self, dt: f32) {
        let inner_size = self.get_window().inner_size();
        let window_size = Vector2::new(inner_size.width as f32, inner_size.height as f32);

        let time = instant::Instant::now();
        self.user_interface.update(window_size, dt);
        self.ui_time = instant::Instant::now() - time;

        self.handle_script_messages();
    }

    fn update_plugins(&mut self, dt: f32) {
        let mut context = PluginContext {
            scenes: &mut self.scenes,
            resource_manager: &self.resource_manager,
            renderer: &mut self.renderer,
            dt,
            serialization_context: self.serialization_context.clone(),
            window: get_window!(self),
        };

        for plugin in self.plugins.iter_mut() {
            plugin.update(&mut context);
        }
    }

    /// Processes an OS event by every registered plugin.
    pub fn handle_os_event_by_plugins(&mut self, event: &Event<()>, dt: f32) {
        if self.plugins_enabled {
            for plugin in self.plugins.iter_mut() {
                plugin.on_os_event(
                    event,
                    PluginContext {
                        scenes: &mut self.scenes,
                        resource_manager: &self.resource_manager,
                        renderer: &mut self.renderer,
                        dt,
                        serialization_context: self.serialization_context.clone(),
                        window: get_window!(self),
                    },
                );
            }
        }
    }

    /// Correctly handle all script instances. It is called automatically once per frame, but
    /// you can call it manually if you want immediate script message processing.
    ///
    /// # Motivation
    ///
    /// There is no way to initialize or destruct script instances on demand, that's why script
    /// initialization and destruction is deferred. It is called in controlled environment that
    /// has unique access to all required components thus solving borrowing issues.
    fn handle_script_messages(&mut self) {
        for (handle, scene) in self.scenes.pair_iter_mut() {
            if self.scripted_scenes.contains(&handle) {
                scene.handle_script_messages(&mut self.plugins, &self.resource_manager);
            } else {
                scene.discard_script_messages();
            }
        }

        // Process scripts from destroyed scenes.
        for (handle, mut detached_scene) in self.scenes.destruction_list.drain(..) {
            // Destroy every queued script instances first.
            if self.scripted_scenes.contains(&handle) {
                detached_scene.handle_script_messages(&mut self.plugins, &self.resource_manager);

                // Destroy every script instance from nodes that were still alive.
                for node_index in 0..detached_scene.graph.capacity() {
                    let node_handle = detached_scene.graph.handle_from_index(node_index);

                    if let Some(mut script) = detached_scene
                        .graph
                        .try_get_mut(node_handle)
                        .and_then(|node| node.script.take())
                    {
                        if let Some(plugin) = self
                            .plugins
                            .iter_mut()
                            .find(|p| p.id() == script.plugin_uuid())
                        {
                            script.on_deinit(ScriptDeinitContext {
                                plugin: &mut **plugin,
                                resource_manager: &self.resource_manager,
                                scene: &mut detached_scene,
                                node_handle,
                            })
                        }
                    }
                }
            }
        }
    }

    fn update_scripted_scene_scripts(&mut self, dt: f32) {
        for &scene in self.scripted_scenes.iter() {
            if let Some(scene) = self.scenes.try_get_mut(scene) {
                // Subscribe to graph events, we're interested in newly added nodes.
                // Subscription is weak and will break after this method automatically.
                let (tx, rx) = mpsc::channel();
                scene.graph.event_broadcaster.subscribe(tx);

                process_scripts(
                    scene,
                    &mut self.plugins,
                    &self.resource_manager,
                    dt,
                    |script, context| script.on_update(context),
                );

                // Initialize and update any newly added nodes, this will ensure that any newly created instances
                // are correctly processed.
                while let Ok(event) = rx.try_recv() {
                    if let GraphEvent::Added(node) = event {
                        // Init first.
                        process_node(
                            scene,
                            dt,
                            node,
                            &mut self.plugins,
                            &self.resource_manager,
                            &mut |script, context| script.on_init(context),
                        );

                        // Then update.
                        process_node(
                            scene,
                            dt,
                            node,
                            &mut self.plugins,
                            &self.resource_manager,
                            &mut |script, context| script.on_update(context),
                        );
                    }
                }
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
    pub fn handle_os_event_by_scripts(&mut self, event: &Event<()>, scene: Handle<Scene>, dt: f32) {
        process_scripts(
            &mut self.scenes[scene],
            &mut self.plugins,
            &self.resource_manager,
            dt,
            |script, context| script.on_os_event(event, context),
        )
    }

    /// Initializes every script in the scene.
    ///
    ///
    /// # Important notes
    ///
    /// This method is intended to be used by the editor and game runner. If you're using the
    /// engine as a framework, then you should not call this method because you'll most likely
    /// do something wrong.
    pub fn initialize_scene_scripts(&mut self, scene: Handle<Scene>, dt: f32) {
        // Wait until all resources are fully loaded (or failed to load). It is needed
        // because some scripts may use resources and any attempt to use non loaded resource
        // will result in panic.
        let wait_context = self
            .resource_manager
            .state()
            .containers_mut()
            .wait_concurrent();
        block_on(wait_context.wait_concurrent());

        if let Some(scene) = self.scenes.try_get_mut(scene) {
            // Subscribe to graph events, we're interested in newly added nodes.
            // Subscription is weak and will break after this method automatically.
            let (tx, rx) = mpsc::channel();
            scene.graph.event_broadcaster.subscribe(tx);

            process_scripts(
                scene,
                &mut self.plugins,
                &self.resource_manager,
                dt,
                |script, context| script.on_init(context),
            );

            // Initialize any newly added nodes, this will ensure that any newly created instances
            // are correctly processed.
            while let Ok(event) = rx.try_recv() {
                if let GraphEvent::Added(node) = event {
                    let wait_context = self
                        .resource_manager
                        .state()
                        .containers_mut()
                        .wait_concurrent();
                    block_on(wait_context.wait_concurrent());

                    process_node(
                        scene,
                        dt,
                        node,
                        &mut self.plugins,
                        &self.resource_manager,
                        &mut |script, context| script.on_init(context),
                    );
                }
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

                Log::info("Propagating changes to active scenes...".to_string());

                // Resolve all scenes.
                // TODO: This might be inefficient if there is bunch of scenes loaded,
                // however this seems to be very rare case so it should be ok.
                for scene in self.scenes.iter_mut() {
                    block_on(scene.resolve(self.resource_manager.clone()));
                }
            }
        }
    }

    /// Performs rendering of single frame, must be called from your game loop, otherwise you won't
    /// see anything.
    #[inline]
    pub fn render(&mut self) -> Result<(), FrameworkError> {
        self.user_interface.draw();

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.renderer.render_and_swap_buffers(
                &self.scenes,
                self.user_interface.get_drawing_context(),
                &self.context,
            )
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.renderer
                .render_and_swap_buffers(&self.scenes, &self.user_interface.get_drawing_context())
        }
    }

    /// Sets master gain of the sound engine. Can be used to control overall gain of all sound
    /// scenes at once.
    pub fn set_sound_gain(&mut self, gain: f32) {
        self.sound_engine.lock().unwrap().set_master_gain(gain);
    }

    /// Returns master gain of the sound engine.
    pub fn sound_gain(&self) -> f32 {
        self.sound_engine.lock().unwrap().master_gain()
    }

    /// Enables or disables registered plugins.
    pub fn enable_plugins(&mut self, override_scene: Handle<Scene>, enabled: bool) {
        if self.plugins_enabled != enabled {
            self.plugins_enabled = enabled;

            if self.plugins_enabled {
                for plugin in self.plugins.iter_mut() {
                    // Initialize plugin.
                    plugin.on_init(
                        override_scene,
                        PluginContext {
                            scenes: &mut self.scenes,
                            resource_manager: &self.resource_manager,
                            renderer: &mut self.renderer,
                            dt: 0.0,
                            serialization_context: self.serialization_context.clone(),
                            window: get_window!(self),
                        },
                    );
                }
            } else {
                self.handle_script_messages();

                for plugin in self.plugins.iter_mut() {
                    // Deinit plugin first.
                    plugin.on_deinit(PluginContext {
                        scenes: &mut self.scenes,
                        resource_manager: &self.resource_manager,
                        renderer: &mut self.renderer,
                        dt: 0.0,
                        serialization_context: self.serialization_context.clone(),
                        window: get_window!(self),
                    });
                    // Reset plugin state.
                    *plugin = plugin.default_boxed();
                }
            }
        }
    }

    /// Adds new plugin.
    pub fn add_plugin<P>(&mut self) -> bool
    where
        P: Plugin + Default + TypeUuidProvider,
    {
        if self.plugins.iter().any(|p| p.id() == P::type_uuid()) {
            false
        } else {
            let mut plugin = P::default();

            plugin.on_register(PluginRegistrationContext {
                serialization_context: self.serialization_context.clone(),
            });

            self.plugins.push(Box::new(plugin));
            true
        }
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
