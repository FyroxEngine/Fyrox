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

mod manager;
mod project;
mod settings;
mod upgrade;
mod utils;

use crate::{manager::ProjectManager, settings::DATA_DIR, utils::make_button};
use fyrox::core::Uuid;
use fyrox::engine::ApplicationLoopController;
use fyrox::{
    asset::{manager::ResourceManager, untyped::ResourceKind},
    core::{
        algebra::Matrix3,
        log::{Log, MessageKind},
        task::TaskPool,
    },
    dpi::PhysicalSize,
    engine::{
        Engine, EngineInitParams, GraphicsContext, GraphicsContextParams, SerializationContext,
    },
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        constructor::new_widget_constructor_container,
        font::{Font, FontResource},
        message::MessageDirection,
        widget::WidgetMessage,
        UserInterface,
    },
    utils::{translate_cursor_icon, translate_event},
    window::WindowAttributes,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

fn set_ui_scaling(ui: &UserInterface, scale: f32) {
    // High-DPI screen support
    ui.send_message(WidgetMessage::render_transform(
        ui.root(),
        MessageDirection::ToWidget,
        Matrix3::new_scaling(scale),
    ));
}

#[allow(clippy::unnecessary_to_owned)]
fn main() {
    Log::set_file_name(DATA_DIR.join("project_manager.log"));

    let mut window_attributes = WindowAttributes::default();
    window_attributes.inner_size = Some(PhysicalSize::new(720, 520).into());
    window_attributes.resizable = true;
    window_attributes.title = "Fyrox Project Manager".to_string();

    let serialization_context = Arc::new(SerializationContext::new());
    let task_pool = Arc::new(TaskPool::new());
    let mut engine = Engine::new(EngineInitParams {
        graphics_context_params: GraphicsContextParams {
            window_attributes,
            vsync: true,
            msaa_sample_count: Some(2),
            graphics_server_constructor: Default::default(),
        },
        resource_manager: ResourceManager::new(task_pool.clone()),
        serialization_context,
        task_pool,
        widget_constructors: Arc::new(new_widget_constructor_container()),
    })
    .unwrap();

    let primary_ui = engine.user_interfaces.first_mut();

    primary_ui.default_font = FontResource::new_ok(
        Uuid::new_v4(),
        ResourceKind::Embedded,
        Font::from_memory(
            include_bytes!("../resources/Roboto-Regular.ttf").to_vec(),
            1024,
        )
        .unwrap(),
    );
    let mut project_manager = ProjectManager::new(&mut primary_ui.build_ctx());

    let event_loop = EventLoop::new().unwrap();

    let mut previous = Instant::now();

    #[allow(unused_assignments)]
    let mut time_step = 1.0 / 60.0;

    event_loop
        .run(move |event, window_target| {
            if project_manager.mode.is_build() {
                // Keep updating with reduced rate to keep printing to the build log, but do not
                // eat as much time as in normal update mode.
                time_step = 1.0 / 10.0;

                window_target.set_control_flow(ControlFlow::wait_duration(
                    Duration::from_secs_f32(time_step),
                ));
            } else {
                // Wait for an event.
                window_target.set_control_flow(ControlFlow::Wait);
                time_step = 1.0 / 60.0;
            }

            match event {
                Event::Resumed => {
                    engine
                        .initialize_graphics_context(window_target)
                        .expect("Unable to initialize graphics context!");
                    let graphics_context = engine.graphics_context.as_initialized_mut();
                    graphics_context
                        .set_window_icon_from_memory(include_bytes!("../resources/icon.png"));
                    set_ui_scaling(
                        engine.user_interfaces.first(),
                        graphics_context.window.scale_factor() as f32,
                    );
                }
                Event::Suspended => {
                    engine
                        .destroy_graphics_context()
                        .expect("Unable to destroy graphics context!");
                }
                Event::AboutToWait => {
                    let ui = engine.user_interfaces.first_mut();

                    let elapsed = previous.elapsed();

                    if project_manager.is_active(ui) && elapsed.as_secs_f32() >= time_step {
                        previous = Instant::now();

                        let mut processed = 0;
                        while let Some(message) = ui.poll_message() {
                            project_manager.handle_ui_message(&message, ui);
                            processed += 1;
                        }
                        if processed > 0 {
                            project_manager
                                .update_loop_state
                                .request_update_in_next_frame();
                        }

                        engine.update(
                            time_step,
                            ApplicationLoopController::WindowTarget(window_target),
                            &mut 0.0,
                            Default::default(),
                        );

                        project_manager.update(engine.user_interfaces.first_mut(), time_step);

                        if let GraphicsContext::Initialized(ref ctx) = engine.graphics_context {
                            let window = &ctx.window;
                            window.set_cursor_icon(translate_cursor_icon(
                                engine.user_interfaces.first_mut().cursor(),
                            ));

                            ctx.window.request_redraw();
                        }

                        project_manager.update_loop_state.decrease_counter();
                    }
                }
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::Focused(focus) => {
                            project_manager.focused = focus;
                        }
                        WindowEvent::CloseRequested => window_target.exit(),
                        WindowEvent::Resized(size) => {
                            if let Err(e) = engine.set_frame_size(size.into()) {
                                Log::writeln(
                                    MessageKind::Error,
                                    format!("Unable to set frame size: {e:?}"),
                                );
                            }

                            let window = &engine.graphics_context.as_initialized_ref().window;

                            let logical_size = size.to_logical(window.scale_factor());
                            let ui = engine.user_interfaces.first_mut();
                            ui.send_message(WidgetMessage::width(
                                project_manager.root_grid,
                                MessageDirection::ToWidget,
                                logical_size.width,
                            ));
                            ui.send_message(WidgetMessage::height(
                                project_manager.root_grid,
                                MessageDirection::ToWidget,
                                logical_size.height,
                            ));
                        }
                        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                            let ui = engine.user_interfaces.first_mut();
                            set_ui_scaling(ui, scale_factor as f32);
                        }
                        WindowEvent::RedrawRequested => {
                            let ui = engine.user_interfaces.first();
                            if project_manager.is_active(ui) {
                                engine.render().unwrap();
                            }
                        }
                        _ => (),
                    }

                    // Any action in the window, other than a redraw request forces the project manager to
                    // do another update pass which then pushes a redraw request to the event
                    // queue. This check prevents infinite loop of this kind.
                    if !matches!(event, WindowEvent::RedrawRequested) {
                        project_manager
                            .update_loop_state
                            .request_update_in_current_frame();
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
