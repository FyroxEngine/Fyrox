//! This module contains common code that is used across multiple examples.

// Suppress warning about unused code, this mod shared across multiple examples and
// some parts can be unused in some examples.
#![allow(dead_code)]

use rapier3d::dynamics::RigidBodyBuilder;
use rapier3d::geometry::ColliderBuilder;
use rapier3d::na::{Isometry3, UnitQuaternion, Vector3};
use rg3d::core::algebra::Vector2;
use rg3d::scene::graph::Graph;
use rg3d::scene::RigidBodyHandle;
use rg3d::sound::effects::{BaseEffect, Effect};
use rg3d::{
    animation::{
        machine::{Machine, Parameter, PoseNode, State, Transition},
        Animation, AnimationSignal,
    },
    core::{color::Color, math::SmoothAngle, pool::Handle},
    engine::resource_manager::ResourceManager,
    event::{DeviceEvent, ElementState, VirtualKeyCode},
    event_loop::EventLoop,
    gui::{
        grid::{Column, GridBuilder, Row},
        node::StubNode,
        progress_bar::ProgressBarBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        HorizontalAlignment, Thickness, VerticalAlignment,
    },
    renderer::QualitySettings,
    resource::texture::TextureWrapMode,
    scene::{
        base::BaseBuilder,
        camera::{CameraBuilder, SkyBox},
        node::Node,
        transform::TransformBuilder,
        Scene,
    },
};
use std::time::Duration;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

// Create our own engine type aliases. These specializations are needed
// because engine provides a way to extend UI with custom nodes and messages.
pub type GameEngine = rg3d::engine::Engine<(), StubNode>;
pub type UiNode = rg3d::gui::node::UINode<(), StubNode>;
pub type BuildContext<'a> = rg3d::gui::BuildContext<'a, (), StubNode>;

/// Creates a camera at given position with a skybox.
pub async fn create_camera(
    resource_manager: ResourceManager,
    position: Vector3<f32>,
    graph: &mut Graph,
) -> Handle<Node> {
    // Load skybox textures in parallel.
    let (front, back, left, right, top, bottom) = rg3d::futures::join!(
        resource_manager
            .request_texture("examples/data/skyboxes/DarkStormy/DarkStormyFront2048.png"),
        resource_manager
            .request_texture("examples/data/skyboxes/DarkStormy/DarkStormyBack2048.png"),
        resource_manager
            .request_texture("examples/data/skyboxes/DarkStormy/DarkStormyLeft2048.png"),
        resource_manager
            .request_texture("examples/data/skyboxes/DarkStormy/DarkStormyRight2048.png"),
        resource_manager.request_texture("examples/data/skyboxes/DarkStormy/DarkStormyUp2048.png"),
        resource_manager
            .request_texture("examples/data/skyboxes/DarkStormy/DarkStormyDown2048.png")
    );

    // Unwrap everything.
    let skybox = SkyBox {
        front: Some(front.unwrap()),
        back: Some(back.unwrap()),
        left: Some(left.unwrap()),
        right: Some(right.unwrap()),
        top: Some(top.unwrap()),
        bottom: Some(bottom.unwrap()),
    };

    // Set S and T coordinate wrap mode, ClampToEdge will remove any possible seams on edges
    // of the skybox.
    for skybox_texture in skybox.textures().iter().filter_map(|t| t.clone()) {
        let mut data = skybox_texture.data_ref();
        data.set_s_wrap_mode(TextureWrapMode::ClampToEdge);
        data.set_t_wrap_mode(TextureWrapMode::ClampToEdge);
    }

    // Camera is our eyes in the world - you won't see anything without it.
    CameraBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(position)
                .build(),
        ),
    )
    .with_skybox(skybox)
    .build(graph)
}

pub struct Game {
    pub game_scene: Option<GameScene>,
    pub load_context: Option<Arc<Mutex<SceneLoadContext>>>,
    pub engine: GameEngine,
}

