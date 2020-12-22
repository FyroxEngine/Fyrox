//! Example 09. Lightmap.
//!
//! Difficulty: Easy.
//!
//! This example shows how to load simple scene made in [rusty-editor](https://github.com/mrDIMAS/rusty-editor)
//! and generate lightmap for it. Lightmaps are still in active development and not meant to be used.

extern crate rg3d;

pub mod shared;

use crate::shared::create_camera;
use rg3d::{
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        color::Color,
        pool::Handle,
        visitor::{Visit, Visitor},
    },
    engine::resource_manager::ResourceManager,
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            ButtonMessage, MessageDirection, ProgressBarMessage, TextMessage, UiMessageData,
            WidgetMessage,
        },
        node::StubNode,
        progress_bar::ProgressBarBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        HorizontalAlignment, Thickness, VerticalAlignment,
    },
    scene::{node::Node, Scene},
    utils::{
        lightmap::{CancellationToken, Lightmap, ProgressIndicator, ProgressStage},
        translate_event,
    },
};
use rg3d_ui::message::WindowMessage;
use rg3d_ui::window::{WindowBuilder, WindowTitle};
use std::path::Path;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

// Create our own engine type aliases. These specializations are needed
// because engine provides a way to extend UI with custom nodes and messages.
type GameEngine = rg3d::engine::Engine<(), StubNode>;
type UiNode = rg3d::gui::node::UINode<(), StubNode>;
type BuildContext<'a> = rg3d::gui::BuildContext<'a, (), StubNode>;

const LIGHTMAP_SCENE_PATH: &str = "examples/data/lightmap_scene.rgs";

struct Interface {
    root: Handle<UiNode>,
    debug_text: Handle<UiNode>,
    progress_bar: Handle<UiNode>,
    progress_text: Handle<UiNode>,
    cancel: Handle<UiNode>,
    progress_grid: Handle<UiNode>,
    choice_window: Handle<UiNode>,
    load_existing: Handle<UiNode>,
    generate_new: Handle<UiNode>,
}

fn create_ui(ctx: &mut BuildContext, screen_size: Vector2<f32>) -> Interface {
    let debug_text;
    let progress_bar;
    let progress_text;
    let cancel;
    let progress_grid;
    let root = GridBuilder::new(
        WidgetBuilder::new()
            .with_width(screen_size.x)
            .with_height(screen_size.y)
            .with_child({
                debug_text =
                    TextBuilder::new(WidgetBuilder::new().on_row(0).on_column(0)).build(ctx);
                debug_text
            })
            .with_child({
                progress_grid = GridBuilder::new(
                    WidgetBuilder::new()
                        .with_visibility(!Path::new(LIGHTMAP_SCENE_PATH).exists())
                        .on_column(1)
                        .on_row(1)
                        .with_child({
                            progress_bar = ProgressBarBuilder::new(
                                WidgetBuilder::new().on_row(1).on_column(0),
                            )
                            .build(ctx);
                            progress_bar
                        })
                        .with_child({
                            progress_text = TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_column(0)
                                    .on_row(0)
                                    .with_margin(Thickness::bottom(20.0))
                                    .with_vertical_alignment(VerticalAlignment::Bottom),
                            )
                            .with_horizontal_text_alignment(HorizontalAlignment::Center)
                            .build(ctx);
                            progress_text
                        })
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .on_column(0)
                                    .on_row(2)
                                    .with_margin(Thickness::uniform(4.0))
                                    .with_child({
                                        cancel = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_height(30.0),
                                        )
                                        .with_text("Cancel")
                                        .build(ctx);
                                        cancel
                                    }),
                            )
                            .build(ctx),
                        ),
                )
                .add_column(Column::stretch())
                .add_row(Row::stretch())
                .add_row(Row::strict(25.0))
                .add_row(Row::stretch())
                .build(ctx);
                progress_grid
            }),
    )
    .add_row(Row::stretch())
    .add_row(Row::stretch())
    .add_row(Row::stretch())
    .add_column(Column::stretch())
    .add_column(Column::strict(500.0))
    .add_column(Column::stretch())
    .build(ctx);

    let load_existing;
    let generate_new;
    let choice_window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0))
        .open(false)
        .can_close(false)
        .with_title(WindowTitle::Text("Select Action".to_owned()))
        .with_content(
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child({
                        load_existing = ButtonBuilder::new(
                            WidgetBuilder::new()
                                .on_column(0)
                                .with_margin(Thickness::uniform(1.0)),
                        )
                        .with_text("Load Existing")
                        .build(ctx);
                        load_existing
                    })
                    .with_child({
                        generate_new = ButtonBuilder::new(
                            WidgetBuilder::new()
                                .on_column(1)
                                .with_margin(Thickness::uniform(1.0)),
                        )
                        .with_text("Generate New")
                        .build(ctx);
                        generate_new
                    }),
            )
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .add_column(Column::stretch())
            .build(ctx),
        )
        .build(ctx);

    Interface {
        root,
        debug_text,
        progress_bar,
        progress_text,
        cancel,
        choice_window,
        load_existing,
        generate_new,
        progress_grid,
    }
}

