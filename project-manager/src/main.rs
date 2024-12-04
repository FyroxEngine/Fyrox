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

//! Project manager is used to create, import, rename, delete, run and edit projects built with Fyrox.

mod build;
mod manager;
mod project;
mod settings;
mod utils;

use crate::{manager::ProjectManager, utils::make_button};
use fyrox::{
    asset::manager::ResourceManager,
    core::{
        instant::Instant,
        log::{Log, MessageKind},
        task::TaskPool,
    },
    dpi::PhysicalSize,
    engine::{
        Engine, EngineInitParams, GraphicsContext, GraphicsContextParams, SerializationContext,
    },
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{constructor::new_widget_constructor_container, font::Font},
    utils::translate_event,
    window::WindowAttributes,
};
use std::sync::Arc;

fn main() {
    let mut window_attributes = WindowAttributes::default();
    window_attributes.inner_size = Some(PhysicalSize::new(520, 562).into());
    window_attributes.resizable = true;
    window_attributes.title = "Fyrox Project Manager".to_string();

    let serialization_context = Arc::new(SerializationContext::new());
    let task_pool = Arc::new(TaskPool::new());
    let mut engine = Engine::new(EngineInitParams {
        graphics_context_params: GraphicsContextParams {
            window_attributes,
            vsync: true,
            msaa_sample_count: Some(2),
        },
        resource_manager: ResourceManager::new(task_pool.clone()),
        serialization_context,
        task_pool,
        widget_constructors: Arc::new(new_widget_constructor_container()),
    })
    .unwrap();

    let primary_ui = engine.user_interfaces.first_mut();
    primary_ui.default_font = engine
        .resource_manager
        .request::<Font>("resources/arial.ttf");
    let mut project_manager = ProjectManager::new(&mut primary_ui.build_ctx());

    let event_loop = EventLoop::new().unwrap();

    let mut previous = Instant::now();
    let fixed_time_step = 1.0 / 60.0;
    let mut lag = 0.0;

    event_loop
        .run(move |event, window_target| {
            window_target.set_control_flow(ControlFlow::Wait);

            match event {
                Event::Resumed => {
                    engine
                        .initialize_graphics_context(window_target)
                        .expect("Unable to initialize graphics context!");
                    engine
                        .graphics_context
                        .as_initialized_mut()
                        .set_window_icon_from_memory(include_bytes!("../resources/icon.png"));
                }
                Event::Suspended => {
                    engine
                        .destroy_graphics_context()
                        .expect("Unable to destroy graphics context!");
                }
                Event::AboutToWait => {
                    let elapsed = previous.elapsed();
                    previous = Instant::now();
                    lag += elapsed.as_secs_f32();

                    while lag >= fixed_time_step {
                        engine.update(fixed_time_step, window_target, &mut lag, Default::default());

                        project_manager.update(engine.user_interfaces.first_mut());

                        lag -= fixed_time_step;
                    }

                    let ui = engine.user_interfaces.first_mut();
                    while let Some(message) = ui.poll_message() {
                        project_manager.handle_ui_message(&message, ui);
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
                                    format!("Unable to set frame size: {e:?}"),
                                );
                            }
                        }
                        WindowEvent::RedrawRequested => {
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
                Event::LoopExiting => {
                    project_manager.settings.save();
                }
                _ => (),
            }
        })
        .unwrap();
}
