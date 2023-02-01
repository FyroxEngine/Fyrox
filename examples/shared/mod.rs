//! This module contains common code that is used across multiple examples.

// Suppress warning about unused code, this mod shared across multiple examples and
// some parts can be unused in some examples.
#![allow(dead_code)]

use fyrox::{
    animation::{
        machine::{Machine, MachineLayer, Parameter, PoseNode, State, Transition},
        Animation, AnimationSignal,
    },
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        color::Color,
        math::SmoothAngle,
        pool::Handle,
    },
    engine::{resource_manager::ResourceManager, Engine, EngineInitParams, SerializationContext},
    event::{DeviceEvent, ElementState, VirtualKeyCode},
    event_loop::EventLoop,
    gui::{
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        progress_bar::ProgressBarBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        HorizontalAlignment, Thickness, VerticalAlignment,
    },
    gui::{BuildContext, UiNode},
    renderer::QualitySettings,
    resource::texture::TextureWrapMode,
    scene::{
        animation::{
            absm::{AnimationBlendingStateMachine, AnimationBlendingStateMachineBuilder},
            AnimationPlayer,
        },
        base::BaseBuilder,
        camera::{CameraBuilder, SkyBoxBuilder},
        collider::{Collider, ColliderBuilder, ColliderShape},
        graph::Graph,
        node::Node,
        pivot::PivotBuilder,
        rigidbody::{RigidBody, RigidBodyBuilder, RigidBodyType},
        sound::{
            effect::{BaseEffectBuilder, Effect, ReverbEffectBuilder},
            listener::ListenerBuilder,
        },
        transform::TransformBuilder,
        Scene,
    },
};
use fyrox_core::uuid::{uuid, Uuid};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

