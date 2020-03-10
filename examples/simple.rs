extern crate rg3d;

use std::time::Instant;

use rg3d::{
    scene::{
        base::{
            AsBase,
            BaseBuilder,
        },
        transform::TransformBuilder,
        camera::CameraBuilder,
        node::Node,
        Scene,
    },
    engine::resource_manager::ResourceManager,
    gui::{
        widget::WidgetBuilder,
        text::TextBuilder,
        node::StubNode,
    },
    event::{
        Event,
        WindowEvent,
        DeviceEvent,
        VirtualKeyCode,
        ElementState
    },
    event_loop::{
        EventLoop,
        ControlFlow,
    },
    core::{
        color::Color,
        pool::Handle,
        math::{
            vec3::Vec3,
            quat::Quat,
        },
    },
    animation::Animation
};

// Create our own engine type aliases. These specializations are needed
// because engine provides a way to extend UI with custom nodes and messages.
type GameEngine = rg3d::engine::Engine<(), StubNode>;
type UserInterface = rg3d::gui::UserInterface<(), StubNode>;
type UiNode = rg3d::gui::node::UINode<(), StubNode>;

fn create_ui(ui: &mut UserInterface) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new())
        .build(ui)
}

struct GameScene {
    scene: Scene,
    model_handle: Handle<Node>,
    walk_animation: Handle<Animation>,
}

fn create_scene(resource_manager: &mut ResourceManager) -> GameScene {
    let mut scene = Scene::new();

    // Camera is our eyes in the world - you won't see anything without it.
    let camera = CameraBuilder::new(BaseBuilder::new()
        .with_local_transform(TransformBuilder::new()
            .with_local_position(Vec3::new(0.0, 6.0, -12.0))
            .build()))
        .build();

    scene.graph.add_node(Node::Camera(camera));

    // Load model resource. Is does *not* adds anything to our scene - it just loads a
    // resource then can be used later on to instantiate models from it on scene. Why
    // loading of resource is separated from instantiation? Because there it is too
    // inefficient to load a resource every time you trying to create instance of it -
    // much more efficient is to load it one and then make copies of it. In case of
    // models it is very efficient because single vertex and index buffer can be used
    // for all models instances, so memory footprint on GPU will be lower.
    let model_resource = resource_manager.request_model("examples/data/mutant.FBX").unwrap();

    // Instantiate model on scene - but only geometry, without any animations.
    // Instantiation is a process of embedding model resource data in desired scene.
    let model_handle = model_resource.lock()
        .unwrap()
        .instantiate_geometry(&mut scene);

    // Now we have whole sub-graph instantiated, we can start modifying model instance.
    scene.graph
        .get_mut(model_handle)
        .base_mut()
        .get_local_transform_mut()
        // Our model is too big, fix it by scale.
        .set_scale(Vec3::new(0.05, 0.05, 0.05));

    // Add simple animation for our model. Animations are loaded from model resources -
    // this is because animation is a set of skeleton bones with their own transforms.
    let walk_animation_resource = resource_manager.request_model("examples/data/walk.fbx").unwrap();

    // Once animation resource is loaded it must be re-targeted to our model instance.
    // Why? Because animation in *resource* uses information about *resource* bones,
    // not model instance bones, retarget_animations maps animations of each bone on
    // model instance so animation will know about nodes it should operate on.
    let walk_animation = *walk_animation_resource
        .lock()
        .unwrap()
        .retarget_animations(model_handle, &mut scene)
        .get(0)
        .unwrap();

    GameScene {
        scene,
        model_handle,
        walk_animation,
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

    let mut engine = GameEngine::new(window_builder, &event_loop).unwrap();

    // Prepare resource manager - it must be notified where to search textures. When engine
    // loads model resource it automatically tries to load textures it uses. But since most
    // model formats store absolute paths, we can't use them as direct path to load texture
    // instead we telling engine to search textures in given folder.
    engine.resource_manager.set_textures_path("examples/data");

    // Create simple user interface that will show some useful info.
    let debug_text = create_ui(&mut engine.user_interface);

    // Create test scene.
    let GameScene { scene, model_handle, walk_animation } = create_scene(&mut engine.resource_manager);

    // Add scene to engine - engine will take ownership over scene and will return
    // you a handle to scene which can be used later on to borrow it and do some
    // actions you need.
    let scene_handle = engine.scenes.add(scene);

    // Set ambient light.
    engine.renderer.set_ambient_color(Color::opaque(200, 200, 200));

    let clock = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    // We will rotate model using keyboard input.
    let mut model_angle = 180.0f32.to_radians();

    // Create input controller - it will hold information about needed actions.
    let mut input_controller = InputController { rotate_left: false, rotate_right: false };

    // Finally run our event loop which will respond to OS and window events and update
    // engine state accordingly. Engine lets you to decide which event should be handled,
    // this is minimal working example if how it should be.
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                // This main game loop - it has fixed time step which means that game
                // code will run at fixed speed even if renderer can't give you desired
                // 60 fps.
                let mut dt = clock.elapsed().as_secs_f64() - elapsed_time;
                while dt >= fixed_timestep as f64 {
                    dt -= fixed_timestep as f64;
                    elapsed_time += fixed_timestep as f64;

                    // ************************
                    // Put your game logic here.
                    // ************************

                    // Use stored scene handle to borrow a mutable reference of scene in
                    // engine.
                    let scene = engine.scenes.get_mut(scene_handle);

                    // Our animation must be applied to scene explicitly, otherwise
                    // it will have no effect.
                    scene.animations
                        .get_mut(walk_animation)
                        .get_pose()
                        .apply(&mut scene.graph);

                    // Rotate model according to input controller state.
                    if input_controller.rotate_left {
                        model_angle -= 5.0f32.to_radians();
                    } else if input_controller.rotate_right {
                        model_angle += 5.0f32.to_radians();
                    }

                    scene.graph
                        .get_mut(model_handle)
                        .base_mut()
                        .get_local_transform_mut()
                        .set_rotation(Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), model_angle));

                    if let UiNode::Text(text) = engine.user_interface.node_mut(debug_text) {
                        let fps = engine.renderer.get_statistics().frames_per_second;
                        text.set_text(format!("Example - Model\nUse [A][D] keys to rotate model.\nFPS: {}", fps));
                    }

                    engine.update(fixed_timestep);
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
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit
                    }
                    WindowEvent::Resized(size) => {
                        // It is very important to handle Resized event from window, because
                        // renderer knows nothing about window size - it must be notified
                        // directly when window size has changed.
                        engine.renderer.set_frame_size(size.into()).unwrap();
                    }
                    _ => ()
                }
            }
            Event::DeviceEvent { event, .. } => {
                if let DeviceEvent::Key(key) = event {
                    if let Some(key_code) = key.virtual_keycode {
                        match key_code {
                            VirtualKeyCode::A => input_controller.rotate_left = key.state == ElementState::Pressed,
                            VirtualKeyCode::D => input_controller.rotate_right = key.state == ElementState::Pressed,
                            _ => ()
                        }
                    }
                }
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}