#![allow(missing_docs)]

use crate::core::pool::Handle;
use crate::scene::Scene;
use crate::{
    core::instant::Instant,
    engine::{resource_manager::ResourceManager, Engine, EngineInitParams, SerializationContext},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    plugin::Plugin,
    utils::{
        log::{Log, MessageKind},
        translate_event,
    },
    window::WindowBuilder,
};
use std::collections::HashSet;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub struct Executor {
    event_loop: EventLoop<()>,
    engine: Engine,
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
    pub fn new() -> Self {
        let event_loop = EventLoop::new();

        let window_builder = WindowBuilder::new()
            .with_title("Fyrox Game Executor")
            .with_resizable(true);

        let serialization_context = Arc::new(SerializationContext::new());
        let engine = Engine::new(EngineInitParams {
            window_builder,
            resource_manager: ResourceManager::new(serialization_context.clone()),
            serialization_context,
            events_loop: &event_loop,
            vsync: true,
        })
        .unwrap();

        Self { event_loop, engine }
    }

    pub fn add_plugin<P>(&mut self, plugin: P)
    where
        P: Plugin,
    {
        self.engine.add_plugin(plugin, false, true);
    }

    pub fn run(self) -> ! {
        let mut engine = self.engine;
        let event_loop = self.event_loop;

        let clock = Instant::now();
        let fixed_timestep = 1.0 / 60.0;
        let mut elapsed_time = 0.0;
        let mut initialized_scenes = HashSet::<Handle<Scene>>::default();

        event_loop.run(move |event, _, control_flow| {
            engine.handle_os_event_by_plugins(&event, fixed_timestep, true);

            let scenes = engine
                .scenes
                .pair_iter()
                .map(|(s, _)| s)
                .collect::<Vec<_>>();

            for scene_handle in scenes.iter() {
                if !initialized_scenes.contains(scene_handle) {
                    engine.initialize_scene_scripts(*scene_handle, fixed_timestep);
                    initialized_scenes.insert(*scene_handle);
                }

                engine.handle_os_event_by_scripts(&event, *scene_handle, fixed_timestep);
            }

            match event {
                Event::MainEventsCleared => {
                    let mut dt = clock.elapsed().as_secs_f32() - elapsed_time;
                    while dt >= fixed_timestep {
                        dt -= fixed_timestep;
                        elapsed_time += fixed_timestep;

                        engine.update_plugins(fixed_timestep, false);

                        for &scene_handle in scenes.iter() {
                            engine.update_scene_scripts(scene_handle, fixed_timestep);
                        }

                        engine.update(fixed_timestep);
                    }

                    while let Some(_ui_event) = engine.user_interface.poll_message() {}

                    engine.get_window().request_redraw();
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
