//! Example 04. User Interface
//!
//! Difficulty: Easy
//!
//! This example shows how to use user interface system of engine. It is
//! based on framework example because UI will be used to operate on
//! model.

pub mod shared;

use crate::shared::create_camera;
use rg3d::engine::resource_manager::MaterialSearchOptions;
use rg3d::engine::Engine;
use rg3d::gui::UiNode;
use rg3d::utils::log::{Log, MessageKind};
use rg3d::{
    animation::Animation,
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        color::Color,
        pool::Handle,
    },
    engine::resource_manager::ResourceManager,
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        border::BorderBuilder,
        button::ButtonBuilder,
        decorator::DecoratorBuilder,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            ButtonMessage, DropdownListMessage, MessageDirection, ScrollBarMessage, TextMessage,
            UiMessageData,
        },
        scroll_bar::ScrollBarBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Orientation, Thickness, VerticalAlignment,
    },
    monitor::VideoMode,
    scene::{node::Node, Scene},
    utils::translate_event,
    window::Fullscreen,
};
use rg3d_ui::formatted_text::WrapMode;
use std::time::Instant;

const DEFAULT_MODEL_ROTATION: f32 = 180.0;
const DEFAULT_MODEL_SCALE: f32 = 0.05;

struct Interface {
    debug_text: Handle<UiNode>,
    yaw: Handle<UiNode>,
    scale: Handle<UiNode>,
    reset: Handle<UiNode>,
    video_modes: Vec<VideoMode>,
    resolutions: Handle<UiNode>,
}

// User interface in the engine build up on graph data structure, on tree to be
// more precise. Each UI element can have single parent and multiple children.
// UI uses complex layout system which automatically organizes your widgets.
// In this example we'll use Grid and StackPanel layout controls. Grid can be
// divided in rows and columns, its child element can set their desired column
// and row and grid will automatically put them in correct position. StackPanel
// will "stack" UI elements either on top of each other or in one line. Such
// complex layout system was borrowed from WPF framework. You can read more here:
// https://docs.microsoft.com/en-us/dotnet/framework/wpf/advanced/layout
fn create_ui(engine: &mut Engine) -> Interface {
    let window_width = engine.renderer.get_frame_size().0 as f32;

    // Gather all suitable video modes, we'll use them to fill combo box of
    // available resolutions.
    let video_modes = engine
        .get_window()
        .primary_monitor()
        .unwrap()
        .video_modes()
        .filter(|vm| {
            // Leave only modern video modes, we are not in 1998.
            vm.size().width > 800 && vm.size().height > 600 && vm.bit_depth() == 32
        })
        .collect::<Vec<_>>();

    let ctx = &mut engine.user_interface.build_ctx();

    // First of all create debug text that will show title of example and current FPS.
    let debug_text = TextBuilder::new(WidgetBuilder::new()).build(ctx);

    // Then create model options window.
    let yaw;
    let scale;
    let reset;
    WindowBuilder::new(
        WidgetBuilder::new()
            // We want the window to be anchored at right top corner at the beginning
            .with_desired_position(Vector2::new(window_width - 300.0, 0.0))
            .with_width(300.0),
    )
    // Window can have any content you want, in this example it is Grid with other
    // controls. The layout looks like this:
    //  ______________________________
    // | Yaw         | Scroll bar    |
    // |_____________|_______________|
    // | Scale       | Scroll bar    |
    // |_____________|_______________|
    // |             | Reset button  |
    // |_____________|_______________|
    //
    .with_content(
        GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_vertical_alignment(VerticalAlignment::Center),
                    )
                    .with_text("Yaw")
                    .build(ctx),
                )
                .with_child({
                    yaw = ScrollBarBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(1)
                            // Make sure scroll bar will stay in center of available space.
                            .with_vertical_alignment(VerticalAlignment::Center)
                            // Add some margin so ui element won't be too close to each other.
                            .with_margin(Thickness::uniform(2.0)),
                    )
                    .with_min(0.0)
                    // Our max rotation is 360 degrees.
                    .with_max(360.0)
                    // Set some initial value
                    .with_value(DEFAULT_MODEL_ROTATION)
                    // Set step by which value will change when user will click on arrows.
                    .with_step(5.0)
                    // Make sure scroll bar will show its current value on slider.
                    .show_value(true)
                    // Turn off all decimal places.
                    .with_value_precision(0)
                    .build(ctx);
                    yaw
                })
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_vertical_alignment(VerticalAlignment::Center),
                    )
                    .with_wrap(WrapMode::Word)
                    .with_text("Scale")
                    .build(ctx),
                )
                .with_child({
                    scale = ScrollBarBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(1)
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_margin(Thickness::uniform(2.0)),
                    )
                    .with_min(0.01)
                    .with_max(0.1)
                    .with_step(0.01)
                    .with_value(DEFAULT_MODEL_SCALE)
                    .show_value(true)
                    .build(ctx);
                    scale
                })
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .on_row(2)
                            .on_column(1)
                            .with_horizontal_alignment(HorizontalAlignment::Right)
                            .with_child({
                                reset = ButtonBuilder::new(WidgetBuilder::new())
                                    .with_text("Reset")
                                    .build(ctx);
                                reset
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .add_column(Column::strict(100.0))
        .add_column(Column::stretch())
        .add_row(Row::strict(30.0))
        .add_row(Row::stretch())
        .add_row(Row::strict(30.0))
        .build(ctx),
    )
    .with_title(WindowTitle::text("Model Options"))
    .can_close(false)
    .build(ctx);

    // Create another window which will show some graphics options.
    let resolutions;
    WindowBuilder::new(
        WidgetBuilder::new()
            .with_desired_position(Vector2::new(window_width - 670.0, 0.0))
            .with_width(350.0),
    )
    .with_content(
        GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    TextBuilder::new(WidgetBuilder::new().on_column(0).on_row(0))
                        .with_text("Resolution")
                        .build(ctx),
                )
                .with_child({
                    resolutions =
                        DropdownListBuilder::new(WidgetBuilder::new().on_row(0).on_column(1))
                            // Set combo box items - each item will represent video mode value.
                            // When user will select something, we'll receive SelectionChanged
                            // message and will use received index to switch to desired video
                            // mode.
                            .with_items({
                                let mut items = Vec::new();
                                for video_mode in video_modes.iter() {
                                    let size = video_mode.size();
                                    let rate = video_mode.refresh_rate();
                                    let item = DecoratorBuilder::new(BorderBuilder::new(
                                        WidgetBuilder::new().with_height(28.0).with_child(
                                            TextBuilder::new(
                                                WidgetBuilder::new().with_horizontal_alignment(
                                                    HorizontalAlignment::Center,
                                                ),
                                            )
                                            .with_text(format!(
                                                "{}x{}@{}Hz",
                                                size.width, size.height, rate
                                            ))
                                            .build(ctx),
                                        ),
                                    ))
                                    .build(ctx);
                                    items.push(item);
                                }
                                items
                            })
                            .build(ctx);
                    resolutions
                }),
        )
        .add_column(Column::strict(120.0))
        .add_column(Column::stretch())
        .add_row(Row::strict(30.0))
        .build(ctx),
    )
    .with_title(WindowTitle::text("Graphics Options"))
    .can_close(false)
    .build(ctx);

    Interface {
        debug_text,
        yaw,
        scale,
        reset,
        resolutions,
        video_modes,
    }
}

