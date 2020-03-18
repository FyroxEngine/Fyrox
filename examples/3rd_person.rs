//! Example 03. 3rd person walk simulator.
//!
//! Difficulty: Advanced.
//!
//! This example based on async example, because it requires to load decent amount of
//! resources which might be slow on some machines.
//!
//! In this example we'll create simple 3rd person game with character that can idle,
//! walk, or jump.
//!
//! Also this example demonstrates the power of animation blending machines. Animation
//! blending machines are used in all modern games to create complex animations from set
//! of simple ones.
//!
//! TODO: Improve explanations. Some places can be explained better.
//!
//! Known bugs: Sometimes character will jump, but jumping animations is not playing.
//!
//! Possible improvements:
//!  - Smart camera - camera which will not penetrate walls.
//!  - Separate animation machines for upper and lower body - upper machine might be
//!    for combat, lower - for locomotion.
//!  - Tons of them, this is simple example after all.

extern crate rg3d;

use std::{
    time::Instant,
    sync::{Arc, Mutex},
    path::Path,
};
use rg3d::{
    core::{
        math::{
            SmoothAngle,
            vec3::Vec3,
            quat::Quat,
            vec2::Vec2,
        },
        color::Color,
        pool::Handle,
    },
    physics::{
        rigid_body::RigidBody,
        convex_shape::{
            ConvexShape,
            CapsuleShape,
            Axis,
        },
    },
    utils::{
        mesh_to_static_geometry,
        translate_event,
    },
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
        progress_bar::ProgressBarBuilder,
        grid::{GridBuilder, Row, Column},
        VerticalAlignment,
        Control,
        Thickness,
        HorizontalAlignment,
    },
    event::{
        Event,
        WindowEvent,
        DeviceEvent,
        VirtualKeyCode,
        ElementState,
    },
    event_loop::{
        EventLoop,
        ControlFlow,
    },
    animation::{
        Animation,
        machine::{
            Machine,
            PoseNode,
            State,
            Transition,
            Parameter,
        },
        AnimationSignal
    }
};

// Create our own engine type aliases. These specializations are needed
// because engine provides a way to extend UI with custom nodes and messages.
type GameEngine = rg3d::engine::Engine<(), StubNode>;
type UserInterface = rg3d::gui::UserInterface<(), StubNode>;
type UiNode = rg3d::gui::node::UINode<(), StubNode>;

struct Interface {
    root: Handle<UiNode>,
    debug_text: Handle<UiNode>,
    progress_bar: Handle<UiNode>,
    progress_text: Handle<UiNode>,
}

