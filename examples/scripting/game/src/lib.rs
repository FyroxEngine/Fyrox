use crate::bot::Bot;
use fyrox::event_loop::ControlFlow;
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        futures::executor::block_on,
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    event::{DeviceEvent, ElementState, Event, VirtualKeyCode, WindowEvent},
    gui::{button::ButtonBuilder, widget::WidgetBuilder},
    impl_component_provider,
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    scene::{
        camera::Camera,
        graph::map::NodeHandleMap,
        node::{Node, TypeUuidProvider},
        rigidbody::RigidBody,
        Scene, SceneLoader,
    },
    script::{ScriptContext, ScriptTrait},
};

mod bot;

#[derive(Default)]
pub struct GamePlugin {
    debug_draw: bool,
    scene: Handle<Scene>,
}

pub struct GameConstructor;

impl PluginConstructor for GameConstructor {
    fn register(&self, context: PluginRegistrationContext) {
        let scripts = &context.serialization_context.script_constructors;

        scripts
            .add::<Player>("Player")
            .add::<Jumper>("Jumper")
            .add::<Bot>("Bot");
    }

    fn create_instance(
        &self,
        override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        Box::new(GamePlugin::new(override_scene, context))
    }
}

impl GamePlugin {
    fn new(override_scene: Handle<Scene>, context: PluginContext) -> Self {
        let scene = if override_scene.is_some() {
            override_scene
        } else {
            let scene = block_on(
                block_on(SceneLoader::from_file(
                    "data/scene.rgs",
                    context.serialization_context.clone(),
                ))
                .expect("Invalid scene!")
                .finish(context.resource_manager.clone()),
            );
            context.scenes.add(scene)
        };

        for node in context.scenes[scene].graph.linear_iter_mut() {
            if let Some(camera) = node.cast_mut::<Camera>() {
                camera.set_enabled(true);
            }
        }

        let ctx = &mut context.user_interface.build_ctx();
        ButtonBuilder::new(WidgetBuilder::new().with_width(200.0).with_height(32.0))
            .with_text("Click me")
            .build(ctx);

        Self {
            scene,
            debug_draw: true,
        }
    }
}

impl Plugin for GamePlugin {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        let scene = &mut context.scenes[self.scene];

        if self.debug_draw {
            let drawing_context = &mut scene.drawing_context;
            drawing_context.clear_lines();
            scene.graph.physics.draw(drawing_context);
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct InputController {
    walk_forward: bool,
    walk_backward: bool,
    walk_left: bool,
    walk_right: bool,
    jump: bool,
}

#[derive(Visit, Inspect, Reflect, Debug, Clone)]
struct Player {
    speed: f32,
    yaw: f32,
    pitch: f32,
    camera: Handle<Node>,

    #[visit(skip)]
    #[inspect(skip)]
    #[reflect(hidden)]
    controller: InputController,
}

impl_component_provider!(Player);

impl Default for Player {
    fn default() -> Self {
        Self {
            speed: 0.2,
            yaw: 0.0,
            pitch: 0.0,
            camera: Default::default(),
            controller: Default::default(),
        }
    }
}

impl TypeUuidProvider for Player {
    fn type_uuid() -> Uuid {
        uuid!("4aa165aa-011b-479f-bc10-b90b2c4b5060")
    }
}

impl ScriptTrait for Player {
    fn remap_handles(&mut self, old_new_mapping: &NodeHandleMap) {
        old_new_mapping.map(&mut self.camera);
    }

    fn on_update(&mut self, ctx: &mut ScriptContext) {
        if let Some(body) = ctx.scene.graph[ctx.handle].cast_mut::<RigidBody>() {
            body.local_transform_mut()
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &Vector3::y_axis(),
                    self.yaw,
                ));

            let look_vector = body
                .look_vector()
                .try_normalize(f32::EPSILON)
                .unwrap_or_else(Vector3::z);

            let side_vector = body
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

            let speed = 2.0 * ctx.dt;
            let velocity = velocity
                .try_normalize(f32::EPSILON)
                .map(|v| v.scale(speed))
                .unwrap_or_default();

            body.set_ang_vel(Default::default());
            body.set_lin_vel(Vector3::new(
                velocity.x / ctx.dt,
                body.lin_vel().y,
                velocity.z / ctx.dt,
            ));
        }

        if let Some(camera) = ctx.scene.graph.try_get_mut(self.camera) {
            camera
                .local_transform_mut()
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &Vector3::x_axis(),
                    self.pitch,
                ));
        }
    }

    #[allow(clippy::collapsible_match)] // False positive
    fn on_os_event(&mut self, event: &Event<()>, _context: &mut ScriptContext) {
        match event {
            Event::DeviceEvent { event, .. } => {
                if let DeviceEvent::MouseMotion { delta } = event {
                    let mouse_sens = 0.025;

                    self.yaw -= mouse_sens * delta.0 as f32;
                    self.pitch = (self.pitch + (delta.1 as f32) * mouse_sens)
                        .max(-90.0f32.to_radians())
                        .min(90.0f32.to_radians());
                }
            }
            Event::WindowEvent { event, .. } => {
                if let WindowEvent::KeyboardInput { input, .. } = event {
                    if let Some(key_code) = input.virtual_keycode {
                        match key_code {
                            VirtualKeyCode::W => {
                                self.controller.walk_forward = input.state == ElementState::Pressed
                            }
                            VirtualKeyCode::S => {
                                self.controller.walk_backward = input.state == ElementState::Pressed
                            }
                            VirtualKeyCode::A => {
                                self.controller.walk_left = input.state == ElementState::Pressed
                            }
                            VirtualKeyCode::D => {
                                self.controller.walk_right = input.state == ElementState::Pressed
                            }
                            VirtualKeyCode::Space => {
                                self.controller.jump = input.state == ElementState::Pressed
                            }
                            _ => (),
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}

#[derive(Visit, Inspect, Reflect, Debug, Clone)]
struct Jumper {
    timer: f32,
    period: f32,
}

impl_component_provider!(Jumper);

impl Default for Jumper {
    fn default() -> Self {
        Self {
            timer: 0.0,
            period: 0.5,
        }
    }
}

impl TypeUuidProvider for Jumper {
    fn type_uuid() -> Uuid {
        uuid!("942e9f5b-e036-4357-b514-91060d4059f5")
    }
}

impl ScriptTrait for Jumper {
    fn on_init(&mut self, _context: &mut ScriptContext) {}

    fn on_update(&mut self, ctx: &mut ScriptContext) {
        if let Some(rigid_body) = ctx.scene.graph[ctx.handle].cast_mut::<RigidBody>() {
            if self.timer > self.period {
                rigid_body.apply_force(Vector3::new(0.0, 200.0, 0.0));
                self.timer = 0.0;
            }

            self.timer += ctx.dt;
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}
