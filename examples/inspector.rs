//! Inspector testbed (WIP)

pub mod shared;

use crate::shared::create_camera;
use rg3d::gui::message::InspectorMessage;
use rg3d::{
    animation::Animation,
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        color::Color,
        pool::Handle,
    },
    engine::resource_manager::{MaterialSearchOptions, ResourceManager},
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        inspector::{
            Inspect, InspectorBuilder, InspectorContext, PropertyDefinitionContainer, PropertyInfo,
        },
        message::Vec3EditorMessage,
        message::{
            MessageDirection, NumericUpDownMessage, TextBoxMessage, TextMessage, UiMessageData,
        },
        node::StubNode,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
    },
    scene::{node::Node, Scene},
    utils::{
        log::{Log, MessageKind},
        translate_event,
    },
};
use std::time::Instant;

// Create our own engine type aliases. These specializations are needed
// because engine provides a way to extend UI with custom nodes and messages.
type GameEngine = rg3d::engine::Engine<(), StubNode>;
type UiNode = rg3d::gui::node::UINode<(), StubNode>;

struct Interface {
    debug_text: Handle<UiNode>,
    view_model: ViewModel,
    inspector: Handle<UiNode>,
    inspector_context: InspectorContext<(), StubNode>,
}

struct ViewModel {
    // In Euler angles.
    rotation: Vector3<f32>,
    scale: f32,
    name: String,
}

impl Inspect for ViewModel {
    fn properties(&self) -> Vec<PropertyInfo> {
        vec![
            PropertyInfo {
                name: "Rotation",
                group: "Transform",
                value: &self.rotation,
            },
            PropertyInfo {
                name: "Scale",
                group: "Transform",
                value: &self.scale,
            },
            PropertyInfo {
                name: "Model Name",
                group: "Common",
                value: &self.name,
            },
        ]
    }
}

fn create_ui(engine: &mut GameEngine) -> Interface {
    let ctx = &mut engine.user_interface.build_ctx();

    let debug_text = TextBuilder::new(WidgetBuilder::new()).build(ctx);

    let view_model = ViewModel {
        rotation: Vector3::new(0.0, 180.0, 0.0),
        scale: 0.05,
        name: "Model".to_string(),
    };

    let definition_container = PropertyDefinitionContainer::new();

    let inspector_context = InspectorContext::from_object(&view_model, ctx, &definition_container);

    let inspector;
    WindowBuilder::new(WidgetBuilder::new())
        .with_title(WindowTitle::text("Inspector"))
        .with_content({
            inspector = InspectorBuilder::new(
                WidgetBuilder::new().with_desired_position(Vector2::new(200.0, 200.0)),
            )
            .with_property_definitions(definition_container)
            .with_context(inspector_context.clone())
            .build(ctx);
            inspector
        })
        .build(ctx);

    Interface {
        debug_text,
        view_model,
        inspector_context,
        inspector,
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

    let model_resource = resource_manager
        .request_model(
            "examples/data/mutant/mutant.FBX",
            MaterialSearchOptions::RecursiveUp,
        )
        .await
        .unwrap();

    let model_handle = model_resource.instantiate_geometry(&mut scene);

    let walk_animation_resource = resource_manager
        .request_model(
            "examples/data/mutant/walk.fbx",
            MaterialSearchOptions::RecursiveUp,
        )
        .await
        .unwrap();

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

    let mut engine = GameEngine::new(window_builder, &event_loop, true).unwrap();

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
                let scene = &mut engine.scenes[scene_handle];
                while let Some(ui_message) = engine.user_interface.poll_message() {
                    if ui_message.destination() == interface.inspector
                        && ui_message.direction() == MessageDirection::FromWidget
                    {
                        if let UiMessageData::Inspector(InspectorMessage::PropertyChanged(args)) =
                            ui_message.data()
                        {
                            match args.name.as_str() {
                                "Scale" => {
                                    let value = args.cast_value::<f32>().unwrap();
                                    scene.graph[model_handle]
                                        .local_transform_mut()
                                        .set_scale(Vector3::new(*value, *value, *value));
                                }
                                "Rotation" => {
                                    let value = args.cast_value::<Vector3<f32>>().unwrap();
                                    scene.graph[model_handle]
                                        .local_transform_mut()
                                        .set_rotation(UnitQuaternion::from_euler_angles(
                                            value.z, value.x, value.y,
                                        ));
                                }
                                "Name" => {
                                    let text = args.cast_value::<String>().unwrap();
                                    scene.graph[model_handle].set_name(text.clone());
                                }
                                _ => (),
                            }
                        }
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
                        if let Err(e) = engine.renderer.set_frame_size(size.into()) {
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
