use fyrox::{
    core::instant::Instant,
    engine::{resource_manager::ResourceManager, Engine, EngineInitParams, SerializationContext},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    utils::{
        log::{Log, MessageKind},
        translate_event,
    },
};
use std::sync::Arc;

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = fyrox::window::WindowBuilder::new()
        .with_title("Fyrox Game Executor")
        .with_resizable(true);

    let serialization_context = Arc::new(SerializationContext::new());
    let mut engine = Engine::new(EngineInitParams {
        window_builder,
        resource_manager: ResourceManager::new(serialization_context.clone()),
        serialization_context,
        events_loop: &event_loop,
        vsync: true,
    })
    .unwrap();

    engine.load_plugins();

    let clock = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    event_loop.run(move |event, _, control_flow| {
        let scenes = engine
            .scenes
            .pair_iter()
            .map(|(s, _)| s)
            .collect::<Vec<_>>();

        for &scene_handle in scenes.iter() {
            engine.handle_os_event_by_scripts(&event, scene_handle, fixed_timestep);
        }

        match event {
            Event::MainEventsCleared => {
                let mut dt = clock.elapsed().as_secs_f32() - elapsed_time;
                while dt >= fixed_timestep {
                    dt -= fixed_timestep;
                    elapsed_time += fixed_timestep;

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
    });
}
