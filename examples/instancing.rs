//! Example 10. Instancing.
//!
//! Difficulty: Easy.
//!
//! This example shows how to create simple scene with lots of animated models with low performance
//! impact.

extern crate rg3d;

pub mod shared;

use crate::shared::create_camera;
use rg3d::{
    animation::Animation,
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector3},
        color::Color,
        pool::Handle,
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
    rand::Rng,
    renderer::{
        surface::{SurfaceBuilder, SurfaceSharedData},
        QualitySettings,
    },
    scene::{
        base::BaseBuilder,
        light::{BaseLightBuilder, PointLightBuilder},
        mesh::MeshBuilder,
        node::Node,
        transform::TransformBuilder,
        Scene,
    },
    utils::translate_event,
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
    camera: Handle<Node>,
    animations: Vec<Handle<Animation>>,
}

async fn create_scene(resource_manager: ResourceManager) -> GameScene {
    let mut scene = Scene::new();

    // Camera is our eyes in the world - you won't see anything without it.
    let camera = create_camera(
        resource_manager.clone(),
        Vector3::new(0.0, 32.0, -140.0),
        &mut scene.graph,
    )
    .await;

    // Load model and animation resource in parallel. Is does *not* adds anything to
    // our scene - it just loads a resource then can be used later on to instantiate
    // models from it on scene. Why loading of resource is separated from instantiation?
    // Because it is too inefficient to load a resource every time you trying to
    // create instance of it - much more efficient is to load it once and then make copies
    // of it. In case of models it is very efficient because single vertex and index buffer
    // can be used for all models instances, so memory footprint on GPU will be lower.
    let (model_resource, walk_animation_resource) = rg3d::futures::join!(
        resource_manager.request_model("examples/data/mutant.FBX"),
        resource_manager.request_model("examples/data/walk.fbx")
    );

    let mut animations = Vec::new();

    for z in -10..10 {
        for x in -10..10 {
            // Instantiate model on scene - but only geometry, without any animations.
            // Instantiation is a process of embedding model resource data in desired scene.
            let model_handle = model_resource
                .clone()
                .unwrap()
                .instantiate_geometry(&mut scene);

            // Now we have whole sub-graph instantiated, we can start modifying model instance.
            scene.graph[model_handle]
                .local_transform_mut()
                // Our model is too big, fix it by scale.
                .set_scale(Vector3::new(0.05, 0.05, 0.05))
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &Vector3::y_axis(),
                    180.0f32.to_radians(),
                ))
                .set_position(Vector3::new((x as f32) * 7.0, 0.0, (z as f32) * 7.0));

            // Add simple animation for our model. Animations are loaded from model resources -
            // this is because animation is a set of skeleton bones with their own transforms.
            // Once animation resource is loaded it must be re-targeted to our model instance.
            // Why? Because animation in *resource* uses information about *resource* bones,
            // not model instance bones, retarget_animations maps animations of each bone on
            // model instance so animation will know about nodes it should operate on.
            let walk_animation = *walk_animation_resource
                .clone()
                .unwrap()
                .retarget_animations(model_handle, &mut scene)
                .get(0)
                .unwrap();

            scene
                .animations
                .get_mut(walk_animation)
                .set_speed(rg3d::rand::thread_rng().gen_range(0.8..1.2));

            animations.push(walk_animation);
        }
    }

    // Add point light with shadows.
    PointLightBuilder::new(BaseLightBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(Vector3::new(0.0, 30.0, -50.0))
                .build(),
        ),
    ))
    .with_radius(100.0)
    .build(&mut scene.graph);

    // Add floor.
    MeshBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(Vector3::new(0.0, -0.25, 0.0))
                .build(),
        ),
    )
    .with_surfaces(vec![SurfaceBuilder::new(Arc::new(RwLock::new(
        SurfaceSharedData::make_cube(Matrix4::new_nonuniform_scaling(&Vector3::new(
            300.0, 0.25, 300.0,
        ))),
    )))
    .with_diffuse_texture(resource_manager.request_texture("examples/data/concrete2.dds"))
    .build()])
    .build(&mut scene.graph);

    GameScene {
        scene,
        camera,
        animations,
    }
}

struct InputController {
    rotate_left: bool,
    rotate_right: bool,
}

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_title("Example - Instancing")
        .with_resizable(true);

    let mut engine = GameEngine::new(window_builder, &event_loop, false).unwrap();

    let mut settings = QualitySettings::ultra();
    settings.point_shadows_distance = 1000.0;
    engine.renderer.set_quality_settings(&settings).unwrap();

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
        camera,
        animations,
    } = rg3d::futures::executor::block_on(create_scene(engine.resource_manager.clone()));

    // Add scene to engine - engine will take ownership over scene and will return
    // you a handle to scene which can be used later on to borrow it and do some
    // actions you need.
    let scene_handle = engine.scenes.add(scene);

    // Set ambient light.
    engine
        .renderer
        .set_ambient_color(Color::opaque(100, 100, 100));

    let clock = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    // We will rotate model using keyboard input.
    let mut camera_angle = 0.0f32.to_radians();

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

                    // Use stored scene handle to borrow a mutable reference of scene in
                    // engine.
                    let scene = &mut engine.scenes[scene_handle];

                    // Our animations must be applied to scene explicitly, otherwise
                    // it will have no effect.
                    for &animation in animations.iter() {
                        scene
                            .animations
                            .get_mut(animation)
                            .get_pose()
                            .apply(&mut scene.graph);
                    }

                    // Rotate model according to input controller state.
                    if input_controller.rotate_left {
                        camera_angle -= 5.0f32.to_radians();
                    } else if input_controller.rotate_right {
                        camera_angle += 5.0f32.to_radians();
                    }

                    scene.graph[camera].local_transform_mut().set_rotation(
                        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), camera_angle),
                    );

                    engine.update(fixed_timestep);
                }

                let text = format!(
                    "Example 10 - Instancing\n\
                    Models count: {}\n\
                    Use [A][D] keys to rotate camera.\n\
                    {}",
                    animations.len(),
                    engine.renderer.get_statistics()
                );
                engine.user_interface.send_message(TextMessage::text(
                    debug_text,
                    MessageDirection::ToWidget,
                    text,
                ));

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