struct GameScene {
    scene: Scene,
    root: Handle<Node>,
}

struct SceneLoadContext {
    data: Option<GameScene>,
    progress_indicator: ProgressIndicator,
    cancellation_token: CancellationToken,
    start_time: Instant,
    generate_lightmap: bool,
}

fn create_scene_async(
    resource_manager: ResourceManager,
    generate_lightmap: bool,
) -> Arc<Mutex<SceneLoadContext>> {
    let progress_indicator = ProgressIndicator::new();
    let cancellation_token = CancellationToken::new();

    // Create load context - it will be shared with caller and loader threads.
    let context = Arc::new(Mutex::new(SceneLoadContext {
        data: None,
        progress_indicator: progress_indicator.clone(),
        cancellation_token: cancellation_token.clone(),
        start_time: Instant::now(),
        generate_lightmap,
    }));
    let result = context.clone();

    // Spawn separate thread which will create scene by loading various assets.
    std::thread::spawn(move || {
        futures::executor::block_on(async move {
            if generate_lightmap {
                let mut scene = Scene::new();

                // Camera is our eyes in the world - you won't see anything without it.
                create_camera(
                    resource_manager.clone(),
                    Vector3::new(5.0, 4.0, -8.0),
                    &mut scene.graph,
                )
                .await;

                // There is no difference between scene created in rusty-editor and any other
                // model file, so any scene can be used directly as resource.
                let root = resource_manager
                    .request_model("examples/data/Sponza.rgs")
                    .await
                    .unwrap()
                    .instantiate(&mut scene)
                    .root;

                if let Ok(lightmap) =
                    Lightmap::new(&mut scene, 64, cancellation_token, progress_indicator)
                {
                    lightmap
                        .save("examples/data/lightmaps/", resource_manager)
                        .unwrap();
                    scene.set_lightmap(lightmap).unwrap();

                    for node in scene.graph.linear_iter_mut() {
                        if let Node::Light(_) = node {
                            node.set_visibility(false);
                        }
                    }

                    let mut visitor = Visitor::new();
                    scene.visit("Scene", &mut visitor).unwrap();
                    visitor.save_binary(LIGHTMAP_SCENE_PATH).unwrap();

                    context.lock().unwrap().data = Some(GameScene { scene, root });
                }
            } else {
                let scene = Scene::from_file(LIGHTMAP_SCENE_PATH, resource_manager)
                    .await
                    .unwrap();
                let root = scene.graph[scene.graph.get_root()].children()[0];

                context.lock().unwrap().data = Some(GameScene { scene, root });
            }
        })
    });

    // Immediately return shared context.
    result
}

