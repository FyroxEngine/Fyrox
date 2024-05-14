//! Executor is a small wrapper that manages plugins and scripts for your game.

use crate::plugin::Plugin;
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
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    utils::translate_event,
    window::WindowAttributes,
};
use clap::Parser;
use fyrox_core::task::TaskPool;
use fyrox_ui::constructor::WidgetConstructorContainer;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

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
        let task_pool = Arc::new(TaskPool::new());
        let engine = Engine::new(EngineInitParams {
            graphics_context_params,
            resource_manager: ResourceManager::new(task_pool.clone()),
            serialization_context,
            task_pool,
            widget_constructors: Arc::new(WidgetConstructorContainer::new()),
        })
        .unwrap();

        Self {
            event_loop,
            engine,
            desired_update_rate: Self::DEFAULT_UPDATE_RATE,
            headless: false,
        }
    }

    /// Creates new game executor using default window and with vsync turned on. For more flexible
    /// way to create an executor see [`Executor::from_params`].
    pub fn new() -> Self {
        let mut window_attributes = WindowAttributes::default();
        window_attributes.resizable = true;
        window_attributes.title = "Fyrox Game".to_string();

        Self::from_params(
            EventLoop::new().unwrap(),
            GraphicsContextParams {
                window_attributes,
                vsync: true,
                msaa_sample_count: None,
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

    /// Adds new plugin to the executor, the plugin will be enabled only on [`Executor::run`].
    pub fn add_plugin<P>(&mut self, plugin: P)
    where
        P: Plugin + 'static,
    {
        self.engine.add_plugin(plugin)
    }

    /// Runs the executor - starts your game.
    pub fn run(self) {
        let mut engine = self.engine;
        let event_loop = self.event_loop;
        let headless = self.headless;

        let args = Args::parse();

        engine.enable_plugins(
            if args.override_scene.is_empty() {
                None
            } else {
                Some(&args.override_scene)
            },
            true,
            Some(&event_loop),
        );

        let mut previous = Instant::now();
        let fixed_time_step = 1.0 / self.desired_update_rate;
        let mut lag = 0.0;

        run_executor(event_loop, move |event, window_target| {
            window_target.set_control_flow(ControlFlow::Wait);

            engine.handle_os_event_by_plugins(&event, fixed_time_step, window_target, &mut lag);

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
                        window_target,
                        &mut lag,
                    );
                }
                Event::Suspended if !headless => {
                    engine
                        .destroy_graphics_context()
                        .expect("Unable to destroy graphics context!");

                    engine.handle_graphics_context_destroyed_by_plugins(
                        fixed_time_step,
                        window_target,
                        &mut lag,
                    );
                }
                Event::AboutToWait => {
                    let elapsed = previous.elapsed();
                    previous = Instant::now();
                    lag += elapsed.as_secs_f32();

                    while lag >= fixed_time_step {
                        engine.update(fixed_time_step, window_target, &mut lag, Default::default());
                        lag -= fixed_time_step;
                    }

                    if let GraphicsContext::Initialized(ref ctx) = engine.graphics_context {
                        ctx.window.request_redraw();
                    }
                }
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => window_target.exit(),
                        WindowEvent::Resized(size) => {
                            if let Err(e) = engine.set_frame_size(size.into()) {
                                Log::writeln(
                                    MessageKind::Error,
                                    format!("Unable to set frame size: {:?}", e),
                                );
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            engine.handle_before_rendering_by_plugins(
                                fixed_time_step,
                                window_target,
                                &mut lag,
                            );

                            engine.render().unwrap();
                        }
                        _ => (),
                    }

                    if let Some(os_event) = translate_event(&event) {
                        for ui in engine.user_interfaces.iter_mut() {
                            ui.process_os_event(&os_event);
                        }
                    }
                }
                _ => (),
            }
        })
    }
}

fn run_executor<F>(event_loop: EventLoop<()>, callback: F)
where
    F: FnMut(Event<()>, &EventLoopWindowTarget<()>) + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn(callback);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        event_loop.run(callback).unwrap();
    }
}
