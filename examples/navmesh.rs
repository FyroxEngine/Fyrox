//! Example 01. Simple scene.
//!
//! Difficulty: Easy.
//!
//! This example shows how to create simple scene with animated model.

extern crate rg3d;

pub mod shared;

use crate::shared::create_camera;
use rg3d::{
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        arrayvec::ArrayVec,
        color::Color,
        math::PositionProvider,
        pool::Handle,
    },
    dpi::LogicalPosition,
    engine::resource_manager::ResourceManager,
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        message::{MessageDirection, TextMessage},
        node::StubNode,
        text::TextBuilder,
        widget::WidgetBuilder,
    },
    renderer::surface::{SurfaceBuilder, SurfaceSharedData},
    scene::{
        base::BaseBuilder,
        mesh::MeshBuilder,
        node::Node,
        physics::{Intersection, RayCastOptions},
        transform::TransformBuilder,
        Line, Scene,
    },
    utils::{navmesh::NavmeshAgent, translate_event},
};
use std::{
    sync::{Arc, RwLock},
    time::Instant,
};

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
    agent: Handle<Node>,
    cursor: Handle<Node>,
    camera: Handle<Node>,
}

async fn create_scene(resource_manager: ResourceManager) -> GameScene {
    let mut scene = Scene::new();

    // Camera is our eyes in the world - you won't see anything without it.
    let camera = create_camera(
        resource_manager.clone(),
        Vector3::new(4.0, 8.0, 0.0),
        &mut scene.graph,
    )
    .await;

    scene.graph[camera]
        .local_transform_mut()
        .set_rotation(UnitQuaternion::from_axis_angle(
            &Vector3::x_axis(),
            90.0f32.to_radians(),
        ));

    resource_manager
        .request_model("examples/data/navmesh_scene.rgs")
        .await
        .unwrap()
        .instantiate_geometry(&mut scene);

    let cursor = MeshBuilder::new(BaseBuilder::new())
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(RwLock::new(
            SurfaceSharedData::make_sphere(10, 10, 0.1),
        )))
        .with_color(Color::opaque(255, 0, 0))
        .build()])
        .build(&mut scene.graph);

    let agent = MeshBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_scale(Vector3::new(1.0, 2.0, 1.0))
                .build(),
        ),
    )
    .with_surfaces(vec![SurfaceBuilder::new(Arc::new(RwLock::new(
        SurfaceSharedData::make_sphere(10, 10, 0.2),
    )))
    .with_color(Color::opaque(0, 200, 0))
    .build()])
    .build(&mut scene.graph);

    GameScene {
        scene,
        cursor,
        agent,
        camera,
    }
}

struct InputController {
    rotate_left: bool,
    rotate_right: bool,
}

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_title("Example 12 - Navigation Mesh")
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
        agent,
        cursor,
        camera,
    } = rg3d::futures::executor::block_on(create_scene(engine.resource_manager.clone()));

    // Add scene to engine - engine will take ownership over scene and will return
    // you a handle to scene which can be used later on to borrow it and do some
    // actions you need.
    let scene_handle = engine.scenes.add(scene);

    // Set ambient light.
    engine
        .renderer
        .set_ambient_color(Color::opaque(200, 200, 200));

    let clock = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    // Create input controller - it will hold information about needed actions.
    let mut input_controller = InputController {
        rotate_left: false,
        rotate_right: false,
    };

    let mut mouse_position = Vector2::default();

    let mut navmesh_agent = NavmeshAgent::new();
    let mut target_position = Vector3::default();

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

                    scene.drawing_context.clear_lines();

                    let ray = scene.graph[camera]
                        .as_camera()
                        .make_ray(mouse_position, engine.renderer.get_frame_bounds());

                    let mut buffer = ArrayVec::<[Intersection; 64]>::new();
                    scene.physics.cast_ray(
                        RayCastOptions {
                            ray,
                            max_len: 9999.0,
                            groups: Default::default(),
                            sort_results: true,
                        },
                        &mut buffer,
                    );

                    if let Some(first) = buffer.first() {
                        target_position = first.position.coords;
                        scene.graph[cursor]
                            .local_transform_mut()
                            .set_position(target_position);
                    }

                    let navmesh = scene.navmeshes.iter_mut().next().unwrap();

                    let last = std::time::Instant::now();
                    navmesh_agent.set_target(target_position);
                    let _ = navmesh_agent.update(fixed_timestep, navmesh);
                    let agent_time = std::time::Instant::now() - last;

                    scene.graph[agent]
                        .local_transform_mut()
                        .set_position(navmesh_agent.position());

                    // Debug drawing.

                    for pt in navmesh.vertices() {
                        for neighbour in pt.neighbours() {
                            scene.drawing_context.add_line(Line {
                                begin: pt.position(),
                                end: navmesh.vertices()[*neighbour as usize].position(),
                                color: Color::opaque(0, 0, 200),
                            });
                        }
                    }

                    for pts in navmesh_agent.path().windows(2) {
                        scene.drawing_context.add_line(Line {
                            begin: pts[0],
                            end: pts[1],
                            color: Color::opaque(255, 0, 0),
                        });
                    }

                    let fps = engine.renderer.get_statistics().frames_per_second;
                    let text = format!(
                        "Example 12 - Navigation Mesh\nFPS: {}\nAgent time: {:?}",
                        fps, agent_time
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
                    WindowEvent::CursorMoved { position, .. } => {
                        let p: LogicalPosition<f32> =
                            position.to_logical(engine.get_window().scale_factor());
                        mouse_position = Vector2::new(p.x as f32, p.y as f32);
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