fn create_ui(ui: &mut UserInterface, screen_size: Vec2) -> Interface {
    let debug_text;
    let progress_bar;
    let progress_text;
    let root = GridBuilder::new(WidgetBuilder::new()
        .with_width(screen_size.x)
        .with_height(screen_size.y)
        .with_child({
            debug_text = TextBuilder::new(WidgetBuilder::new()
                .on_row(0)
                .on_column(0))
                .build(ui);
            debug_text
        })
        .with_child({
            progress_bar = ProgressBarBuilder::new(WidgetBuilder::new()
                .on_row(1)
                .on_column(1))
                .build(ui);
            progress_bar
        })
        .with_child({
            progress_text = TextBuilder::new(WidgetBuilder::new()
                .on_column(1)
                .on_row(0)
                .with_margin(Thickness::bottom(20.0))
                .with_vertical_alignment(VerticalAlignment::Bottom))
                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                .build(ui);
            progress_text
        }))
        .add_row(Row::stretch())
        .add_row(Row::strict(30.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::strict(200.0))
        .add_column(Column::stretch())
        .build(ui);

    Interface {
        root,
        debug_text,
        progress_bar,
        progress_text,
    }
}

struct SceneLoadResult {
    scene: Scene,
    player: Player,
}

struct GameScene {
    scene: Handle<Scene>,
    player: Player,
}

struct SceneLoadContext {
    data: Option<SceneLoadResult>,
    message: String,
    progress: f32,
}

impl SceneLoadContext {
    pub fn report_progress(&mut self, progress: f32, message: &str) {
        self.progress = progress;
        self.message = message.to_owned();
        println!("Loading progress: {}% - {}", progress * 100.0, message);
    }
}

// Small helper function that loads animation from given file and retargets it to given model.
fn load_animation<P: AsRef<Path>>(
    path: P,
    scene: &mut Scene,
    model: Handle<Node>,
    resource_manager: &mut ResourceManager,
) -> Handle<Animation> {
    *resource_manager
        .request_model(path)
        .unwrap()
        .lock()
        .unwrap()
        .retarget_animations(model, scene)
        .get(0)
        .unwrap()
}

// Small helper function that creates PlayAnimation machine node and creates
// state from it.
fn create_play_animation_state<P: AsRef<Path>>(
    path: P,
    name: &str,
    machine: &mut Machine,
    scene: &mut Scene,
    model: Handle<Node>,
    resource_manager: &mut ResourceManager,
) -> (Handle<Animation>, Handle<State>) {
    // First of all load required animation and apply it on model.
    let animation = load_animation(path, scene, model, resource_manager);

    // Create PlayAnimation machine node. What is that "machine node"? First of all
    // animation blending machine is a graph, and it has two types of nodes:
    // 1) Animation pose nodes (PoseNode) which provides poses for states.
    // 2) State - a node that uses connected pose for transitions. Transitions
    //    can be done only from state to state. Other nodes are just provides animations.
    let node = machine.add_node(PoseNode::make_play_animation(animation));

    // Finally use new node and create state from it.
    let state = machine.add_state(State::new(name, node));

    (animation, state)
}

pub struct LocomotionMachine {
    machine: Machine,
    jump_animation: Handle<Animation>,
}

pub struct LocomotionMachineInput {
    is_walking: bool,
    is_jumping: bool,
}

impl LocomotionMachine {
    // Define names for Rule parameters. Rule parameters are used by transitions
    // to check whether transition can be performed or not.
    const WALK_TO_IDLE: &'static str = "WalkToIdle";
    const WALK_TO_JUMP: &'static str = "WalkToJump";
    const IDLE_TO_WALK: &'static str = "IdleToWalk";
    const IDLE_TO_JUMP: &'static str = "IdleToJump";
    const JUMP_TO_IDLE: &'static str = "JumpToIdle";

    const JUMP_SIGNAL: u64 = 1;

    fn new(scene: &mut Scene, model: Handle<Node>, resource_manager: &mut ResourceManager) -> Self {
        let mut machine = Machine::new();

        let (_, walk_state) = create_play_animation_state("examples/data/walk.fbx", "Walk", &mut machine, scene, model, resource_manager);
        let (_, idle_state) = create_play_animation_state("examples/data/idle.fbx", "Idle", &mut machine, scene, model, resource_manager);

        // Jump animation is a bit special - it must be non-looping.
        let (jump_animation, jump_state) = create_play_animation_state("examples/data/jump.fbx", "Jump", &mut machine, scene, model, resource_manager);
        scene.animations
            .get_mut(jump_animation)
            // Actual jump (applying force to physical body) must be synced with animation
            // so we have to be notified about this. This is where signals come into play
            // you can assign any signal in animation timeline and then in update loop you
            // can iterate over them and react appropriately.
            .add_signal(AnimationSignal::new(Self::JUMP_SIGNAL, 0.32))
            .set_loop(false);

        // Add transitions between states. This is the "heart" of animation blending state machine
        // it defines how it will respond to input parameters.
        machine
            .add_transition(Transition::new("Walk->Idle", walk_state, idle_state, 0.30, Self::WALK_TO_IDLE))
            .add_transition(Transition::new("Walk->Jump", walk_state, jump_state, 0.20, Self::WALK_TO_JUMP))
            .add_transition(Transition::new("Idle->Walk", idle_state, walk_state, 0.30, Self::IDLE_TO_WALK))
            .add_transition(Transition::new("Idle->Jump", idle_state, jump_state, 0.25, Self::IDLE_TO_JUMP))
            .add_transition(Transition::new("Jump->Idle", jump_state, idle_state, 0.30, Self::JUMP_TO_IDLE));

        Self {
            machine,
            jump_animation,
        }
    }

    fn apply(&mut self, scene: &mut Scene, dt: f32, input: LocomotionMachineInput) {
        self.machine
            // Update parameters which will be used by transitions.
            .set_parameter(Self::IDLE_TO_WALK, Parameter::Rule(input.is_walking))
            .set_parameter(Self::WALK_TO_IDLE, Parameter::Rule(!input.is_walking))
            .set_parameter(Self::WALK_TO_JUMP, Parameter::Rule(input.is_jumping))
            .set_parameter(Self::IDLE_TO_JUMP, Parameter::Rule(input.is_jumping))
            .set_parameter(Self::JUMP_TO_IDLE, Parameter::Rule(!input.is_jumping && scene.animations.get(self.jump_animation).has_ended()))
            // Finally we can do update tick for machine that will evaluate current pose for character.
            .evaluate_pose(&scene.animations, dt)
            // Pose must be applied to graph - remember that animations operate on multiple nodes at once.
            .apply(&mut scene.graph);
    }
}

struct Player {
    body: Handle<RigidBody>,
    pivot: Handle<Node>,
    camera_pivot: Handle<Node>,
    camera_hinge: Handle<Node>,
    model: Handle<Node>,
    controller: InputController,
    locomotion_machine: LocomotionMachine,
    model_yaw: SmoothAngle,
}

impl Player {
    fn new(scene: &mut Scene, resource_manager: &mut ResourceManager, context: Arc<Mutex<SceneLoadContext>>) -> Self {
        // It is important to lock context for short period of time so other thread can
        // read data from it as soon as possible - not when everything was loaded.
        context.lock().unwrap().report_progress(0.0, "Creating camera...");

        // Camera is our eyes in the world - you won't see anything without it.
        let camera = CameraBuilder::new(BaseBuilder::new()
            .with_local_transform(TransformBuilder::new()
                .with_local_position(Vec3::new(0.0, 0.0, -3.0))
                .build()))
            .build();
        let camera = scene.graph.add_node(Node::Camera(camera));

        let camera_pivot = scene.graph.add_node(Node::Base(BaseBuilder::new().build()));
        let camera_hinge = scene.graph.add_node(Node::Base(BaseBuilder::new()
            .with_local_transform(TransformBuilder::new()
                .with_local_position(Vec3::new(0.0, 1.0, 0.0))
                .build())
            .build()));
        scene.graph.link_nodes(camera_hinge, camera_pivot);
        scene.graph.link_nodes(camera, camera_hinge);

        context.lock().unwrap().report_progress(0.4, "Loading model...");

        // Load model resource. Is does *not* adds anything to our scene - it just loads a
        // resource then can be used later on to instantiate models from it on scene. Why
        // loading of resource is separated from instantiation? Because there it is too
        // inefficient to load a resource every time you trying to create instance of it -
        // much more efficient is to load it one and then make copies of it. In case of
        // models it is very efficient because single vertex and index buffer can be used
        // for all models instances, so memory footprint on GPU will be lower.
        let model_resource = resource_manager.request_model("examples/data/mutant.FBX").unwrap();

        context.lock().unwrap().report_progress(0.60, "Instantiating model...");

        // Instantiate model on scene - but only geometry, without any animations.
        // Instantiation is a process of embedding model resource data in desired scene.
        let model_handle = model_resource.lock()
            .unwrap()
            .instantiate_geometry(scene);

        let body_height = 1.2;

        // Now we have whole sub-graph instantiated, we can start modifying model instance.
        scene.graph
            .get_mut(model_handle)
            .base_mut()
            .local_transform_mut()
            .set_position(Vec3::new(0.0, -body_height, 0.0))
            // Our model is too big, fix it by scale.
            .set_scale(Vec3::new(0.0125, 0.0125, 0.0125));

        let pivot = scene.graph.add_node(Node::Base(BaseBuilder::new().build()));

        scene.graph.link_nodes(model_handle, pivot);

        let capsule = CapsuleShape::new(0.6, body_height, Axis::Y);
        let mut body = RigidBody::new(ConvexShape::Capsule(capsule));
        body.set_friction(Vec3::new(0.2, 0.0, 0.2));
        let body = scene.physics.add_body(body);

        scene.physics_binder.bind(pivot, body);

        context.lock().unwrap().report_progress(0.80, "Creating machine...");

        let locomotion_machine = LocomotionMachine::new(scene, model_handle, resource_manager);

        Self {
            body,
            pivot,
            model: model_handle,
            camera_pivot,
            controller: Default::default(),
            locomotion_machine,
            camera_hinge,
            model_yaw: SmoothAngle {
                angle: 0.0,
                target: 0.0,
                speed: 10.0,
            },
        }
    }

    fn update(&mut self, scene: &mut Scene, dt: f32) {
        let pivot = scene
            .graph
            .get(self.pivot)
            .base();

        let look_vector = pivot
            .look_vector()
            .normalized()
            .unwrap_or(Vec3::LOOK);

        let side_vector = pivot
            .side_vector()
            .normalized()
            .unwrap_or(Vec3::RIGHT);

        let position = pivot
            .local_transform()
            .position();

        let mut velocity = Vec3::ZERO;

        if self.controller.walk_right {
            velocity -= side_vector;
        }
        if self.controller.walk_left {
            velocity += side_vector;
        }
        if self.controller.walk_forward {
            velocity += look_vector;
        }
        if self.controller.walk_backward {
            velocity -= look_vector;
        }

        let speed = 2.0 * dt;
        let velocity = velocity.normalized()
            .and_then(|v| Some(v.scale(speed)))
            .unwrap_or(Vec3::ZERO);
        let is_moving = velocity.sqr_len() > 0.0;

        let body = scene.physics.borrow_body_mut(self.body);

        body.set_x_velocity(velocity.x)
            .set_z_velocity(velocity.z);

        let mut has_ground_contact = false;
        for contact in body.get_contacts() {
            if contact.position.y < position.y {
                has_ground_contact = true;
                break;
            }
        }

        while let Some(event) = scene.animations.get_mut(self.locomotion_machine.jump_animation).pop_event() {
            if event.signal_id == LocomotionMachine::JUMP_SIGNAL {
                body.set_y_velocity(6.0 * dt);
            }
        }

        let quat_yaw = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), self.controller.yaw);

        if is_moving {
            // Since we have free camera while not moving, we have to sync rotation of pivot
            // with rotation of camera so character will start moving in look direction.
            scene.graph
                .get_mut(self.pivot)
                .base_mut()
                .local_transform_mut()
                .set_rotation(quat_yaw);

            // Apply additional rotation to model - it will turn in front of walking direction.
            let angle: f32 = if self.controller.walk_left {
                if self.controller.walk_forward {
                    45.0
                } else if self.controller.walk_backward {
                    135.0
                } else {
                    90.0
                }
            } else if self.controller.walk_right {
                if self.controller.walk_forward {
                    -45.0
                } else if self.controller.walk_backward {
                    -135.0
                } else {
                    -90.0
                }
            } else {
                if self.controller.walk_backward {
                    180.0
                } else {
                    0.0
                }
            };

            self.model_yaw
                .set_target(angle.to_radians())
                .update(dt);

            scene.graph
                .get_mut(self.model)
                .base_mut()
                .local_transform_mut()
                .set_rotation(Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), self.model_yaw.angle));
        }

        let camera_pivot_transform = scene.graph
            .get_mut(self.camera_pivot)
            .base_mut()
            .local_transform_mut();

        camera_pivot_transform.set_rotation(quat_yaw)
            .set_position(position + velocity);

        // Rotate camera hinge - this will make camera move up and down while look at character
        // (well not exactly on character - on characters head)
        scene.graph
            .get_mut(self.camera_hinge)
            .base_mut()
            .local_transform_mut()
            .set_rotation(Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), self.controller.pitch));

        if has_ground_contact && self.controller.jump {
            // Rewind jump animation to beginning before jump.
            scene.animations
                .get_mut(self.locomotion_machine.jump_animation)
                .rewind();
        }

        // Make sure to apply animation machine pose to model explicitly.
        self.locomotion_machine.apply(scene, dt, LocomotionMachineInput {
            is_walking: self.controller.walk_backward || self.controller.walk_forward || self.controller.walk_right || self.controller.walk_left,
            is_jumping: has_ground_contact && self.controller.jump,
        });
    }

    fn handle_input(&mut self, device_event: &DeviceEvent, dt: f32) {
        match device_event {
            DeviceEvent::Key(key) => {
                if let Some(key_code) = key.virtual_keycode {
                    match key_code {
                        VirtualKeyCode::W => self.controller.walk_forward = key.state == ElementState::Pressed,
                        VirtualKeyCode::S => self.controller.walk_backward = key.state == ElementState::Pressed,
                        VirtualKeyCode::A => self.controller.walk_left = key.state == ElementState::Pressed,
                        VirtualKeyCode::D => self.controller.walk_right = key.state == ElementState::Pressed,
                        VirtualKeyCode::Space => {
                            self.controller.jump = key.state == ElementState::Pressed
                        }
                        _ => ()
                    }
                }
            }
            DeviceEvent::MouseMotion { delta } => {
                let mouse_sens = 0.2 * dt;
                self.controller.yaw -= (delta.0 as f32) * mouse_sens;
                self.controller.pitch = (self.controller.pitch + (delta.1 as f32) * mouse_sens)
                    .max(-90.0f32.to_radians())
                    .min(90.0f32.to_radians());
            }
            _ => {}
        }
    }
}

