//! Flying camera controller script is used to create flying cameras, that can be rotated via mouse and moved via keyboard keys.
//! See [`FlyingCameraController`] docs for more info and usage examples.

use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
        TypeUuidProvider,
    },
    event::{DeviceEvent, ElementState, Event, WindowEvent},
    gui::{key::KeyBinding, message::KeyCode},
    impl_component_provider,
    scene::node::Node,
    script::{ScriptContext, ScriptTrait},
    utils,
};
use std::ops::Range;

/// Flying camera controller script is used to create flying cameras, that can be rotated via mouse and moved via keyboard keys.
/// Use it, if you need to create a sort of "spectator" camera.
#[derive(Visit, Reflect, Debug, Clone)]
pub struct FlyingCameraController {
    pub camera: InheritableVariable<Handle<Node>>,
    pub yaw: InheritableVariable<f32>,
    pub pitch: InheritableVariable<f32>,
    pub speed: InheritableVariable<f32>,
    pub sensitivity: InheritableVariable<f32>,
    pub pitch_limit: InheritableVariable<Range<f32>>,

    // KeyBinding belongs to fyrox-ui which is unideal, this is only used here because it has built-in
    // property editor, so it will be shown in the editor correctly. It might be worth to create a
    // separate property editor for this instead to be able to use KeyCode here.
    pub move_forward_key: InheritableVariable<KeyBinding>,
    pub move_backward_key: InheritableVariable<KeyBinding>,
    pub move_left_key: InheritableVariable<KeyBinding>,
    pub move_right_key: InheritableVariable<KeyBinding>,

    #[reflect(hidden)]
    #[visit(skip)]
    move_forward: bool,
    #[reflect(hidden)]
    #[visit(skip)]
    move_backward: bool,
    #[reflect(hidden)]
    #[visit(skip)]
    move_left: bool,
    #[reflect(hidden)]
    #[visit(skip)]
    move_right: bool,
}

impl Default for FlyingCameraController {
    fn default() -> Self {
        Self {
            camera: Default::default(),
            yaw: Default::default(),
            pitch: Default::default(),
            speed: 5.0.into(),
            sensitivity: 0.7.into(),
            pitch_limit: (-89.9f32.to_radians()..89.9f32.to_radians()).into(),
            move_forward_key: KeyBinding::Some(KeyCode::KeyW).into(),
            move_backward_key: KeyBinding::Some(KeyCode::KeyS).into(),
            move_left_key: KeyBinding::Some(KeyCode::KeyA).into(),
            move_right_key: KeyBinding::Some(KeyCode::KeyD).into(),
            move_forward: false,
            move_backward: false,
            move_left: false,
            move_right: false,
        }
    }
}

impl_component_provider!(FlyingCameraController);

impl TypeUuidProvider for FlyingCameraController {
    fn type_uuid() -> Uuid {
        uuid!("8d9e2feb-8c61-482c-8ba4-b0b13b201113")
    }
}

impl ScriptTrait for FlyingCameraController {
    fn on_os_event(&mut self, event: &Event<()>, context: &mut ScriptContext) {
        match event {
            Event::WindowEvent { event, .. } => {
                if let WindowEvent::KeyboardInput { event, .. } = event {
                    for (binding, state) in [
                        (&self.move_forward_key, &mut self.move_forward),
                        (&self.move_backward_key, &mut self.move_backward),
                        (&self.move_left_key, &mut self.move_left),
                        (&self.move_right_key, &mut self.move_right),
                    ] {
                        if let KeyBinding::Some(key_code) = **binding {
                            if utils::translate_key_from_ui(key_code) == event.physical_key {
                                *state = event.state == ElementState::Pressed;
                            }
                        }
                    }
                }
            }
            Event::DeviceEvent { event, .. } => {
                if let DeviceEvent::MouseMotion { delta, .. } = event {
                    let speed = *self.sensitivity * context.dt;
                    *self.yaw -= (delta.0 as f32) * speed;
                    *self.pitch = (*self.pitch + delta.1 as f32 * speed)
                        .max(self.pitch_limit.start)
                        .min(self.pitch_limit.end);
                }
            }
            _ => {}
        }
    }

    fn on_update(&mut self, context: &mut ScriptContext) {
        if let Some(pivot) = context.scene.graph.try_get_mut(*self.camera) {
            pivot
                .local_transform_mut()
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &Vector3::x_axis(),
                    *self.pitch,
                ));
        }

        let this = &mut context.scene.graph[context.handle];

        this.local_transform_mut()
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::y_axis(),
                *self.yaw,
            ));

        let mut velocity = Vector3::default();
        if self.move_forward {
            velocity += this.look_vector();
        }
        if self.move_backward {
            velocity -= this.look_vector();
        }
        if self.move_left {
            velocity += this.side_vector();
        }
        if self.move_right {
            velocity -= this.side_vector();
        }
        if let Some(normalized_velocity) = velocity.try_normalize(f32::EPSILON) {
            this.local_transform_mut()
                .offset(normalized_velocity.scale(*self.speed * context.dt));
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}
