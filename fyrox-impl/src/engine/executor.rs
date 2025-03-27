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

//! Executor is a small wrapper that manages plugins and scripts for your game.

use crate::engine::ApplicationLoopController;
use crate::scene::Scene;
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
    plugin::Plugin,
    utils::translate_event,
    window::WindowAttributes,
};
use clap::Parser;
use fyrox_core::pool::Handle;
use fyrox_ui::constructor::new_widget_constructor_container;
use std::cell::Cell;
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
    event_loop: Option<EventLoop<()>>,
    engine: Engine,
    desired_update_rate: f32,
    throttle_threshold: f32,
    throttle_frame_interval: usize,
    resource_hot_reloading: bool,
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

impl Executor {
    /// Default update rate in frames per second.
    pub const DEFAULT_UPDATE_RATE: f32 = 60.0;
    /// Default time step (in seconds).
    pub const DEFAULT_TIME_STEP: f32 = 1.0 / Self::DEFAULT_UPDATE_RATE;

    /// Creates new game executor using specified set of parameters. Much more flexible version of
    /// [`Executor::new`]. To run the engine in headless mode, pass [`None`] to the `event_loop`
    /// argument.
    pub fn from_params(
        event_loop: Option<EventLoop<()>>,
        graphics_context_params: GraphicsContextParams,
    ) -> Self {
        let serialization_context = Arc::new(SerializationContext::new());
        let task_pool = Arc::new(TaskPool::new());
        let engine = Engine::new(EngineInitParams {
            graphics_context_params,
            resource_manager: ResourceManager::new(task_pool.clone()),
            serialization_context,
            task_pool,
            widget_constructors: Arc::new(new_widget_constructor_container()),
        })
        .unwrap();

        Self {
            event_loop,
            engine,
            desired_update_rate: Self::DEFAULT_UPDATE_RATE,
            throttle_threshold: 2.0 * Self::DEFAULT_TIME_STEP,
            throttle_frame_interval: 5,
            resource_hot_reloading: true,
        }
    }

    /// Creates new game executor using default window and with vsync turned on. For more flexible
    /// way to create an executor see [`Executor::from_params`]. To run the engine in headless mode,
    /// pass [`None`] to the `event_loop` argument.
    pub fn new(event_loop: Option<EventLoop<()>>) -> Self {
        let mut window_attributes = WindowAttributes::default();
        window_attributes.resizable = true;
        window_attributes.title = "Fyrox Game".to_string();

        Self::from_params(
            event_loop,
            GraphicsContextParams {
                window_attributes,
                vsync: true,
                msaa_sample_count: None,
                graphics_server_constructor: Default::default(),
            },
        )
    }

    /// Enables or disables hot reloading of changed resources (such as textures, shaders, scenes, etc.).
    /// Enabled by default.
    ///
    /// # Platform-specific
    ///
    /// Does nothing on Android and WebAssembly, because these OSes does not have rich file system
    /// as PC.
    pub fn set_resource_hot_reloading_enabled(&mut self, enabled: bool) {
        self.resource_hot_reloading = enabled;
    }

    /// Returns `true` if hot reloading of changed resources is enabled, `false` - otherwise.
    pub fn is_resource_hot_reloading_enabled(&self) -> bool {
        self.resource_hot_reloading
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
        let engine = self.engine;
        let event_loop = self.event_loop;
        let throttle_threshold = self.throttle_threshold;
        let throttle_frame_interval = self.throttle_frame_interval;

        if self.resource_hot_reloading {
            #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
            {
                use crate::core::watcher::FileSystemWatcher;
                use std::time::Duration;
                match FileSystemWatcher::new(".", Duration::from_secs(1)) {
                    Ok(watcher) => {
                        engine.resource_manager.state().set_watcher(Some(watcher));
                    }
                    Err(e) => {
                        Log::err(format!("Unable to create resource watcher. Reason {e:?}"));
                    }
                }
            }
        }

        let args = Args::try_parse().unwrap_or_default();

        match event_loop {
            Some(event_loop) => run_normal(
                engine,
                args.override_scene.as_deref(),
                event_loop,
                throttle_threshold,
                throttle_frame_interval,
                self.desired_update_rate,
            ),
            None => run_headless(
                engine,
                args.override_scene.as_deref(),
                throttle_threshold,
                throttle_frame_interval,
                self.desired_update_rate,
            ),
        }
    }
}

fn run_headless(
    mut engine: Engine,
    override_scene: Option<&str>,
    throttle_threshold: f32,
    throttle_frame_interval: usize,
    desired_update_rate: f32,
) {
    let mut previous = Instant::now();
    let fixed_time_step = 1.0 / desired_update_rate;
    let mut lag = 0.0;
    let mut frame_counter = 0usize;
    let mut last_throttle_frame_number = 0usize;
    let is_running = Cell::new(true);

    engine.enable_plugins(
        override_scene,
        true,
        ApplicationLoopController::Headless {
            running: &is_running,
        },
    );

    while is_running.get() {
        register_scripted_scenes(&mut engine);

        game_loop_iteration(
            &mut engine,
            ApplicationLoopController::Headless {
                running: &is_running,
            },
            &mut previous,
            &mut lag,
            fixed_time_step,
            throttle_threshold,
            throttle_frame_interval,
            frame_counter,
            &mut last_throttle_frame_number,
        );

        frame_counter += 1;
    }
}