impl Game {
    pub fn new(title: &str) -> (Self, EventLoop<()>) {
        let event_loop = EventLoop::new();

        let window_builder = rg3d::window::WindowBuilder::new()
            .with_title(title)
            .with_resizable(true);

        let mut engine = GameEngine::new(window_builder, &event_loop, false).unwrap();

        // Prepare resource manager - it must be notified where to search textures. When engine
        // loads model resource it automatically tries to load textures it uses. But since most
        // model formats store absolute paths, we can't use them as direct path to load texture
        // instead we telling engine to search textures in given folder.
        engine
            .resource_manager
            .state()
            .set_textures_path("examples/data");

        // Set ambient light.
        engine.renderer.set_ambient_color(Color::opaque(80, 80, 80));

        engine
            .renderer
            .set_quality_settings(&fix_shadows_distance(QualitySettings::high()))
            .unwrap();

        let game = Self {
            // Initially scene is None, once scene is loaded it'll have actual state.
            game_scene: None,
            // Create scene asynchronously - this method immediately returns empty load context
            // which will be filled with data over time.
            load_context: Some(create_scene_async(engine.resource_manager.clone())),
            engine,
        };
        (game, event_loop)
    }
}

pub struct Interface {
    pub root: Handle<UiNode>,
    pub debug_text: Handle<UiNode>,
    pub progress_bar: Handle<UiNode>,
    pub progress_text: Handle<UiNode>,
}

