//! Inspector testbed (WIP)

pub mod shared;

use crate::shared::create_camera;
use fyrox::{
    asset::manager::ResourceManager,
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
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
        inspector::{
            editors::PropertyEditorDefinitionContainer, FieldKind, InspectorBuilder,
            InspectorContext, InspectorMessage,
        },
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        UiNode,
    },
    keyboard::KeyCode,
    resource::model::{Model, ModelResourceExtension},
    scene::{node::Node, Scene},
    utils::translate_event,
    window::WindowAttributes,
};
use std::{rc::Rc, sync::Arc, time::Instant};

struct Interface {
    debug_text: Handle<UiNode>,
    inspector: Handle<UiNode>,
    definition_container: Rc<PropertyEditorDefinitionContainer>,
}

fn create_ui(engine: &mut Engine) -> Interface {
    let ctx = &mut engine.user_interface.build_ctx();

    let debug_text = TextBuilder::new(WidgetBuilder::new()).build(ctx);

    let definition_container = Rc::new(PropertyEditorDefinitionContainer::new());

    let inspector;
    WindowBuilder::new(WidgetBuilder::new().with_width(400.0))
        .with_title(WindowTitle::text("Inspector"))
        .with_content({
            inspector = InspectorBuilder::new(
                WidgetBuilder::new().with_desired_position(Vector2::new(200.0, 200.0)),
            )
            .build(ctx);
            inspector
        })
        .build(ctx);

    Interface {
        debug_text,
        inspector,
        definition_container,
    }
}

struct GameScene {
    scene: Scene,
    model_handle: Handle<Node>,
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

    let model_resource = resource_manager
        .request::<Model, _>("examples/data/mutant/mutant.FBX")
        .await
        .unwrap();

    let model_handle = model_resource.instantiate(&mut scene);

    let walk_animation_resource = resource_manager
        .request::<Model, _>("examples/data/mutant/walk.fbx")
        .await
        .unwrap();

    walk_animation_resource.retarget_animations(model_handle, &mut scene.graph);

    GameScene {
        scene,
        model_handle,
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let graphics_context_params = GraphicsContextParams {
        window_attributes: WindowAttributes {
            title: "Example - User Interface".to_string(),
            resizable: true,
            ..Default::default()
        },
        vsync: false,
    };
    let serialization_context = Arc::new(SerializationContext::new());
    let mut engine = Engine::new(EngineInitParams {
        graphics_context_params,
        resource_manager: ResourceManager::new(),
        serialization_context,
    })
    .unwrap();

    // Create simple user interface that will show some useful info.
    let interface = create_ui(&mut engine);

    // Create test scene.
    let GameScene {
        scene,
        model_handle,
    } = fyrox::core::futures::executor::block_on(create_scene(engine.resource_manager.clone()));

    let user_interface = &mut engine.user_interface;
    scene.graph[model_handle].as_reflect(&mut |object| {
        let inspector_context = InspectorContext::from_object(
            object,
            &mut user_interface.build_ctx(),
            interface.definition_container.clone(),
            None,
            1,
            0,
            true,
            Default::default(),
        );
        user_interface.send_message(InspectorMessage::context(
            interface.inspector,
            MessageDirection::ToWidget,
            inspector_context,
        ));
    });

    // Add scene to engine - engine will take ownership over scene and will return
    // you a handle to scene which can be used later on to borrow it and do some
    // actions you need.
    let scene_handle = engine.scenes.add(scene);

    let mut previous = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut lag = 0.0;

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

                        if let GraphicsContext::Initialized(ref ctx) = engine.graphics_context {
                            let fps = ctx.renderer.get_statistics().frames_per_second;
                            engine.user_interface.send_message(TextMessage::text(
                                interface.debug_text,
                                MessageDirection::ToWidget,
                                format!("Example 04 - User Interface\nFPS: {}", fps),
                            ));
                        }

                        engine.update(fixed_timestep, control_flow, &mut lag, Default::default());
                        lag -= fixed_timestep;
                    }

                    // It is very important to "pump" messages from UI. This our main point where we communicate
                    // with user interface. As you saw earlier, there is no callbacks on UI elements, instead we
                    // use messages to get information from UI elements. This provides perfect decoupling of logic
                    // from UI elements and works well with borrow checker.
                    let scene = &mut engine.scenes[scene_handle];
                    while let Some(ui_message) = engine.user_interface.poll_message() {
                        if ui_message.destination() == interface.inspector
                            && ui_message.direction() == MessageDirection::FromWidget
                        {
                            if let Some(InspectorMessage::PropertyChanged(args)) =
                                ui_message.data::<InspectorMessage>()
                            {
                                if let FieldKind::Object(ref value) = args.value {
                                    match args.name.as_str() {
                                        "local_scale" => {
                                            value.cast_clone::<Vector3<f32>>(&mut |result| {
                                                scene.graph[model_handle]
                                                    .local_transform_mut()
                                                    .set_scale(result.unwrap());
                                            });
                                        }
                                        "visibility" => value.cast_clone::<bool>(&mut |result| {
                                            scene.graph[model_handle]
                                                .set_visibility(result.unwrap());
                                        }),
                                        "local_rotation" => {
                                            value.cast_clone::<UnitQuaternion<f32>>(
                                                &mut |result| {
                                                    scene.graph[model_handle]
                                                        .local_transform_mut()
                                                        .set_rotation(result.unwrap());
                                                },
                                            );
                                        }
                                        "name" => value.cast_clone::<String>(&mut |result| {
                                            scene.graph[model_handle].set_name(result.unwrap())
                                        }),
                                        // TODO: Add rest of properties.
                                        _ => (),
                                    }
                                }
                            }
                        }
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
                        WindowEvent::KeyboardInput { event: input, .. } => {
                            if input.state == ElementState::Pressed
                                && input.physical_key == KeyCode::Escape
                            {
                                *control_flow = ControlFlow::Exit;
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
        })
        .unwrap();
}