/// Creates a camera at given position with a skybox.
pub async fn create_camera(
    resource_manager: ResourceManager,
    position: Vector3<f32>,
    graph: &mut Graph,
) -> Handle<Node> {
    // Load skybox textures in parallel.
    let (front, back, left, right, top, bottom) = fyrox::core::futures::join!(
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
    let skybox = SkyBoxBuilder {
        front: Some(front.unwrap()),
        back: Some(back.unwrap()),
        left: Some(left.unwrap()),
        right: Some(right.unwrap()),
        top: Some(top.unwrap()),
        bottom: Some(bottom.unwrap()),
    }
    .build()
    .unwrap();

    // Set S and T coordinate wrap mode, ClampToEdge will remove any possible seams on edges
    // of the skybox.
    if let Some(cubemap) = skybox.cubemap() {
        let mut data = cubemap.data_ref();
        data.set_s_wrap_mode(TextureWrapMode::ClampToEdge);
        data.set_t_wrap_mode(TextureWrapMode::ClampToEdge);
    }

    // Camera is our eyes in the world - you won't see anything without it.
    CameraBuilder::new(
        BaseBuilder::new()
            .with_local_transform(
                TransformBuilder::new()
                    .with_local_position(position)
                    .build(),
            )
            .with_children(&[
                // Create sound listener, otherwise we'd heat sound as if we'd be in (0,0,0)
                ListenerBuilder::new(BaseBuilder::new()).build(graph),
            ]),
    )
    .with_skybox(skybox)
    .build(graph)
}

pub struct Game {
    pub game_scene: Option<GameScene>,
    pub load_context: Option<Arc<Mutex<SceneLoadContext>>>,
    pub engine: Engine,
}

impl Game {
    pub fn new(title: &str) -> (Self, EventLoop<()>) {
        let event_loop = EventLoop::new();

        let window_builder = fyrox::window::WindowBuilder::new()
            .with_title(title)
            .with_resizable(true);

        let serialization_context = Arc::new(SerializationContext::new());
        let mut engine = Engine::new(EngineInitParams {
            window_builder,
            resource_manager: ResourceManager::new(serialization_context.clone()),
            serialization_context,
            events_loop: &event_loop,
            vsync: false,
            headless: false,
        })
        .unwrap();

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
                    .with_wrap(WrapMode::Word)
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
        .retarget_animations(model, &mut scene.graph)
        .get(0)
        .unwrap()
}

// Small helper function that creates PlayAnimation machine node and creates
// state from it.
pub async fn create_play_animation_state<P: AsRef<Path>>(
    path: P,
    name: &str,
    machine_layer: &mut MachineLayer,
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
    let node = machine_layer.add_node(PoseNode::make_play_animation(animation));

    // Finally use new node and create state from it.
    let state = machine_layer.add_state(State::new(name, node));

    (animation, state)
}

#[derive(Default)]
pub struct LocomotionMachine {
    pub machine: Handle<Node>,
    pub jump_animation: Handle<Animation>,
    pub walk_animation: Handle<Animation>,
    pub walk_state: Handle<State>,
    pub animation_player: Handle<Node>,
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

    pub const JUMP_SIGNAL: Uuid = uuid!("3e536261-9edf-4436-bba0-11173e61c8e9");

    pub async fn new(
        scene: &mut Scene,
        model: Handle<Node>,
        resource_manager: ResourceManager,
        animation_player: Handle<Node>,
    ) -> Self {
        let mut machine = Machine::new();

        let root_layer = &mut machine.layers_mut()[0];

        let (walk_animation, walk_state) = create_play_animation_state(
            "examples/data/mutant/walk.fbx",
            "Walk",
            root_layer,
            scene,
            model,
            resource_manager.clone(),
        )
        .await;
        let (_, idle_state) = create_play_animation_state(
            "examples/data/mutant/idle.fbx",
            "Idle",
            root_layer,
            scene,
            model,
            resource_manager.clone(),
        )
        .await;

        // Jump animation is a bit special - it must be non-looping.
        let (jump_animation, jump_state) = create_play_animation_state(
            "examples/data/mutant/jump.fbx",
            "Jump",
            root_layer,
            scene,
            model,
            resource_manager,
        )
        .await;

        (**scene.graph[animation_player]
            .query_component_mut::<AnimationPlayer>()
            .unwrap()
            .animations_mut())
        .get_mut(jump_animation)
        // Actual jump (applying force to physical body) must be synced with animation
        // so we have to be notified about this. This is where signals come into play
        // you can assign any signal in animation timeline and then in update loop you
        // can iterate over them and react appropriately.
        .add_signal(AnimationSignal {
            id: Self::JUMP_SIGNAL,
            name: "Jump".to_string(),
            time: 0.32,
            enabled: true,
        })
        .set_loop(false);

        // Add transitions between states. This is the "heart" of animation blending state machine
        // it defines how it will respond to input parameters.
        root_layer.add_transition(Transition::new(
            "Walk->Idle",
            walk_state,
            idle_state,
            0.30,
            Self::WALK_TO_IDLE,
        ));
        root_layer.add_transition(Transition::new(
            "Walk->Jump",
            walk_state,
            jump_state,
            0.20,
            Self::WALK_TO_JUMP,
        ));
        root_layer.add_transition(Transition::new(
            "Idle->Walk",
            idle_state,
            walk_state,
            0.30,
            Self::IDLE_TO_WALK,
        ));
        root_layer.add_transition(Transition::new(
            "Idle->Jump",
            idle_state,
            jump_state,
            0.25,
            Self::IDLE_TO_JUMP,
        ));
        root_layer.add_transition(Transition::new(
            "Jump->Idle",
            jump_state,
            idle_state,
            0.30,
            Self::JUMP_TO_IDLE,
        ));

        let machine = AnimationBlendingStateMachineBuilder::new(BaseBuilder::new())
            .with_machine(machine)
            .with_animation_player(animation_player)
            .build(&mut scene.graph);

        Self {
            machine,
            jump_animation,
            walk_animation,
            walk_state,
            animation_player,
        }
    }

    pub fn apply(&mut self, scene: &mut Scene, input: LocomotionMachineInput) {
        let animation_player = scene.graph[self.animation_player]
            .query_component_ref::<AnimationPlayer>()
            .unwrap();

        let jump_ended = (**animation_player.animations())
            .get(self.jump_animation)
            .has_ended();

        scene.graph[self.machine]
            .query_component_mut::<AnimationBlendingStateMachine>()
            .unwrap()
            .machine_mut()
            // Update parameters which will be used by transitions.
            .set_parameter(Self::IDLE_TO_WALK, Parameter::Rule(input.is_walking))
            .set_parameter(Self::WALK_TO_IDLE, Parameter::Rule(!input.is_walking))
            .set_parameter(Self::WALK_TO_JUMP, Parameter::Rule(input.is_jumping))
            .set_parameter(Self::IDLE_TO_JUMP, Parameter::Rule(input.is_jumping))
            .set_parameter(
                Self::JUMP_TO_IDLE,
                Parameter::Rule(!input.is_jumping && jump_ended),
            );
    }
}

#[derive(Default)]
pub struct Player {
    pub capsule_collider: Handle<Node>,
    pub body: Handle<Node>,
    pub pivot: Handle<Node>,
    pub camera_pivot: Handle<Node>,
    pub camera_hinge: Handle<Node>,
    pub camera: Handle<Node>,
    pub model: Handle<Node>,
    pub controller: InputController,
    pub locomotion_machine: LocomotionMachine,
    pub model_yaw: SmoothAngle,
    pub animation_player: Handle<Node>,
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
        let camera_pivot = PivotBuilder::new(BaseBuilder::new().with_children(&[{
            camera_hinge = PivotBuilder::new(
                BaseBuilder::new()
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
                    }]),
            )
            .build(&mut scene.graph);
            camera_hinge
        }]))
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
            .request_model("examples/data/mutant/mutant.FBX")
            .await
            .unwrap();

        context
            .lock()
            .unwrap()
            .report_progress(0.60, "Instantiating model...");

        let model_handle = model_resource.instantiate(scene);

        let animation_player = scene
            .graph
            .find(model_handle, &mut |n| {
                n.query_component_ref::<AnimationPlayer>().is_some()
            })
            .unwrap()
            .0;

        let body_height = 0.5;

        // Now we have whole sub-graph instantiated, we can start modifying model instance.
        scene.graph[model_handle]
            .local_transform_mut()
            .set_position(Vector3::new(0.0, -body_height * 2.0, 0.0))
            // Our model is too big, fix it by scale.
            .set_scale(Vector3::new(0.0125, 0.0125, 0.0125));

        let pivot;
        let capsule_collider;
        let body = RigidBodyBuilder::new(
            BaseBuilder::new()
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vector3::new(0.0, 2.0, 0.0))
                        .build(),
                )
                .with_children(&[
                    {
                        capsule_collider = ColliderBuilder::new(BaseBuilder::new())
                            .with_shape(ColliderShape::capsule_y(body_height, 0.6))
                            .build(&mut scene.graph);
                        capsule_collider
                    },
                    {
                        pivot =
                            PivotBuilder::new(BaseBuilder::new().with_children(&[model_handle]))
                                .build(&mut scene.graph);
                        pivot
                    },
                ]),
        )
        .with_locked_rotations(true)
        .with_can_sleep(false)
        .with_body_type(RigidBodyType::Dynamic)
        .build(&mut scene.graph);

        context
            .lock()
            .unwrap()
            .report_progress(0.80, "Creating machine...");

        let locomotion_machine =
            LocomotionMachine::new(scene, model_handle, resource_manager, animation_player).await;

        Self {
            capsule_collider,
            body,
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
            animation_player,
        }
    }

    pub fn update(&mut self, scene: &mut Scene, dt: f32) {
        let pivot = &scene.graph[self.pivot];

        let look_vector = pivot
            .look_vector()
            .try_normalize(f32::EPSILON)
            .unwrap_or_else(Vector3::z);

        let side_vector = pivot
            .side_vector()
            .try_normalize(f32::EPSILON)
            .unwrap_or_else(Vector3::x);

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
            .try_normalize(f32::EPSILON)
            .map(|v| v.scale(speed))
            .unwrap_or_default();
        let is_moving = velocity.norm_squared() > 0.0;

        let animation_player = scene.graph[self.animation_player]
            .query_component_mut::<AnimationPlayer>()
            .unwrap();

        let mut new_y_vel = None;
        while let Some(event) = (**animation_player.animations_mut())
            .get_mut(self.locomotion_machine.jump_animation)
            .pop_event()
        {
            if event.signal_id == LocomotionMachine::JUMP_SIGNAL {
                new_y_vel = Some(6.0 * dt);
            }
        }

        let body = scene.graph[self.body].cast_mut::<RigidBody>().unwrap();

        let position = **body.local_transform().position();

        let quat_yaw = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.controller.yaw);

        body.set_ang_vel(Default::default());
        if let Some(new_y_vel) = new_y_vel {
            body.set_lin_vel(Vector3::new(
                velocity.x / dt,
                new_y_vel / dt,
                velocity.z / dt,
            ));
        } else {
            body.set_lin_vel(Vector3::new(
                velocity.x / dt,
                body.lin_vel().y,
                velocity.z / dt,
            ));
        }

        if is_moving {
            // Since we have free camera while not moving, we have to sync rotation of pivot
            // with rotation of camera so character will start moving in look direction.
            scene.graph[self.pivot]
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
            } else if self.controller.walk_backward {
                180.0
            } else {
                0.0
            };

            self.model_yaw.set_target(angle.to_radians()).update(dt);

            scene.graph[self.model].local_transform_mut().set_rotation(
                UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.model_yaw.angle),
            );
        }

        scene.graph[self.camera_pivot]
            .local_transform_mut()
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

        let mut has_ground_contact = false;
        if let Some(collider) = scene
            .graph
            .try_get(self.capsule_collider)
            .and_then(|n| n.cast::<Collider>())
        {
            'outer_loop: for contact in collider.contacts(&scene.graph.physics) {
                for manifold in contact.manifolds.iter() {
                    if manifold.local_n1.y > 0.7 {
                        has_ground_contact = true;
                        break 'outer_loop;
                    }
                }
            }
        }

        if has_ground_contact && self.controller.jump {
            let animation_player = scene.graph[self.animation_player]
                .query_component_mut::<AnimationPlayer>()
                .unwrap();

            // Rewind jump animation to beginning before jump.
            (**animation_player.animations_mut())
                .get_mut(self.locomotion_machine.jump_animation)
                .rewind();
        }

        // Make sure to apply animation machine pose to model explicitly.
        self.locomotion_machine.apply(
            scene,
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
                    .clamp(-90.0f32.to_radians(), 90.0f32.to_radians());
            }
            _ => {}
        }
    }

    pub fn handle_key_event(&mut self, key: &fyrox::event::KeyboardInput, _dt: f32) {
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
        fyrox::core::futures::executor::block_on(async move {
            let mut scene = Scene::new();

            // Set ambient light.
            scene.ambient_lighting_color = Color::opaque(80, 80, 80);

            // Create reverb effect for more natural sound - our player walks in some sort of cathedral,
            // so there will be pretty decent echo.
            let reverb_effect = ReverbEffectBuilder::new(
                BaseEffectBuilder::new()
                    // Make sure it won't be too loud - fyrox-sound doesn't care about energy conservation law, it
                    // just makes requested calculation.
                    .with_gain(0.7)
                    .with_name("Reverb".to_string()),
            )
            // Set reverb time to ~3 seconds - the more time the deeper the echo.
            .with_decay_time(3.0)
            .build(&mut scene.graph.sound_context);

            context
                .lock()
                .unwrap()
                .report_progress(0.25, "Loading map...");

            // Load simple map.
            resource_manager
                .request_model("examples/data/sponza/Sponza.rgs")
                .await
                .unwrap()
                .instantiate(&mut scene);

            scene.graph.update_hierarchical_data();

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
