use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        futures::executor::block_on,
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        uuid::Uuid,
        visitor::prelude::*,
    },
    event::{DeviceEvent, ElementState, Event, VirtualKeyCode, WindowEvent},
    gui::{
        button::ButtonBuilder,
        inspector::{FieldKind, PropertyChanged},
        widget::WidgetBuilder,
        UserInterface,
    },
    plugin::{Plugin, PluginContext, PluginRegistrationContext},
    renderer::{
        framework::{error::FrameworkError, gpu_texture::GpuTextureKind},
        ui_renderer::UiRenderContext,
        RenderPassStatistics, SceneRenderPass, SceneRenderPassContext,
    },
    scene::{
        camera::Camera, node::Node, node::TypeUuidProvider, rigidbody::RigidBody, Scene,
        SceneLoader,
    },
    script::{ScriptContext, ScriptTrait},
    utils::translate_event,
};
use std::{cell::RefCell, rc::Rc, str::FromStr};

pub struct GamePlugin {
    scene: Handle<Scene>,
    ui: Rc<RefCell<UserInterface>>,
}

impl GamePlugin {
    pub fn new() -> Self {
        Self {
            scene: Default::default(),
            ui: Rc::new(RefCell::new(UserInterface::new(Vector2::new(100.0, 100.0)))),
        }
    }
}

struct UiRenderPass {
    scene: Handle<Scene>,
    ui: Rc<RefCell<UserInterface>>,
}

impl SceneRenderPass for UiRenderPass {
    fn on_ldr_render(
        &mut self,
        ctx: SceneRenderPassContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        if ctx.scene_handle == self.scene {
            let mut ui = self.ui.borrow_mut();

            ctx.ui_renderer.render(UiRenderContext {
                state: ctx.pipeline_state,
                viewport: ctx.viewport,
                frame_buffer: ctx.framebuffer,
                frame_width: ctx.viewport.size.x as f32,
                frame_height: ctx.viewport.size.y as f32,
                drawing_context: ui.draw(),
                white_dummy: ctx.white_dummy.clone(),
                texture_cache: ctx.texture_cache,
            })?;
        }

        Ok(Default::default())
    }
}

impl GamePlugin {
    pub fn set_scene(&mut self, scene: Handle<Scene>, context: PluginContext) {
        self.scene = scene;

        context
            .renderer
            .add_render_pass(Rc::new(RefCell::new(UiRenderPass {
                scene,
                ui: self.ui.clone(),
            })));

        let mut ui = self.ui.borrow_mut();
        let ctx = &mut ui.build_ctx();
        ButtonBuilder::new(WidgetBuilder::new().with_width(200.0).with_height(32.0))
            .with_text("Click me")
            .build(ctx);
    }
}

impl TypeUuidProvider for GamePlugin {
    fn type_uuid() -> Uuid {
        Uuid::from_str("a9507fb2-0945-4fc1-91ce-115ae7c8a615").unwrap()
    }
}

impl Plugin for GamePlugin {
    fn on_register(&mut self, context: PluginRegistrationContext) {
        context
            .serialization_context
            .script_constructors
            .add::<GamePlugin, Player, &str>("Player");

        context
            .serialization_context
            .script_constructors
            .add::<GamePlugin, Jumper, &str>("Jumper");
    }

    fn on_standalone_init(&mut self, context: PluginContext) {
        let mut scene = block_on(
            block_on(SceneLoader::from_file(
                "unnamed.rgs",
                context.serialization_context.clone(),
            ))
            .expect("Invalid scene!")
            .finish(context.resource_manager.clone()),
        );

        for node in scene.graph.linear_iter_mut() {
            if let Some(camera) = node.cast_mut::<Camera>() {
                camera.set_enabled(true);
            }
        }

        self.set_scene(context.scenes.add(scene), context);
    }

    fn on_enter_play_mode(&mut self, scene: Handle<Scene>, context: PluginContext) {
        self.set_scene(scene, context);
    }

    fn on_leave_play_mode(&mut self, _context: PluginContext) {
        self.scene = Handle::NONE;
    }

    fn on_unload(&mut self, _context: &mut PluginContext) {}