struct GameScene {
    scene: Scene,
    model_handle: Handle<Node>,
    walk_animation: Handle<Animation>,
}

async fn create_scene(resource_manager: ResourceManager) -> GameScene {
    let mut scene = Scene::new();

    // Set ambient light.
    scene.ambient_lighting_color = Color::opaque(200, 200, 200);

    // Camera is our eyes in the world - you won't see anything without it.
    create_camera(
        resource_manager.clone(),
        Vector3::new(0.0, 6.0, -12.0),
        &mut scene.graph,
    )
    .await;

    // Load model resource. Is does *not* adds anything to our scene - it just loads a
    // resource then can be used later on to instantiate models from it on scene. Why
    // loading of resource is separated from instantiation? Because there it is too
    // inefficient to load a resource every time you trying to create instance of it -
    // much more efficient is to load it one and then make copies of it. In case of
    // models it is very efficient because single vertex and index buffer can be used
    // for all models instances, so memory footprint on GPU will be lower.
    let model_resource = resource_manager
        .request_model(
            "examples/data/mutant/mutant.FBX",
            MaterialSearchOptions::RecursiveUp,
        )
        .await
        .unwrap();

    // Instantiate model on scene - but only geometry, without any animations.
    // Instantiation is a process of embedding model resource data in desired scene.
    let model_handle = model_resource.instantiate_geometry(&mut scene);

    // Add simple animation for our model. Animations are loaded from model resources -
    // this is because animation is a set of skeleton bones with their own transforms.
    let walk_animation_resource = resource_manager
        .request_model(
            "examples/data/mutant/walk.fbx",
            MaterialSearchOptions::RecursiveUp,
        )
        .await
        .unwrap();

    // Once animation resource is loaded it must be re-targeted to our model instance.
    // Why? Because animation in *resource* uses information about *resource* bones,
    // not model instance bones, retarget_animations maps animations of each bone on
    // model instance so animation will know about nodes it should operate on.
    let walk_animation = *walk_animation_resource
        .retarget_animations(model_handle, &mut scene)
        .get(0)
        .unwrap();

    GameScene {
        scene,
        model_handle,
        walk_animation,
    }
}

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_title("Example - User Interface")
        .with_resizable(true);

    let mut engine = Engine::new(window_builder, &event_loop, true).unwrap();

    // Create simple user interface that will show some useful info.
    let interface = create_ui(&mut engine);

    // Create test scene.
    let GameScene {
        scene,
        model_handle,
        walk_animation,
    } = rg3d::core::futures::executor::block_on(create_scene(engine.resource_manager.clone()));

    // Add scene to engine - engine will take ownership over scene and will return
    // you a handle to scene which can be used later on to borrow it and do some
    // actions you need.
    let scene_handle = engine.scenes.add(scene);

    let clock = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    // We will rotate model using keyboard input.
    let mut model_angle = DEFAULT_MODEL_ROTATION;
    let mut model_scale = DEFAULT_MODEL_SCALE;

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

                    // Use stored scene handle to borrow a mutable reference of scene in
                    // engine.
                    let scene = &mut engine.scenes[scene_handle];

                    // Our animation must be applied to scene explicitly, otherwise
                    // it will have no effect.
                    scene
                        .animations
                        .get_mut(walk_animation)
                        .get_pose()
                        .apply(&mut scene.graph);

                    scene.graph[model_handle]
                        .local_transform_mut()
                        .set_scale(Vector3::new(model_scale, model_scale, model_scale))
                        .set_rotation(UnitQuaternion::from_axis_angle(
                            &Vector3::y_axis(),
                            model_angle.to_radians(),
                        ));

                    let fps = engine.renderer.get_statistics().frames_per_second;
                    engine.user_interface.send_message(TextMessage::text(
                        interface.debug_text,
                        MessageDirection::ToWidget,
                        format!("Example 04 - User Interface\nFPS: {}", fps),
                    ));

                    engine.update(fixed_timestep);
                }

                // It is very important to "pump" messages from UI. This our main point where we communicate
                // with user interface. As you saw earlier, there is no callbacks on UI elements, instead we
                // use messages to get information from UI elements. This provides perfect decoupling of logic
                // from UI elements and works well with borrow checker.
                while let Some(ui_message) = engine.user_interface.poll_message() {
                    match ui_message.data() {
                        UiMessageData::ScrollBar(msg)
                            if ui_message.direction() == MessageDirection::FromWidget =>
                        {
                            // Some of our scroll bars has changed its value. Check which one.
                            if let ScrollBarMessage::Value(value) = msg {
                                // Each message has source - a handle of UI element that created this message.
                                // It is used to understand from which UI element message has come.
                                if ui_message.destination() == interface.scale {
                                    model_scale = *value;
                                } else if ui_message.destination() == interface.yaw {
                                    model_angle = *value;
                                }
                            }
                        }
                        UiMessageData::Button(msg) => {
                            if let ButtonMessage::Click = msg {
                                // Once we received Click event from Reset button, we have to reset angle and scale
                                // of model. To do that we borrow each UI element in engine and set its value directly.
                                // This is not ideal because there is tight coupling between UI code and model values,
                                // but still good enough for example.
                                if ui_message.destination() == interface.reset {
                                    engine.user_interface.send_message(ScrollBarMessage::value(
                                        interface.scale,
                                        MessageDirection::ToWidget,
                                        DEFAULT_MODEL_SCALE,
                                    ));
                                    engine.user_interface.send_message(ScrollBarMessage::value(
                                        interface.yaw,
                                        MessageDirection::ToWidget,
                                        DEFAULT_MODEL_ROTATION,
                                    ));
                                }
                            }
                        }
                        UiMessageData::DropdownList(msg) => {
                            if let DropdownListMessage::SelectionChanged(idx) = msg {
                                // Video mode has changed and we must change video mode to what user wants.
                                if let Some(idx) = idx {
                                    if ui_message.destination() == interface.resolutions {
                                        let video_mode = interface.video_modes.get(*idx).unwrap();
                                        engine.get_window().set_fullscreen(Some(
                                            Fullscreen::Exclusive(video_mode.clone()),
                                        ));

                                        // Due to some weird bug in winit it does not send Resized event.
                                        if let Err(e) = engine.set_frame_size((
                                            video_mode.size().width,
                                            video_mode.size().height,
                                        )) {
                                            Log::writeln(
                                                MessageKind::Error,
                                                format!("Unable to set frame size: {:?}", e),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
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
                    WindowEvent::KeyboardInput { input, .. } => {
                        if let Some(key_code) = input.virtual_keycode {
                            if input.state == ElementState::Pressed
                                && key_code == VirtualKeyCode::Escape
                            {
                                *control_flow = ControlFlow::Exit;
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