fn run_normal(
    mut engine: Engine,
    override_scene: Option<&str>,
    event_loop: EventLoop<()>,
    throttle_threshold: f32,
    throttle_frame_interval: usize,
    desired_update_rate: f32,
) {
    let mut previous = Instant::now();
    let fixed_time_step = 1.0 / desired_update_rate;
    let mut lag = 0.0;
    let mut frame_counter = 0usize;
    let mut last_throttle_frame_number = 0usize;

    engine.enable_plugins(
        override_scene,
        true,
        ApplicationLoopController::WindowTarget(&event_loop),
    );

    run_executor(event_loop, move |event, window_target| {
        window_target.set_control_flow(ControlFlow::Wait);

        engine.handle_os_event_by_plugins(
            &event,
            fixed_time_step,
            ApplicationLoopController::WindowTarget(window_target),
            &mut lag,
        );

        let scripted_scenes = register_scripted_scenes(&mut engine);
        for scripted_scene in scripted_scenes {
            engine.handle_os_event_by_scripts(&event, scripted_scene, fixed_time_step);
        }

        match event {
            Event::Resumed => {
                engine
                    .initialize_graphics_context(window_target)
                    .expect("Unable to initialize graphics context!");

                engine.handle_graphics_context_created_by_plugins(
                    fixed_time_step,
                    ApplicationLoopController::WindowTarget(window_target),
                    &mut lag,
                );
            }
            Event::Suspended => {
                engine
                    .destroy_graphics_context()
                    .expect("Unable to destroy graphics context!");

                engine.handle_graphics_context_destroyed_by_plugins(
                    fixed_time_step,
                    ApplicationLoopController::WindowTarget(window_target),
                    &mut lag,
                );
            }
            Event::AboutToWait => {
                game_loop_iteration(
                    &mut engine,
                    ApplicationLoopController::WindowTarget(window_target),
                    &mut previous,
                    &mut lag,
                    fixed_time_step,
                    throttle_threshold,
                    throttle_frame_interval,
                    frame_counter,
                    &mut last_throttle_frame_number,
                );
            }
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => window_target.exit(),
                    WindowEvent::Resized(size) => {
                        if let Err(e) = engine.set_frame_size(size.into()) {
                            Log::writeln(
                                MessageKind::Error,
                                format!("Unable to set frame size: {e:?}"),
                            );
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        engine.handle_before_rendering_by_plugins(
                            fixed_time_step,
                            ApplicationLoopController::WindowTarget(window_target),
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

fn register_scripted_scenes(engine: &mut Engine) -> Vec<Handle<Scene>> {
    let scenes = engine
        .scenes
        .pair_iter()
        .map(|(s, _)| s)
        .collect::<Vec<_>>();

    for &scene_handle in scenes.iter() {
        if !engine.has_scripted_scene(scene_handle) {
            engine.register_scripted_scene(scene_handle);
        }
    }

    scenes
}

fn game_loop_iteration(
    engine: &mut Engine,
    controller: ApplicationLoopController,
    previous: &mut Instant,
    lag: &mut f32,
    fixed_time_step: f32,
    throttle_threshold: f32,
    throttle_frame_interval: usize,
    frame_counter: usize,
    last_throttle_frame_number: &mut usize,
) {
    let elapsed = previous.elapsed();
    *previous = Instant::now();
    *lag += elapsed.as_secs_f32();

    // Update rate stabilization loop.
    while *lag >= fixed_time_step {
        let time_step;
        if *lag >= throttle_threshold
            && (frame_counter - *last_throttle_frame_number >= throttle_frame_interval)
        {
            // Modify the delta time to let the game internals to fast-forward the
            // logic by the current lag.
            time_step = *lag;
            // Reset the lag to exit early from the loop, thus preventing its
            // potential infinite increase, that in its turn could hang up the game.
            *lag = 0.0;

            *last_throttle_frame_number = frame_counter;
        } else {
            time_step = fixed_time_step;
        }

        engine.update(time_step, controller, lag, Default::default());

        // Additional check is needed, because the `update` call above could modify
        // the lag.
        if *lag >= fixed_time_step {
            *lag -= fixed_time_step;
        } else if *lag < 0.0 {
            // Prevent from going back in time.
            *lag = 0.0;
        }
    }

    if let GraphicsContext::Initialized(ref ctx) = engine.graphics_context {
        ctx.window.request_redraw();
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