    fn update(&mut self, context: &mut PluginContext) {
        let scene = &mut context.scenes[self.scene];

        let drawing_context = &mut scene.drawing_context;
        drawing_context.clear_lines();
        scene.graph.physics.draw(drawing_context);

        let mut ui = self.ui.borrow_mut();

        if let Some(data) = context.renderer.scene_data_map.get(&self.scene) {
            if let GpuTextureKind::Rectangle { width, height } =
                data.ldr_scene_frame_texture().borrow().kind()
            {
                ui.update(Vector2::new(width as f32, height as f32), context.dt);
            }
        }

        while let Some(_) = ui.poll_message() {}
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn on_os_event(&mut self, event: &Event<()>, _context: PluginContext) {
        if let Event::WindowEvent { event, .. } = event {
            if let Some(e) = translate_event(event) {
                let mut ui = self.ui.borrow_mut();
                ui.process_os_event(&e);
            }
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

#[derive(Visit, Inspect, Debug, Clone)]
struct Player {
    speed: f32,
    yaw: f32,

    #[visit(optional)]
    pitch: f32,

    #[visit(optional)]
    camera: Handle<Node>,

    #[visit(skip)]
    #[inspect(skip)]
    controller: InputController,
}

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
        Uuid::from_str("4aa165aa-011b-479f-bc10-b90b2c4b5060").unwrap()
    }
}

impl ScriptTrait for Player {
    fn on_property_changed(&mut self, args: &PropertyChanged) {
        if let FieldKind::Object(ref value) = args.value {
            match args.name.as_ref() {
                Self::SPEED => self.speed = value.cast_clone().unwrap(),
                Self::YAW => self.yaw = value.cast_clone().unwrap(),
                Self::CAMERA => self.camera = value.cast_clone().unwrap(),
                _ => (),
            }
        }
    }

    fn on_init(&mut self, context: ScriptContext) {
        let ScriptContext { node, scene, .. } = context;

        for &child in node.children() {
            if scene.graph[child].name() == "Camera" {
                self.camera = child;
                break;
            }
        }
    }

    fn on_update(&mut self, context: ScriptContext) {
        let ScriptContext {
            dt, node, scene, ..
        } = context;

        node.local_transform_mut()
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::y_axis(),
                self.yaw,
            ));

        if let Some(body) = node.cast_mut::<RigidBody>() {
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

            let speed = 2.0 * dt;
            let velocity = velocity
                .try_normalize(f32::EPSILON)
                .map(|v| v.scale(speed))
                .unwrap_or_default();

            body.set_ang_vel(Default::default());
            body.set_lin_vel(Vector3::new(
                velocity.x / dt,
                body.lin_vel().y,
                velocity.z / dt,
            ));
        }

        if let Some(camera) = scene.graph.try_get_mut(self.camera) {
            camera
                .local_transform_mut()
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &Vector3::x_axis(),
                    self.pitch,
                ));
        }
    }

    fn on_os_event(&mut self, event: &Event<()>, _context: ScriptContext) {
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

    fn plugin_uuid(&self) -> Uuid {
        GamePlugin::type_uuid()
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}

#[derive(Visit, Inspect, Debug, Clone)]
struct Jumper {
    timer: f32,

    #[visit(optional)]
    period: f32,
}

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
        Uuid::from_str("942e9f5b-e036-4357-b514-91060d4059f5").unwrap()
    }
}

impl ScriptTrait for Jumper {
    fn on_property_changed(&mut self, args: &PropertyChanged) {
        if let FieldKind::Object(ref value) = args.value {
            match args.name.as_ref() {
                Self::TIMER => self.timer = value.cast_clone().unwrap(),
                Self::PERIOD => self.period = value.cast_clone().unwrap(),
                _ => (),
            }
        }
    }

    fn on_init(&mut self, _context: ScriptContext) {}

    fn on_update(&mut self, context: ScriptContext) {
        if let Some(rigid_body) = context.node.cast_mut::<RigidBody>() {
            if self.timer > self.period {
                rigid_body.apply_force(Vector3::new(0.0, 200.0, 0.0));
                self.timer = 0.0;
            }

            self.timer += context.dt;
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        GamePlugin::type_uuid()
    }
}

// Script entry point.
#[no_mangle]
pub extern "C" fn fyrox_main() -> Box<Box<dyn Plugin>> {
    Box::new(Box::new(GamePlugin::new()))
}
