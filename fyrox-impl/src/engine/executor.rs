//! Executor is a small wrapper that manages plugins and scripts for your game.

use crate::{
    asset::manager::ResourceManager,
    core::{
        instant::Instant,
        log::{Log, MessageKind},
        task::TaskPool,
    },
    engine::{
        Engine, EngineInitParams, GraphicsContext, GraphicsContextParams, SerializationContext,
    },
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    gui::constructor::WidgetConstructorContainer,
    plugin::Plugin,
    utils::translate_event,
    window::WindowAttributes,
};
use clap::Parser;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Parser, Debug, Default)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = None)]
    override_scene: Option<String>,
}

/// Executor is a small wrapper that manages plugins and scripts for your game.
pub struct Executor {
    event_loop: EventLoop<()>,
    engine: Engine,
    desired_update_rate: f32,
    headless: bool,
    throttle_threshold: f32,
    throttle_frame_interval: usize,
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
    /// Default time step (in seconds).
    pub const DEFAULT_TIME_STEP: f32 = 1.0 / Self::DEFAULT_UPDATE_RATE;

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
            throttle_threshold: 2.0 * Self::DEFAULT_TIME_STEP,
            throttle_frame_interval: 5,
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

    /// Sets the desired throttle threshold (in seconds), at which the engine will stop trying to
    /// stabilize the update rate of the game logic and will increase the time step. This option
    /// could be useful to prevent potential hang up of the game if its logic or rendering takes too
    /// much time at each frame. The default value is two default time steps (33.3(3) milliseconds
    /// or 0.0333(3) seconds).
    ///
    /// ## Important notes
    ///
    /// Physics could suffer from variable time step which may result in objects falling through the
    /// ground and some other nasty things. Throttle threshold should be at reasonably high levels
    /// (usually 2x-3x of the fixed time step).
    pub fn set_throttle_threshold(&mut self, threshold: f32) {
        self.throttle_threshold = threshold.max(0.001);
    }

    /// Returns current throttle threshold. See [`Self::set_throttle_threshold`] docs for more info.
    pub fn throttle_threshold(&self) -> f32 {
        self.throttle_threshold
    }

    /// Sets the amount of frames (consecutive) that will be allowed to have lag spikes and the engine
    /// won't modify time step for internal update calls during such interval. This setting allows the
    /// engine to ignore small lag spikes and do not fast-forward game logic using variable time step.
    /// Variable time step could be bad for physics, which may result in objects falling through the
    /// ground, etc. Default is 5 frames.
    pub fn set_throttle_frame_interval(&mut self, interval: usize) {
        self.throttle_frame_interval = interval;
    }

    /// Returns current throttle frame interval. See [`Self::set_throttle_frame_interval`] docs for
    /// more info.
    pub fn throttle_frame_interval(&self) -> usize {
        self.throttle_frame_interval
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
        let throttle_threshold = self.throttle_threshold;
        let throttle_frame_interval = self.throttle_frame_interval;

        let args = Args::try_parse().unwrap_or_default();

        engine.enable_plugins(args.override_scene.as_deref(), true, Some(&event_loop));

        let mut previous = Instant::now();
        let fixed_time_step = 1.0 / self.desired_update_rate;
        let mut lag = 0.0;
        let mut frame_counter = 0usize;
        let mut last_throttle_frame_number = 0usize;

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

                    // Update rate stabilization loop.
                    while lag >= fixed_time_step {
                        let time_step;
                        if lag >= throttle_threshold
                            && (frame_counter - last_throttle_frame_number
                                >= throttle_frame_interval)
                        {
                            // Modify the delta time to let the game internals to fast-forward the
                            // logic by the current lag.
                            time_step = lag;
                            // Reset the lag to exit early from the loop, thus preventing its
                            // potential infinite increase, that in its turn could hang up the game.
                            lag = 0.0;

                            last_throttle_frame_number = frame_counter;
                        } else {
                            time_step = fixed_time_step;
                        }

                        engine.update(time_step, window_target, &mut lag, Default::default());

                        // Additional check is needed, because the `update` call above could modify
                        // the lag.
                        if lag >= fixed_time_step {
                            lag -= fixed_time_step;
                        } else if lag < 0.0 {
                            // Prevent from going back in time.
                            lag = 0.0;
                        }
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

                            frame_counter += 1;
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
