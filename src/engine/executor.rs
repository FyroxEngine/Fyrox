#![allow(missing_docs)]

use crate::{
    core::{futures::executor::block_on, instant::Instant, pool::Handle},
    engine::{resource_manager::ResourceManager, Engine, EngineInitParams, SerializationContext},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    plugin::PluginConstructor,
    scene::SceneLoader,
    utils::{
        log::{Log, MessageKind},
        translate_event,
    },
    window::WindowBuilder,
};
use clap::Parser;
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

    pub fn add_plugin_constructor<P>(&mut self, plugin: P)
    where
        P: PluginConstructor + 'static,
    {
        self.engine.add_plugin_constructor(plugin)
    }

    pub fn run(self) -> ! {
        let mut engine = self.engine;
        let event_loop = self.event_loop;

        let clock = Instant::now();
        let fixed_timestep = 1.0 / 60.0;
        let mut elapsed_time = 0.0;

        let args = Args::parse();

        let mut override_scene = Handle::NONE;
        if !args.override_scene.is_empty() {
            match block_on(SceneLoader::from_file(
                &args.override_scene,
                engine.serialization_context.clone(),
            )) {
                Ok(loader) => {
                    override_scene = engine
                        .scenes
                        .add(block_on(loader.finish(engine.resource_manager.clone())));
                }
                Err(e) => Log::warn(format!(
                    "Unable to load {} override scene! Reason: {:?}",
                    args.override_scene, e
                )),
            }
        }

        engine.enable_plugins(override_scene, true);

        event_loop.run(move |event, _, control_flow| {
            engine.handle_os_event_by_plugins(&event, fixed_timestep, control_flow);

            let scenes = engine
                .scenes
                .pair_iter()
                .map(|(s, _)| s)
                .collect::<Vec<_>>();

            for scene_handle in scenes.iter() {
                if !engine.scripted_scenes.contains(scene_handle) {
                    engine.initialize_scene_scripts(*scene_handle, fixed_timestep);
                    engine.scripted_scenes.insert(*scene_handle);
                }

                engine
                    .scripted_scenes
                    .retain(|s| engine.scenes.is_valid_handle(*s));

                engine.handle_os_event_by_scripts(&event, *scene_handle, fixed_timestep);
            }

            match event {
                Event::MainEventsCleared => {
                    let mut dt = clock.elapsed().as_secs_f32() - elapsed_time;
                    while dt >= fixed_timestep {
                        dt -= fixed_timestep;
                        elapsed_time += fixed_timestep;

                        engine.update(fixed_timestep, control_flow);
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