fn create_scene_async(resource_manager: Arc<Mutex<ResourceManager>>) -> Arc<Mutex<SceneLoadContext>> {
    // Create load context - it will be shared with caller and loader threads.
    let context = Arc::new(Mutex::new(SceneLoadContext {
        data: None,
        message: "Starting..".to_string(),
        progress: 0.0,
    }));
    let result = context.clone();

    // Spawn separate thread which will create scene by loading various assets.
    std::thread::spawn(move || {
        let mut scene = Scene::new();

        let mut resource_manager = resource_manager.lock().unwrap();

        context.lock().unwrap().report_progress(0.25, "Loading map...");

        // Load simple map.
        resource_manager
            .request_model("examples/data/map.FBX")
            .unwrap()
            .lock()
            .unwrap()
            .instantiate_geometry(&mut scene);

        // And create collision mesh so our character won't fall thru ground.
        let collision_mesh_handle = scene.graph.find_by_name_from_root("Map");
        let collision_mesh = scene.graph.get(collision_mesh_handle).as_mesh();
        let static_geometry = mesh_to_static_geometry(collision_mesh);
        scene.physics.add_static_geometry(static_geometry);

        // Finally create player.
        let player = Player::new(&mut scene, &mut resource_manager, context.clone());

        context.lock().unwrap().report_progress(1.0, "Done");

        context.lock().unwrap().data = Some(SceneLoadResult {
            scene,
            player,
        })
    });

    // Immediately return shared context.
    result
}