pub fn create_ui(ui: &mut BuildContext, screen_size: Vector2<f32>) -> Interface {
    let debug_text;
    let progress_bar;
    let progress_text;
    let root = GridBuilder::new(
        WidgetBuilder::new()
            .with_width(screen_size.x)
            .with_height(screen_size.y)
            .with_child({
                debug_text = TextBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
                    .with_wrap(true)
                    .build(ui);
                debug_text
            })
            .with_child({
                progress_bar =
                    ProgressBarBuilder::new(WidgetBuilder::new().on_row(1).on_column(1)).build(ui);
                progress_bar
            })
            .with_child({
                progress_text = TextBuilder::new(
                    WidgetBuilder::new()
                        .on_column(1)
                        .on_row(0)
                        .with_margin(Thickness::bottom(20.0))
                        .with_vertical_alignment(VerticalAlignment::Bottom),
                )
                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                .build(ui);
                progress_text
            }),
    )
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

pub struct SceneLoadResult {
    pub scene: Scene,
    pub player: Player,
    pub reverb_effect: Handle<Effect>,
}

#[derive(Default)]
pub struct GameScene {
    pub scene: Handle<Scene>,
    pub player: Player,
    pub reverb_effect: Handle<Effect>,
}

pub struct SceneLoadContext {
    pub scene_data: Option<SceneLoadResult>,
    pub message: String,
    pub progress: f32,
}

impl SceneLoadContext {
    pub fn report_progress(&mut self, progress: f32, message: &str) {
        self.progress = progress;
        self.message = message.to_owned();
        println!("Loading progress: {}% - {}", progress * 100.0, message);
    }
}

// Small helper function that loads animation from given file and retargets it to given model.
pub async fn load_animation<P: AsRef<Path>>(
    path: P,
    scene: &mut Scene,
    model: Handle<Node>,
    resource_manager: ResourceManager,
) -> Handle<Animation> {
    *resource_manager
        .request_model(path)
        .await
        .unwrap()
        .retarget_animations(model, scene)
        .get(0)
        .unwrap()
}

// Small helper function that creates PlayAnimation machine node and creates
// state from it.
pub async fn create_play_animation_state<P: AsRef<Path>>(
    path: P,
    name: &str,
    machine: &mut Machine,
    scene: &mut Scene,
    model: Handle<Node>,
    resource_manager: ResourceManager,
) -> (Handle<Animation>, Handle<State>) {
    // First of all load required animation and apply it on model.
    let animation = load_animation(path, scene, model, resource_manager).await;

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

#[derive(Default)]
pub struct LocomotionMachine {
    pub machine: Machine,
    pub jump_animation: Handle<Animation>,
    pub walk_animation: Handle<Animation>,
    pub walk_state: Handle<State>,
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

    pub const JUMP_SIGNAL: u64 = 1;

    pub async fn new(
        scene: &mut Scene,
        model: Handle<Node>,
        resource_manager: ResourceManager,
    ) -> Self {
        let mut machine = Machine::new();

        let (walk_animation, walk_state) = create_play_animation_state(
            "examples/data/walk.fbx",
            "Walk",
            &mut machine,
            scene,
            model,
            resource_manager.clone(),
        )
        .await;
        let (_, idle_state) = create_play_animation_state(
            "examples/data/idle.fbx",
            "Idle",
            &mut machine,
            scene,
            model,
            resource_manager.clone(),
        )
        .await;

        // Jump animation is a bit special - it must be non-looping.
        let (jump_animation, jump_state) = create_play_animation_state(
            "examples/data/jump.fbx",
            "Jump",
            &mut machine,
            scene,
            model,
            resource_manager,
        )
        .await;
        scene
            .animations
            .get_mut(jump_animation)
            // Actual jump (applying force to physical body) must be synced with animation
            // so we have to be notified about this. This is where signals come into play
            // you can assign any signal in animation timeline and then in update loop you
            // can iterate over them and react appropriately.
            .add_signal(AnimationSignal::new(Self::JUMP_SIGNAL, 0.32))
            .set_loop(false);

        // Add transitions between states. This is the "heart" of animation blending state machine
        // it defines how it will respond to input parameters.
        machine.add_transition(Transition::new(
            "Walk->Idle",
            walk_state,
            idle_state,
            0.30,
            Self::WALK_TO_IDLE,
        ));
        machine.add_transition(Transition::new(
            "Walk->Jump",
            walk_state,
            jump_state,
            0.20,
            Self::WALK_TO_JUMP,
        ));
        machine.add_transition(Transition::new(
            "Idle->Walk",
            idle_state,
            walk_state,
            0.30,
            Self::IDLE_TO_WALK,
        ));
        machine.add_transition(Transition::new(
            "Idle->Jump",
            idle_state,
            jump_state,
            0.25,
            Self::IDLE_TO_JUMP,
        ));
        machine.add_transition(Transition::new(
            "Jump->Idle",
            jump_state,
            idle_state,
            0.30,
            Self::JUMP_TO_IDLE,
        ));

        Self {
            machine,
            jump_animation,
            walk_animation,
            walk_state,
        }
    }

    pub fn apply(&mut self, scene: &mut Scene, dt: f32, input: LocomotionMachineInput) {
        self.machine
            // Update parameters which will be used by transitions.
            .set_parameter(Self::IDLE_TO_WALK, Parameter::Rule(input.is_walking))
            .set_parameter(Self::WALK_TO_IDLE, Parameter::Rule(!input.is_walking))
            .set_parameter(Self::WALK_TO_JUMP, Parameter::Rule(input.is_jumping))
            .set_parameter(Self::IDLE_TO_JUMP, Parameter::Rule(input.is_jumping))
            .set_parameter(
                Self::JUMP_TO_IDLE,
                Parameter::Rule(
                    !input.is_jumping && scene.animations.get(self.jump_animation).has_ended(),
                ),
            )
            // Finally we can do update tick for machine that will evaluate current pose for character.
            .evaluate_pose(&scene.animations, dt)
            // Pose must be applied to graph - remember that animations operate on multiple nodes at once.
            .apply(&mut scene.graph);
    }
}

#[derive(Default)]
pub struct Player {
    pub body: RigidBodyHandle,
    pub pivot: Handle<Node>,
    pub camera_pivot: Handle<Node>,
    pub camera_hinge: Handle<Node>,
    pub camera: Handle<Node>,
    pub model: Handle<Node>,
    pub controller: InputController,
    pub locomotion_machine: LocomotionMachine,
    pub model_yaw: SmoothAngle,
}

impl Player {
    pub async fn new(
        scene: &mut Scene,
        resource_manager: ResourceManager,
        context: Arc<Mutex<SceneLoadContext>>,
    ) -> Self {
        // It is important to lock context for short period of time so other thread can
        // read data from it as soon as possible - not when everything was loaded.
        context
            .lock()
            .unwrap()
            .report_progress(0.0, "Creating camera...");

        let camera;
        let camera_hinge;
        let camera_pivot = BaseBuilder::new()
            .with_children(&[{
                camera_hinge = BaseBuilder::new()
                    .with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(Vector3::new(0.0, 1.0, 0.0))
                            .build(),
                    )
                    .with_children(&[{
                        camera = create_camera(
                            resource_manager.clone(),
                            Vector3::new(0.0, 0.0, -3.0),
                            &mut scene.graph,
                        )
                        .await;
                        camera
                    }])
                    .build(&mut scene.graph);
                camera_hinge
            }])
            .build(&mut scene.graph);

        context
            .lock()
            .unwrap()
            .report_progress(0.4, "Loading model...");

        // Load model resource. Is does *not* adds anything to our scene - it just loads a
        // resource then can be used later on to instantiate models from it on scene. Why
        // loading of resource is separated from instantiation? Because there it is too
        // inefficient to load a resource every time you trying to create instance of it -
        // much more efficient is to load it one and then make copies of it. In case of
        // models it is very efficient because single vertex and index buffer can be used
        // for all models instances, so memory footprint on GPU will be lower.
        let model_resource = resource_manager
            .request_model("examples/data/mutant.FBX")
            .await
            .unwrap();

        context
            .lock()
            .unwrap()
            .report_progress(0.60, "Instantiating model...");

        // Instantiate model on scene - but only geometry, without any animations.
        // Instantiation is a process of embedding model resource data in desired scene.
        let model_handle = model_resource.instantiate_geometry(scene);

        let body_height = 1.2;

        // Now we have whole sub-graph instantiated, we can start modifying model instance.
        scene.graph[model_handle]
            .local_transform_mut()
            .set_position(Vector3::new(0.0, -body_height, 0.0))
            // Our model is too big, fix it by scale.
            .set_scale(Vector3::new(0.0125, 0.0125, 0.0125));

        let pivot = BaseBuilder::new()
            .with_children(&[model_handle])
            .build(&mut scene.graph);

        let capsule = ColliderBuilder::capsule_y(body_height, 0.6).build();
        let body = scene.physics.add_body(
            RigidBodyBuilder::new_dynamic()
                .position(Isometry3::new(
                    Vector3::new(0.0, 2.0, 0.0),
                    Default::default(),
                ))
                .build(),
        );
        scene.physics.add_collider(capsule, body);

        scene.physics_binder.bind(pivot, body.into());

        context
            .lock()
            .unwrap()
            .report_progress(0.80, "Creating machine...");

        let locomotion_machine =
            LocomotionMachine::new(scene, model_handle, resource_manager).await;

        Self {
            body: body.into(),
            pivot,
            model: model_handle,
            camera_pivot,
            controller: Default::default(),
            locomotion_machine,
            camera_hinge,
            camera,
            model_yaw: SmoothAngle {
                angle: 0.0,
                target: 0.0,
                speed: 10.0,
            },
        }
    }

    pub fn update(&mut self, scene: &mut Scene, dt: f32) {
        let pivot = &scene.graph[self.pivot];

        let look_vector = pivot
            .look_vector()
            .try_normalize(std::f32::EPSILON)
            .unwrap_or(Vector3::z());

        let side_vector = pivot
            .side_vector()
            .try_normalize(std::f32::EPSILON)
            .unwrap_or(Vector3::x());

        let position = **pivot.local_transform().position();

        let mut velocity = Vector3::default();

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
        let velocity = velocity
            .try_normalize(std::f32::EPSILON)
            .and_then(|v| Some(v.scale(speed)))
            .unwrap_or(Vector3::default());
        let is_moving = velocity.norm_squared() > 0.0;

        let body = scene.physics.bodies.get_mut(self.body.into()).unwrap();

        let mut has_ground_contact = false;
        if let Some(iterator) = scene
            .physics
            .narrow_phase
            .contacts_with(body.colliders()[0])
        {
            'outer_loop: for (_, _, contact) in iterator {
                for manifold in contact.manifolds.iter() {
                    if manifold.local_n1.y > 0.7 {
                        has_ground_contact = true;
                        break 'outer_loop;
                    }
                }
            }
        }

        let mut new_y_vel = None;
        while let Some(event) = scene
            .animations
            .get_mut(self.locomotion_machine.jump_animation)
            .pop_event()
        {
            if event.signal_id == LocomotionMachine::JUMP_SIGNAL {
                new_y_vel = Some(6.0 * dt);
            }
        }

        let quat_yaw = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.controller.yaw);

        body.wake_up(true);
        body.set_angvel(Default::default(), true);
        if let Some(new_y_vel) = new_y_vel {
            body.set_linvel(
                Vector3::new(velocity.x / dt, new_y_vel / dt, velocity.z / dt),
                true,
            );
        } else {
            body.set_linvel(
                Vector3::new(velocity.x / dt, body.linvel().y, velocity.z / dt),
                true,
            );
        }

        if is_moving {
            // Since we have free camera while not moving, we have to sync rotation of pivot
            // with rotation of camera so character will start moving in look direction.
            let mut current_position = *body.position();
            current_position.rotation = quat_yaw;
            body.set_position(current_position, true);

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

            self.model_yaw.set_target(angle.to_radians()).update(dt);

            scene.graph[self.model].local_transform_mut().set_rotation(
                UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.model_yaw.angle),
            );
        }

        let camera_pivot_transform = scene.graph[self.camera_pivot].local_transform_mut();

        camera_pivot_transform
            .set_rotation(quat_yaw)
            .set_position(position + velocity);

        // Rotate camera hinge - this will make camera move up and down while look at character
        // (well not exactly on character - on characters head)
        scene.graph[self.camera_hinge]
            .local_transform_mut()
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::x_axis(),
                self.controller.pitch,
            ));

        if has_ground_contact && self.controller.jump {
            // Rewind jump animation to beginning before jump.
            scene
                .animations
                .get_mut(self.locomotion_machine.jump_animation)
                .rewind();
        }

        // Make sure to apply animation machine pose to model explicitly.
        self.locomotion_machine.apply(
            scene,
            dt,
            LocomotionMachineInput {
                is_walking: self.controller.walk_backward
                    || self.controller.walk_forward
                    || self.controller.walk_right
                    || self.controller.walk_left,
                is_jumping: has_ground_contact && self.controller.jump,
            },
        );
    }

    pub fn handle_device_event(&mut self, device_event: &DeviceEvent, dt: f32) {
        match device_event {
            DeviceEvent::Key(_key) => {
                // Handle key input events via `WindowEvent`, not via `DeviceEvent` (#32)
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

    pub fn handle_key_event(&mut self, key: &rg3d::event::KeyboardInput, _dt: f32) {
        if let Some(key_code) = key.virtual_keycode {
            match key_code {
                VirtualKeyCode::W => {
                    self.controller.walk_forward = key.state == ElementState::Pressed
                }
                VirtualKeyCode::S => {
                    self.controller.walk_backward = key.state == ElementState::Pressed
                }
                VirtualKeyCode::A => self.controller.walk_left = key.state == ElementState::Pressed,
                VirtualKeyCode::D => {
                    self.controller.walk_right = key.state == ElementState::Pressed
                }
                VirtualKeyCode::Space => self.controller.jump = key.state == ElementState::Pressed,
                _ => (),
            }
        }
    }
}

pub fn create_scene_async(resource_manager: ResourceManager) -> Arc<Mutex<SceneLoadContext>> {
    // Create load context - it will be shared with caller and loader threads.
    let context = Arc::new(Mutex::new(SceneLoadContext {
        scene_data: None,
        message: "Starting..".to_string(),
        progress: 0.0,
    }));
    let result = context.clone();

    // Spawn separate thread which will create scene by loading various assets.
    std::thread::spawn(move || {
        futures::executor::block_on(async move {
            let mut scene = Scene::new();

            // Create reverb effect for more natural sound - our player walks in some sort of cathedral,
            // so there will be pretty decent echo.
            let mut base_effect = BaseEffect::default();
            // Make sure it won't be too loud - rg3d-sound doesn't care about energy conservation law, it
            // just makes requested calculation.
            base_effect.set_gain(0.7);
            let mut reverb = rg3d::sound::effects::reverb::Reverb::new(base_effect);
            // Set reverb time to ~3 seconds - the more time the deeper the echo.
            reverb.set_decay_time(Duration::from_secs_f32(3.0));
            let reverb_effect = scene
                .sound_context
                .state()
                .add_effect(rg3d::sound::effects::Effect::Reverb(reverb));

            context
                .lock()
                .unwrap()
                .report_progress(0.25, "Loading map...");

            // Load simple map.
            resource_manager
                .request_model("examples/data/Sponza.fbx")
                .await
                .unwrap()
                .instantiate_geometry(&mut scene);

            scene.graph.update_hierarchical_data();

            // And create collision mesh so our character won't fall thru ground.
            let collision_mesh_handle = scene.graph.find_by_name_from_root("CollisionShape");
            let collision_mesh = &mut scene.graph[collision_mesh_handle];

            collision_mesh.set_visibility(false);
            // Create collision geometry from special mesh on the level.
            let body = scene
                .physics
                .mesh_to_trimesh(collision_mesh_handle, &scene.graph);
            scene.physics_binder.bind(collision_mesh_handle, body);

            // Finally create player.
            let player = Player::new(&mut scene, resource_manager, context.clone()).await;

            context.lock().unwrap().report_progress(1.0, "Done");

            context.lock().unwrap().scene_data = Some(SceneLoadResult {
                scene,
                player,
                reverb_effect,
            });
        })
    });

    // Immediately return shared context.
    result
}

pub struct InputController {
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

pub fn fix_shadows_distance(mut quality: QualitySettings) -> QualitySettings {
    // Scale distance because game world has different scale.
    quality.spot_shadows_distance *= 2.0;
    quality.point_shadows_distance *= 2.0;
    quality
}
