//! Example 08. Level of detail (LOD).
//!
//! Difficulty: Easy.
//!
//! This example shows how create a simple object with few detail levels.

pub mod shared;

use crate::shared::create_camera;
use std::sync::Arc;

use fyrox::{
    asset::manager::ResourceManager,
    core::{
        algebra::{UnitQuaternion, Vector3},
        color::Color,
        log::{Log, MessageKind},
        pool::Handle,
    },
    engine::{
        Engine, EngineInitParams, GraphicsContext, GraphicsContextParams, SerializationContext,
    },
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        BuildContext, UiNode,
    },
    keyboard::KeyCode,
    resource::model::{Model, ModelResourceExtension},
    scene::{
        base::{LevelOfDetail, LodGroup},
        node::Node,
        Scene,
    },
    utils::translate_event,
    window::WindowAttributes,
};
use std::time::Instant;

fn create_ui(ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new()).build(ctx)
}

struct GameScene {
    scene: Scene,
    camera: Handle<Node>,
    model_handle: Handle<Node>,
}

async fn create_scene(resource_manager: ResourceManager) -> GameScene {
    let mut scene = Scene::new();

    // Set ambient light.
    scene.ambient_lighting_color = Color::opaque(200, 200, 200);

    // Camera is our eyes in the world - you won't see anything without it.
    let camera = create_camera(
        resource_manager.clone(),
        Vector3::new(0.0, 1.5, -5.0),
        &mut scene.graph,
    )
    .await;

    // Set small z far for the sake of example.
    scene.graph[camera]
        .as_camera_mut()
        .projection_mut()
        .set_z_far(32.0);

    // Load model resource. Is does *not* adds anything to our scene - it just loads a
    // resource then can be used later on to instantiate models from it on scene. Why
    // loading of resource is separated from instantiation? Because there it is too
    // inefficient to load a resource every time you trying to create instance of it -
    // much more efficient is to load it one and then make copies of it. In case of
    // models it is very efficient because single vertex and index buffer can be used
    // for all models instances, so memory footprint on GPU will be lower.
    let model_resource = resource_manager
        .request::<Model, _>("examples/data/train/train.FBX")
        .await
        .unwrap();

    // Instantiate model on scene - but only geometry, without any animations.
    // Instantiation is a process of embedding model resource data in desired scene.
    let model_handle = model_resource.instantiate(&mut scene);

    // To enable level of detail we have to add lod group to model root, lod group
    // defines which parts of model should be visible at various distances.
    let lod_group = LodGroup {
        levels: vec![
            LevelOfDetail::new(
                // Distances given in normalized coordinates which has [0; 1] range.
                // 0 - closest to camera, 1 - farthest. Real distance can be obtained
                // by multiplying normalized distance to z far.
                0.0,
                0.33,
                vec![
                    scene
                        .graph
                        .find_by_name(model_handle, "metroLOD0")
                        .unwrap()
                        .0,
                ],
            ),
            LevelOfDetail::new(
                0.33,
                0.66,
                vec![
                    scene
                        .graph
                        .find_by_name(model_handle, "metroLOD1")
                        .unwrap()
                        .0,
                ],
            ),
            LevelOfDetail::new(
                0.66,
                1.0,
                vec![
                    scene
                        .graph
                        .find_by_name(model_handle, "metroLOD2")
                        .unwrap()
                        .0,
                ],
            ),
        ],
    };

    scene.graph[model_handle].set_lod_group(Some(lod_group));

    GameScene {
        scene,
        camera,
        model_handle,
    }
}

struct InputController {
    rotate_left: bool,
    rotate_right: bool,
    forward: bool,
    backward: bool,
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let graphics_context_params = GraphicsContextParams {
        window_attributes: WindowAttributes {
            title: "Example - Level of Detail".to_string(),
            resizable: true,
            ..Default::default()
        },
        vsync: true,
    };
    let serialization_context = Arc::new(SerializationContext::new());
    let mut engine = Engine::new(EngineInitParams {
        graphics_context_params,
        resource_manager: ResourceManager::new(),
        serialization_context,
    })
    .unwrap();

    // Create simple user interface that will show some useful info.
    let debug_text = create_ui(&mut engine.user_interface.build_ctx());

    // Create test scene.
    let GameScene {
        scene,
        model_handle,
        camera,
    } = fyrox::core::futures::executor::block_on(create_scene(engine.resource_manager.clone()));

    // Add scene to engine - engine will take ownership over scene and will return
    // you a handle to scene which can be used later on to borrow it and do some
    // actions you need.
    let scene_handle = engine.scenes.add(scene);