struct InputController {
    walk_forward: bool,
    walk_backward: bool,
    walk_left: bool,
    walk_right: bool,
    jump: bool,
    yaw: f32,
    pitch: f32,
}

impl Default for InputController {
    fn default() -> Self {
        Self {
            walk_forward: false,
            walk_backward: false,
            walk_left: false,
            walk_right: false,
            jump: false,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_title("Example - 3rd Person")
        .with_resizable(true);

    let mut engine = GameEngine::new(window_builder, &event_loop).unwrap();

    // Prepare resource manager - it must be notified where to search textures. When engine
    // loads model resource it automatically tries to load textures it uses. But since most
    // model formats store absolute paths, we can't use them as direct path to load texture
    // instead we telling engine to search textures in given folder.
    engine.resource_manager.lock().unwrap().set_textures_path("examples/data");

    // Create simple user interface that will show some useful info.
    let window = engine.get_window();
    let screen_size = window.inner_size().to_logical(window.scale_factor());
    let interface = create_ui(&mut engine.user_interface, Vec2::new(screen_size.width, screen_size.height));

    // Create scene asynchronously - this method immediately returns empty load context
    // which will be filled with data over time.
    let load_context = create_scene_async(engine.resource_manager.clone());

    // Initially scene is None, once scene is loaded it'll have actual state.
    let mut game_scene: Option<GameScene> = None;

    // Set ambient light.
    engine.renderer.set_ambient_color(Color::opaque(80, 80, 80));
    let mut quality = engine.renderer.get_quality_settings();
    quality.spot_shadows_distance = 300.0;
    quality.point_shadows_distance = 300.0;
    engine.renderer.set_quality_settings(&quality).unwrap();

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

                    // Check each frame if our scene is created - here we just trying to lock context
                    // without blocking, it is important for main thread to be functional while other
                    // thread still loading data.
                    if let Ok(mut load_context) = load_context.try_lock() {
                        if let Some(load_result) = load_context.data.take() {
                            // Add scene to engine - engine will take ownership over scene and will return
                            // you a handle to scene which can be used later on to borrow it and do some
                            // actions you need.
                            game_scene = Some(GameScene {
                                scene: engine.scenes.add(load_result.scene),
                                player: load_result.player,
                            });

                            // Once scene is loaded, we should hide progress bar and text.
                            if let UiNode::ProgressBar(progress_bar) = engine.user_interface.node_mut(interface.progress_bar) {
                                progress_bar.widget_mut().set_visibility(false);
                            }

                            if let UiNode::Text(progress_text) = engine.user_interface.node_mut(interface.progress_text) {
                                progress_text.widget_mut().set_visibility(false);
                            }
                        }

                        // Report progress in UI.
                        if let UiNode::ProgressBar(progress_bar) = engine.user_interface.node_mut(interface.progress_bar) {
                            progress_bar.set_progress(load_context.progress);
                        }

                        if let UiNode::Text(progress_text) = engine.user_interface.node_mut(interface.progress_text) {
                            progress_text.set_text(format!("Loading scene: {}%\n{}", load_context.progress * 100.0, load_context.message));
                        }
                    }

                    // Update scene only if it is loaded.
                    if let Some(game_scene) = game_scene.as_mut() {
                        // Use stored scene handle to borrow a mutable reference of scene in
                        // engine.
                        let scene = engine.scenes.get_mut(game_scene.scene);

                        game_scene.player.update(scene, fixed_timestep);
                    }

                    // While scene is loading, we will update progress bar.
                    if let UiNode::Text(text) = engine.user_interface.node_mut(interface.debug_text) {
                        let fps = engine.renderer.get_statistics().frames_per_second;
                        text.set_text(format!("Example - 3rd Person\n[W][S][A][D] - walk, [SPACE] - jump.\nFPS: {}", fps));
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
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit
                    }
                    WindowEvent::Resized(size) => {
                        // It is very important to handle Resized event from window, because
                        // renderer knows nothing about window size - it must be notified
                        // directly when window size has changed.
                        engine.renderer.set_frame_size(size.into());

                        // Root UI node should be resized too, otherwise progress bar will stay
                        // in wrong position after resize.
                        let size = size.to_logical(engine.get_window().scale_factor());
                        if let UiNode::Grid(root) = engine.user_interface.node_mut(interface.root) {
                            root.widget_mut()
                                .set_width_mut(size.width)
                                .set_height_mut(size.height);
                        }
                    }
                    _ => ()
                }

                // It is very important to "feed" user interface (UI) with events coming
                // from main window, otherwise UI won't respond to mouse, keyboard, or any
                // other event.
                if let Some(os_event) = translate_event(&event) {
                    engine.user_interface.process_os_event(&os_event);
                }
            }
            Event::DeviceEvent { event, .. } => {
                if let Some(game_scene) = game_scene.as_mut() {
                    game_scene.player.handle_input(&event, fixed_timestep);
                }
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}