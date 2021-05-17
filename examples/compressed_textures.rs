//! Example - Texture compression
//!
//! Just shows two textures with compression. Engine compresses textures automatically,
//! based on compression options.

extern crate rg3d;

pub mod shared;

use rg3d::{
    core::{algebra::Vector2, color::Color, futures::executor::block_on},
    engine::resource_manager::TextureImportOptions,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{image::ImageBuilder, node::StubNode, widget::WidgetBuilder},
    resource::texture::CompressionOptions,
    utils::{into_gui_texture, translate_event},
};
use std::time::Instant;

// Create our own engine type aliases. These specializations are needed
// because engine provides a way to extend UI with custom nodes and messages.
type GameEngine = rg3d::engine::Engine<(), StubNode>;

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_title("Example - Compressed Textures")
        .with_resizable(true);

    let mut engine = GameEngine::new(window_builder, &event_loop, true).unwrap();

    // Explicitly set compression options - here we use Quality which in most cases will use
    // DXT5 compression with compression ratio 4:1
    engine.resource_manager.state().set_textures_import_options(
        TextureImportOptions::default().with_compression(CompressionOptions::Quality),
    );

    engine
        .renderer
        .set_backbuffer_clear_color(Color::opaque(120, 120, 120));

    ImageBuilder::new(
        WidgetBuilder::new()
            .with_desired_position(Vector2::new(0.0, 0.0))
            .with_width(512.0)
            .with_height(512.0),
    )
    .with_texture(into_gui_texture(
        block_on(
            engine
                .resource_manager
                .request_texture("examples/data/MetalMesh_Base_Color.png"),
        )
        .unwrap(),
    ))
    .build(&mut engine.user_interface.build_ctx());

    ImageBuilder::new(
        WidgetBuilder::new()
            .with_desired_position(Vector2::new(512.0, 0.0))
            .with_width(512.0)
            .with_height(512.0),
    )
    .with_texture(into_gui_texture(
        block_on(
            engine
                .resource_manager
                .request_texture("examples/data/R8Texture.png"),
        )
        .unwrap(),
    ))
    .build(&mut engine.user_interface.build_ctx());

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
                    engine.update(fixed_timestep);
                }

                // It is very important to "pump" messages from UI. Even if don't need to
                // respond to such message, you should call this method, otherwise UI
                // might behave very weird.
                while let Some(_ui_event) = engine.user_interface.poll_message() {
                    // ************************
                    // Put your data model synchronization code here. It should
                    // take message and update data in your game according to
                    // changes in UI.
                    // ************************
                }

                // Rendering must be explicitly requested and handled after RedrawRequested event is received.
                engine.get_window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                // Run renderer at max speed - it is not tied to game code.
                engine.render(fixed_timestep).unwrap();
            }
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        // It is very important to handle Resized event from window, because
                        // renderer knows nothing about window size - it must be notified
                        // directly when window size has changed.
                        engine.renderer.set_frame_size(size.into());
                    }
                    _ => (),
                }

                // It is very important to "feed" user interface (UI) with events coming
                // from main window, otherwise UI won't respond to mouse, keyboard, or any
                // other event.
                if let Some(os_event) = translate_event(&event) {
                    engine.user_interface.process_os_event(&os_event);
                }
            }
            Event::DeviceEvent { .. } => {
                // Handle key input events via `WindowEvent`, not via `DeviceEvent` (#32)
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}
