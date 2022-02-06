//! Example - Custom game loop.
//!
//! Difficulty: Easy.
//!
//! This example shows how to create custom game loop.

use fyrox::{
    core::instant::Instant,
    engine::Engine,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    utils::{
        log::{Log, MessageKind},
        translate_event,
    },
};

fn main() {
    let event_loop = EventLoop::new();

    // Create window builder first.
    let window_builder = fyrox::window::WindowBuilder::new()
        .with_title("Example - Custom Game Loop")
        .with_resizable(true);

    let resource_manager_builder = fyrox::engine::resource_manager::ResourceManagerBuilder::new();

    // Then initialize the engine.
    let mut engine =
        Engine::new(window_builder, resource_manager_builder, &event_loop, true).unwrap();

    // Define game loop variables.
    let clock = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    // Finally run our event loop which will respond to OS and window events and update
    // engine state accordingly. Engine lets you to decide which event should be handled,
    // this is minimal working example if how it should be.
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                // This main game loop - it has fixed time step which means that game
                // code will run at fixed speed even if renderer can't give you desired
                // 60 fps.
                let mut dt = clock.elapsed().as_secs_f32() - elapsed_time;
                while dt >= fixed_timestep {
                    dt -= fixed_timestep;
                    elapsed_time += fixed_timestep;

                    // ************************
                    // ************************
                    // Put your game logic here.
                    // ************************
                    // ************************

                    // It is very important to update the engine every frame!
                    engine.update(fixed_timestep);
                }

                // It is very important to "pump" messages from UI. Even if don't need to
                // respond to such message, you should call this method, otherwise UI
                // might behave very weird.
                while let Some(_ui_event) = engine.user_interface.poll_message() {
                    // ************************
                    // ************************
                    // Put your data model synchronization code here. It should
                    // take message and update data in your game according to
                    // changes in UI.
                    // ************************
                    // ************************
                }

                // Rendering must be explicitly requested and handled after RedrawRequested event is received.
                engine.get_window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                // Run renderer at max speed - it is not tied to game code.
                engine.render().unwrap();
            }
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        // It is very important to handle Resized event from window, because
                        // renderer knows nothing about window size - it must be notified
                        // directly when window size has changed.
                        if let Err(e) = engine.set_frame_size(size.into()) {
                            Log::writeln(
                                MessageKind::Error,
                                format!("Unable to set frame size: {:?}", e),
                            );
                        }
                    }
                    // Handle rest of events here if necessary.
                    _ => (),
                }

                // It is very important to "feed" user interface (UI) with events coming
                // from main window, otherwise UI won't respond to mouse, keyboard, or any
                // other event.
                if let Some(os_event) = translate_event(&event) {
                    engine.user_interface.process_os_event(&os_event);
                }
            }
            // Continue polling messages from OS.
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}
