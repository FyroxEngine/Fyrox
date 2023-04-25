//! Executor is a small wrapper that manages plugins and scripts for your game.

use crate::{
    asset::manager::ResourceManager,
    core::{
        instant::Instant,
        log::{Log, MessageKind},
    },
    engine::{
        Engine, EngineInitParams, GraphicsContext, GraphicsContextParams, SerializationContext,
    },
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    plugin::PluginConstructor,
    scene::loader::AsyncSceneLoader,
    utils::translate_event,
};
use clap::Parser;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use winit::window::WindowAttributes;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "")]
    override_scene: String,
}

/// Executor is a small wrapper that manages plugins and scripts for your game.
pub struct Executor {
    event_loop: EventLoop<()>,
    engine: Engine,
    desired_update_rate: f32,
    loader: Option<AsyncSceneLoader>,
    headless: bool,
}

impl Deref for Executor {
    type Target = Engine;

    fn deref(&self) -> &Self::Target {
        &self.engine
    }
}

impl DerefMut for Executor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.engine
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    /// Default update rate in frames per second.
    pub const DEFAULT_UPDATE_RATE: f32 = 60.0;

    /// Creates new game executor using specified set of parameters. Much more flexible version of
    /// [`Executor::new`].
    pub fn from_params(
        event_loop: EventLoop<()>,
        graphics_context_params: GraphicsContextParams,
    ) -> Self {
        let serialization_context = Arc::new(SerializationContext::new());
        let engine = Engine::new(EngineInitParams {
            graphics_context_params,
            resource_manager: ResourceManager::new(),
            serialization_context,
        })
        .unwrap();

        Self {
            event_loop,
            engine,
            desired_update_rate: Self::DEFAULT_UPDATE_RATE,
            loader: None,
            headless: false,
        }
    }

    /// Creates new game executor using default window and with vsync turned on. For more flexible
    /// way to create an executor see [`Executor::from_params`].
    pub fn new() -> Self {
        Self::from_params(
            EventLoop::new(),
            GraphicsContextParams {
                window_attributes: WindowAttributes {
                    resizable: true,
                    title: "Fyrox Game".to_string(),
                    ..Default::default()
                },
                vsync: true,
            },
        )
    }

    /// Defines whether the executor should initialize graphics context or not. Headless mode could
    /// be useful for game servers, where you don't need to have a window, renderer, sound, etc.
    /// By default, headless mode is off.
    pub fn set_headless(&mut self, headless: bool) {
        self.headless = headless;
    }

    /// Returns `true` if the headless mode is turned on, `false` - otherwise.
    pub fn is_headless(&self) -> bool {
        self.headless
    }

    /// Sets the desired update rate in frames per second.
    pub fn set_desired_update_rate(&mut self, update_rate: f32) {
        self.desired_update_rate = update_rate.abs();
    }

    /// Returns desired update rate in frames per second.
    pub fn desired_update_rate(&self) -> f32 {
        self.desired_update_rate
    }

    /// Adds new plugin constructor to the executor, the plugin will be enabled only on [`Executor::run`].
    pub fn add_plugin_constructor<P>(&mut self, plugin: P)
    where
        P: PluginConstructor + 'static,
    {
        self.engine.add_plugin_constructor(plugin)
    }

    /// Runs the executor - starts your game. This function is never returns.
    pub fn run(mut self) -> ! {
        let mut engine = self.engine;
        let event_loop = self.event_loop;
        let headless = self.headless;

        let args = Args::parse();

        if !args.override_scene.is_empty() {
            // Try to load specified scene in a separate thread.
            self.loader = Some(AsyncSceneLoader::begin_loading(
                args.override_scene.into(),
                engine.serialization_context.clone(),
                engine.resource_manager.clone(),
            ));
        } else {
            // Enable plugins immediately.
            engine.enable_plugins(Default::default(), true);
        }

        let mut previous = Instant::now();
        let fixed_time_step = 1.0 / self.desired_update_rate;
        let mut lag = 0.0;

        event_loop.run(move |event, window_target, control_flow| {
            engine.handle_os_event_by_plugins(&event, fixed_time_step, control_flow, &mut lag);

            let scenes = engine
                .scenes
                .pair_iter()
                .map(|(s, _)| s)
                .collect::<Vec<_>>();

            for &scene_handle in scenes.iter() {
                if !engine.has_scripted_scene(scene_handle) {
                    engine.register_scripted_scene(scene_handle);
                }

                engine.handle_os_event_by_scripts(&event, scene_handle, fixed_time_step);
            }

            match event {
                Event::Resumed if !headless => {
                    engine
                        .initialize_graphics_context(window_target)
                        .expect("Unable to initialize graphics context!");

                    engine.handle_graphics_context_created_by_plugins(
                        fixed_time_step,
                        control_flow,
                        &mut lag,
                    );
                }
                Event::Suspended if !headless => {
                    engine
                        .destroy_graphics_context()
                        .expect("Unable to destroy graphics context!");

                    engine.handle_graphics_context_destroyed_by_plugins(
                        fixed_time_step,
                        control_flow,
                        &mut lag,
                    );
                }
                Event::MainEventsCleared => {
                    if let Some(loader) = self.loader.as_ref() {
                        if let Some(result) = loader.fetch_result() {
                            let override_scene = match result {
                                Ok(scene) => engine.scenes.add(scene),
                                Err(e) => {
                                    Log::err(e);
                                    Default::default()
                                }
                            };

                            engine.enable_plugins(override_scene, true);

                            self.loader = None;
                        }
                    }

                    let elapsed = previous.elapsed();
                    previous = Instant::now();
                    lag += elapsed.as_secs_f32();

                    while lag >= fixed_time_step {
                        engine.update(fixed_time_step, control_flow, &mut lag, Default::default());
                        lag -= fixed_time_step;
                    }

                    if let GraphicsContext::Initialized(ref ctx) = engine.graphics_context {
                        ctx.window.request_redraw();
                    }
                }
                Event::RedrawRequested(_) => {
                    engine.render().unwrap();
                }
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(size) => {
                            if let Err(e) = engine.set_frame_size(size.into()) {
                                Log::writeln(
                                    MessageKind::Error,
                                    format!("Unable to set frame size: {:?}", e),
                                );
                            }
                        }
                        _ => (),
                    }

                    if let Some(os_event) = translate_event(&event) {
                        engine.user_interface.process_os_event(&os_event);
                    }
                }
                _ => *control_flow = ControlFlow::Poll,
            }
        })
    }
}