    let mut previous = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut lag = 0.0;

    // We will rotate model using keyboard input.
    let mut model_angle = 180.0f32.to_radians();
    let mut distance = 5.0;

    // Create input controller - it will hold information about needed actions.
    let mut input_controller = InputController {
        rotate_left: false,
        rotate_right: false,
        forward: false,
        backward: false,
    };

    // Finally run our event loop which will respond to OS and window events and update
    // engine state accordingly. Engine lets you to decide which event should be handled,
    // this is minimal working example if how it should be.
    event_loop
        .run(move |event, window_target, control_flow| {
            match event {
                Event::AboutToWait => {
                    // This main game loop - it has fixed time step which means that game
                    // code will run at fixed speed even if renderer can't give you desired
                    // 60 fps.
                    let elapsed = previous.elapsed();
                    previous = Instant::now();
                    lag += elapsed.as_secs_f32();
                    while lag >= fixed_timestep {
                        // ************************
                        // Put your game logic here.
                        // ************************

                        // Use stored scene handle to borrow a mutable reference of scene in
                        // engine.
                        let scene = &mut engine.scenes[scene_handle];

                        // Rotate model according to input controller state.
                        if input_controller.rotate_left {
                            model_angle -= 5.0f32.to_radians();
                        } else if input_controller.rotate_right {
                            model_angle += 5.0f32.to_radians();
                        }

                        if input_controller.forward {
                            distance = (distance - 0.1f32).max(0.0f32);
                        } else if input_controller.backward {
                            distance = (distance + 0.1f32).min(32.0f32)
                        }

                        scene.graph[model_handle]
                            .local_transform_mut()
                            .set_rotation(UnitQuaternion::from_axis_angle(
                                &Vector3::y_axis(),
                                model_angle,
                            ));

                        scene.graph[camera]
                            .local_transform_mut()
                            .set_position(Vector3::new(0.0, 1.5, -distance));

                        if let GraphicsContext::Initialized(ref ctx) = engine.graphics_context {
                            let fps = ctx.renderer.get_statistics().frames_per_second;
                            let text = format!(
                                "Example 08 - Level of Detail\n\
                            Use [A][D] keys to rotate model, \
                            [W][S] to zoom in/out.\nFPS: {}\n\
                            Triangles rendered: {}",
                                fps,
                                ctx.renderer.get_statistics().geometry.triangles_rendered
                            );
                            engine.user_interface.send_message(TextMessage::text(
                                debug_text,
                                MessageDirection::ToWidget,
                                text,
                            ));
                        }

                        engine.update(fixed_timestep, control_flow, &mut lag, Default::default());

                        lag -= fixed_timestep;
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
                    if let GraphicsContext::Initialized(ref ctx) = engine.graphics_context {
                        ctx.window.request_redraw();
                    }
                }
                Event::Resumed => {
                    engine.initialize_graphics_context(window_target).unwrap();
                }
                Event::Suspended => {
                    engine.destroy_graphics_context().unwrap();
                }
                Event::RedrawRequested(_) => {
                    // Run renderer at max speed - it is not tied to game code.
                    engine.render().unwrap();
                }
                Event::WindowEvent { event, .. } => {
                    match &event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(size) => {
                            // It is very important to handle Resized event from window, because
                            // renderer knows nothing about window size - it must be notified
                            // directly when window size has changed.
                            if let Err(e) = engine.set_frame_size((*size).into()) {
                                Log::writeln(
                                    MessageKind::Error,
                                    format!("Unable to set frame size: {:?}", e),
                                );
                            }
                        }
                        WindowEvent::KeyboardInput { event: input, .. } => match input.physical_key
                        {
                            KeyCode::KeyA => {
                                input_controller.rotate_left = input.state == ElementState::Pressed
                            }
                            KeyCode::KeyD => {
                                input_controller.rotate_right = input.state == ElementState::Pressed
                            }
                            KeyCode::KeyW => {
                                input_controller.forward = input.state == ElementState::Pressed
                            }
                            KeyCode::KeyS => {
                                input_controller.backward = input.state == ElementState::Pressed
                            }
                            _ => (),
                        },
                        _ => (),
                    }

                    // It is very important to "feed" user interface (UI) with events coming
                    // from main window, otherwise UI won't respond to mouse, keyboard, or any
                    // other event.
                    if let Some(os_event) = translate_event(&event) {
                        engine.user_interface.process_os_event(&os_event);
                    }
                }
                _ => *control_flow = ControlFlow::Poll,
            }
        })
        .unwrap();
}
