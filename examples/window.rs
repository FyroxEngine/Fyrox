extern crate rg3d;

use rg3d::{
    event::{Event, WindowEvent},
    event_loop::{EventLoop, ControlFlow},
    engine::Engine,
    core::color::Color
};
use std::time::Instant;

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_title("Example 01: Model")
        .with_resizable(true);

    let mut engine = Engine::new(window_builder, &event_loop).unwrap();

    engine.interface_mut().renderer.set_ambient_color(Color::opaque(100, 100, 200));

    let clock = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::EventsCleared => {
                let mut dt = clock.elapsed().as_secs_f64() - elapsed_time;
                while dt >= fixed_timestep as f64 {
                    dt -= fixed_timestep as f64;
                    elapsed_time += fixed_timestep as f64;

                    engine.update(fixed_timestep);
                }
                engine.get_window().request_redraw();
            }
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::RedrawRequested => {
                        engine.render().unwrap();
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit
                    }
                    _ => ()
                }
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}