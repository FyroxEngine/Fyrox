//! Example - Terrain.

extern crate rg3d;

pub mod shared;

use crate::shared::create_camera;
use rg3d::core::algebra::Vector2;
use rg3d::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        color::Color,
        pool::Handle,
        rand::Rng,
    },
    engine::resource_manager::ResourceManager,
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        message::{MessageDirection, TextMessage},
        node::StubNode,
        text::TextBuilder,
        widget::WidgetBuilder,
    },
    rand::thread_rng,
    scene::{
        base::BaseBuilder,
        light::{BaseLightBuilder, PointLightBuilder},
        node::Node,
        terrain::{Brush, BrushKind, BrushMode, LayerDefinition, TerrainBuilder},
        transform::TransformBuilder,
        Scene,
    },
    utils::translate_event,
};
use std::time::Instant;

// Create our own engine type aliases. These specializations are needed
// because engine provides a way to extend UI with custom nodes and messages.
type GameEngine = rg3d::engine::Engine<(), StubNode>;
type UiNode = rg3d::gui::node::UINode<(), StubNode>;
type BuildContext<'a> = rg3d::gui::BuildContext<'a, (), StubNode>;

fn create_ui(ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new()).build(ctx)
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
    let model_handle = create_camera(
        resource_manager.clone(),
        Vector3::new(32.0, 8.0, 32.0),
        &mut scene.graph,
    )
    .await;

    // Add terrain.
    let terrain = TerrainBuilder::new(BaseBuilder::new())
        .with_layers(vec![
            LayerDefinition {
                diffuse_texture: Some(
                    resource_manager.request_texture("examples/data/Grass_DiffuseColor.jpg"),
                ),
                normal_texture: Some(
                    resource_manager.request_texture("examples/data/Grass_Normal.jpg"),
                ),
                specular_texture: None,
                roughness_texture: None,
                height_texture: None,
                tile_factor: Vector2::new(10.0, 10.0),
            },
            LayerDefinition {
                diffuse_texture: Some(
                    resource_manager.request_texture("examples/data/Rock_DiffuseColor.jpg"),
                ),
                normal_texture: Some(
                    resource_manager.request_texture("examples/data/Rock_Normal.jpg"),
                ),
                specular_texture: None,
                roughness_texture: None,
                height_texture: None,
                tile_factor: Vector2::new(10.0, 10.0),
            },
        ])
        .build(&mut scene.graph);

    let terrain = scene.graph[terrain].as_terrain_mut();

    for _ in 0..60 {
        let x = thread_rng().gen_range(4.0..60.00);
        let z = thread_rng().gen_range(4.0..60.00);
        let radius = thread_rng().gen_range(1.0..3.0);
        let height = thread_rng().gen_range(1.0..4.0);

        // Draw something on the terrain.

        // Pull terrain.
        terrain.draw(&Brush {
            center: Vector3::new(x, 0.0, z),
            kind: BrushKind::Circle { radius },
            mode: BrushMode::AlternateHeightMap { amount: height },
        });

        // Draw rock texture on top.
        terrain.draw(&Brush {
            center: Vector3::new(x, 0.0, z),
            kind: BrushKind::Circle { radius },
            mode: BrushMode::DrawOnMask { layer: 1 },
        });
    }

    // Add some light.
    PointLightBuilder::new(BaseLightBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(Vector3::new(0.0, 12.0, 0.0))
                .build(),
        ),
    ))
    .with_radius(20.0)
    .build(&mut scene.graph);

    GameScene {
        scene,
        model_handle,
    }
}

struct InputController {
    rotate_left: bool,
    rotate_right: bool,
}

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_title("Example - Model")
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
    let debug_text = create_ui(&mut engine.user_interface.build_ctx());

    // Create test scene.
    let GameScene {
        scene,
        model_handle,
    } = rg3d::core::futures::executor::block_on(create_scene(engine.resource_manager.clone()));

    // Add scene to engine - engine will take ownership over scene and will return
    // you a handle to scene which can be used later on to borrow it and do some
    // actions you need.
    let scene_handle = engine.scenes.add(scene);

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

                    let fps = engine.renderer.get_statistics().frames_per_second;
                    let text = format!(
                        "Example 01 - Simple Scene\nUse [A][D] keys to rotate model.\nFPS: {}",
                        fps
                    );
                    engine.user_interface.send_message(TextMessage::text(
                        debug_text,
                        MessageDirection::ToWidget,
                        text,
                    ));

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