struct InputController {
    rotate_left: bool,
    rotate_right: bool,
}

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_title("Example 09 - Lightmap")
        .with_resizable(true);

    let mut engine = GameEngine::new(window_builder, &event_loop, true).unwrap();

    // Prepare resource manager - it must be notified where to search textures. When engine
    // loads model resource it automatically tries to load textures it uses. But since most
    // model formats store absolute paths, we can't use them as direct path to load texture
    // instead we telling engine to search textures in given folder.
    engine
        .resource_manager
        .state()
        .set_textures_path("examples/data");

    // Create simple user interface that will show some useful info.
    let window = engine.get_window();
    let screen_size = window.inner_size().to_logical(window.scale_factor());
    let interface = create_ui(
        &mut engine.user_interface.build_ctx(),
        Vector2::new(screen_size.width, screen_size.height),
    );

    let mut game_scene = if Path::new(LIGHTMAP_SCENE_PATH).exists() {
        engine
            .user_interface
            .send_message(WindowMessage::open_modal(
                interface.choice_window,
                MessageDirection::ToWidget,
                true,
            ));
        Arc::new(Mutex::new(SceneLoadContext {
            data: None,
            progress_indicator: ProgressIndicator::new(),
            cancellation_token: CancellationToken::new(),
            start_time: Instant::now(),
            generate_lightmap: false,
        }))
    } else {
        create_scene_async(engine.resource_manager.clone(), true)
    };

    // Initially these handles are None, once scene is loaded they'll be assigned.
    let mut scene_handle = Handle::NONE;
    let mut model_handle = Handle::NONE;

    // Set ambient light.
    engine.renderer.set_ambient_color(Color::opaque(80, 80, 80));

    let clock = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    // We will rotate model using keyboard input.
    let mut model_angle = 180.0f32.to_radians();

    // Create input controller - it will hold information about needed actions.
    let mut input_controller = InputController {
        rotate_left: false,
        rotate_right: false,
    };

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
                    // Put your game logic here.
                    // ************************

                    // Check each frame if our scene is created - here we just trying to lock context
                    // without blocking, it is important for main thread to be functional while other
                    // thread still loading data.
                    if let Ok(mut load_context) = game_scene.try_lock() {
                        if let Some(game_scene) = load_context.data.take() {
                            // Add scene to engine - engine will take ownership over scene and will return
                            // you a handle to scene which can be used later on to borrow it and do some
                            // actions you need.
                            scene_handle = engine.scenes.add(game_scene.scene);
                            model_handle = game_scene.root;

                            // Once scene is loaded, we should hide progress bar and text.
                            engine
                                .user_interface
                                .send_message(WidgetMessage::visibility(
                                    interface.progress_grid,
                                    MessageDirection::ToWidget,
                                    false,
                                ));
                            engine.user_interface.send_message(WindowMessage::close(
                                interface.choice_window,
                                MessageDirection::ToWidget,
                            ));
                        }

                        let stage = match load_context.progress_indicator.stage() {
                            ProgressStage::LightsCaching => "Caching Lights",
                            ProgressStage::UvGeneration => "Generating UVs",
                            ProgressStage::GeometryCaching => "Caching Geometry",
                            ProgressStage::CalculatingLight => "Calculating Light",
                        };

                        let message = if load_context.generate_lightmap {
                            format!(
                                "Please wait until lightmap is fully generated.\n\
                                Stage {} of 4: {}\n\
                                Elapsed time: {:.2} s",
                                load_context.progress_indicator.stage() as u32 + 1,
                                stage,
                                load_context.start_time.elapsed().as_secs_f32(),
                            )
                        } else {
                            format!(
                                "Please wait until existing lightmap is fully loaded.\n\
                                Elapsed time: {:.2} s",
                                load_context.start_time.elapsed().as_secs_f32(),
                            )
                        };

                        // Report progress in UI.
                        engine
                            .user_interface
                            .send_message(ProgressBarMessage::progress(
                                interface.progress_bar,
                                MessageDirection::ToWidget,
                                load_context.progress_indicator.progress_percent() as f32 / 100.0,
                            ));
                        engine.user_interface.send_message(TextMessage::text(
                            interface.progress_text,
                            MessageDirection::ToWidget,
                            message,
                        ));
                    }

                    // Update scene only if it is loaded.
                    if scene_handle.is_some() {
                        // Use stored scene handle to borrow a mutable reference of scene in
                        // engine.
                        let scene = &mut engine.scenes[scene_handle];

                        // Rotate model according to input controller state.
                        if input_controller.rotate_left {
                            model_angle -= 5.0f32.to_radians();
                        } else if input_controller.rotate_right {
                            model_angle += 5.0f32.to_radians();
                        }

                        scene.graph[model_handle]
                            .local_transform_mut()
                            .set_rotation(UnitQuaternion::from_axis_angle(
                                &Vector3::y_axis(),
                                model_angle,
                            ));
                    }

                    // While scene is loading, we will update progress bar.
                    let debug_text = format!(
                        "Example 09 - Lightmap\nUse [A][D] keys to rotate model.\n{}",
                        engine.renderer.get_statistics()
                    );
                    engine.user_interface.send_message(TextMessage::text(
                        interface.debug_text,
                        MessageDirection::ToWidget,
                        debug_text,
                    ));

                    while let Some(ui_event) = engine.user_interface.poll_message() {
                        if let UiMessageData::Button(msg) = ui_event.data() {
                            if let ButtonMessage::Click = msg {
                                if ui_event.destination() == interface.cancel {
                                    game_scene.lock().unwrap().cancellation_token.cancel();
                                    engine
                                        .user_interface
                                        .send_message(WidgetMessage::visibility(
                                            interface.progress_grid,
                                            MessageDirection::ToWidget,
                                            false,
                                        ));
                                    engine
                                        .user_interface
                                        .send_message(WindowMessage::open_modal(
                                            interface.choice_window,
                                            MessageDirection::ToWidget,
                                            true,
                                        ));
                                } else if ui_event.destination() == interface.generate_new {
                                    game_scene =
                                        create_scene_async(engine.resource_manager.clone(), true);
                                    engine.user_interface.send_message(WindowMessage::close(
                                        interface.choice_window,
                                        MessageDirection::ToWidget,
                                    ));
                                    engine
                                        .user_interface
                                        .send_message(WidgetMessage::visibility(
                                            interface.progress_grid,
                                            MessageDirection::ToWidget,
                                            true,
                                        ));
                                } else if ui_event.destination() == interface.load_existing {
                                    game_scene =
                                        create_scene_async(engine.resource_manager.clone(), false);
                                    engine.user_interface.send_message(WindowMessage::close(
                                        interface.choice_window,
                                        MessageDirection::ToWidget,
                                    ));
                                    engine
                                        .user_interface
                                        .send_message(WidgetMessage::visibility(
                                            interface.progress_grid,
                                            MessageDirection::ToWidget,
                                            true,
                                        ));
                                }
                            }
                        }
                    }

                    engine.update(fixed_timestep);
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

                        // Root UI node should be resized too, otherwise progress bar will stay
                        // in wrong position after resize.
                        let size = size.to_logical(engine.get_window().scale_factor());
                        engine.user_interface.send_message(WidgetMessage::width(
                            interface.root,
                            MessageDirection::ToWidget,
                            size.width,
                        ));
                        engine.user_interface.send_message(WidgetMessage::height(
                            interface.root,
                            MessageDirection::ToWidget,
                            size.height,
                        ));
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        // Handle key input events via `WindowEvent`, not via `DeviceEvent` (#32)
                        if let Some(key_code) = input.virtual_keycode {
                            match key_code {
                                VirtualKeyCode::A => {
                                    input_controller.rotate_left =
                                        input.state == ElementState::Pressed
                                }
                                VirtualKeyCode::D => {
                                    input_controller.rotate_right =
                                        input.state == ElementState::Pressed
                                }
                                _ => (),
                            }
                        }
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
