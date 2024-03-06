//! Flying camera controller script is used to create flying cameras, that can be rotated via mouse and moved via keyboard keys.
//! See [`FlyingCameraController`] docs for more info and usage examples.

use fyrox::{
    core::{
        algebra::{UnitQuaternion, UnitVector3, Vector3},
        impl_component_provider,
        math::curve::{Curve, CurveKey, CurveKeyKind},
        math::Vector3Ext,
        reflect::prelude::*,
        uuid_provider,
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    event::{DeviceEvent, ElementState, Event, WindowEvent},
    gui::{key::KeyBinding, message::KeyCode},
    script::{ScriptContext, ScriptTrait},
    utils,
};
use std::ops::Range;

/// Flying camera controller script is used to create flying cameras, that can be rotated via mouse and moved via keyboard keys.
/// Use it, if you need to create a sort of "spectator" camera. To use it, all you need to do is to assign it to your camera
/// node (or one if its parent nodes).
#[derive(Visit, Reflect, Debug, Clone)]
pub struct FlyingCameraController {
    #[reflect(description = "Current yaw of the camera pivot (in radians).")]
    #[visit(optional)]
    pub yaw: InheritableVariable<f32>,

    #[reflect(description = "Current pitch of the camera (in radians).")]
    #[visit(optional)]
    pub pitch: InheritableVariable<f32>,

    #[reflect(description = "Maximum speed of the camera.")]
    #[visit(optional)]
    pub speed: InheritableVariable<f32>,

    #[reflect(description = "Mouse sensitivity.")]
    #[visit(optional)]
    pub sensitivity: InheritableVariable<f32>,

    #[reflect(description = "Angular limit of the pitch of the camera (in radians).")]
    #[visit(optional)]
    pub pitch_limit: InheritableVariable<Range<f32>>,

    // KeyBinding belongs to fyrox-ui which is unideal, this is only used here because it has built-in
    // property editor, so it will be shown in the editor correctly. It might be worth to create a
    // separate property editor for this instead to be able to use KeyCode here.
    #[reflect(description = "A key, that corresponds to forward movement.")]
    #[visit(optional)]
    pub move_forward_key: InheritableVariable<KeyBinding>,

    #[reflect(description = "A key, that corresponds to backward movement.")]
    #[visit(optional)]
    pub move_backward_key: InheritableVariable<KeyBinding>,

    #[reflect(description = "A key, that corresponds to left movement.")]
    #[visit(optional)]
    pub move_left_key: InheritableVariable<KeyBinding>,

    #[reflect(description = "A key, that corresponds to right movement.")]
    #[visit(optional)]
    pub move_right_key: InheritableVariable<KeyBinding>,

    #[reflect(
        description = "A curve, that defines a how speed of the camera changes when accelerating to the \
    max speed."
    )]
    #[visit(optional)]
    pub acceleration_curve: InheritableVariable<Curve>,

    #[reflect(
        description = "A curve, that defines a how speed of the camera changes when decelerating to the \
    zero speed."
    )]
    #[visit(optional)]
    pub deceleration_curve: InheritableVariable<Curve>,

    #[reflect(
        description = "Amount of time (in seconds) during which the camera will accelerate to the max speed.",
        min_value = 0.0
    )]
    #[visit(optional)]
    pub acceleration_time: InheritableVariable<f32>,

    #[reflect(
        description = "Amount of time (in seconds) during which the camera will decelerate to the zero speed.",
        min_value = 0.0
    )]
    #[visit(optional)]
    pub deceleration_time: InheritableVariable<f32>,

    #[reflect(
        description = "A coefficient, that defines how fast the camera will respond to pressed keys.",
        min_value = 0.01,
        max_value = 1.0
    )]
    #[visit(optional)]
    pub reactivity: InheritableVariable<f32>,

    #[reflect(hidden)]
    #[visit(optional)]
    pub velocity: InheritableVariable<Vector3<f32>>,

    #[reflect(hidden)]
    #[visit(optional)]
    pub target_velocity: InheritableVariable<Vector3<f32>>,

    #[reflect(hidden)]
    #[visit(skip)]
    pub acceleration_coeff: f32,

    #[reflect(hidden)]
    #[visit(skip)]
    pub move_forward: bool,

    #[reflect(hidden)]
    #[visit(skip)]
    pub move_backward: bool,

    #[reflect(hidden)]
    #[visit(skip)]
    pub move_left: bool,

    #[reflect(hidden)]
    #[visit(skip)]
    pub move_right: bool,
}

impl Default for FlyingCameraController {
    fn default() -> Self {
        Self {
            yaw: Default::default(),
            pitch: Default::default(),
            speed: 5.0.into(),
            sensitivity: 0.7.into(),
            pitch_limit: (-89.9f32.to_radians()..89.9f32.to_radians()).into(),
            move_forward_key: KeyBinding::Some(KeyCode::KeyW).into(),
            move_backward_key: KeyBinding::Some(KeyCode::KeyS).into(),
            move_left_key: KeyBinding::Some(KeyCode::KeyA).into(),
            move_right_key: KeyBinding::Some(KeyCode::KeyD).into(),
            acceleration_curve: Curve::from(vec![
                CurveKey::new(
                    0.0,
                    0.0,
                    CurveKeyKind::Cubic {
                        left_tangent: 0.0,
                        right_tangent: 0.0,
                    },
                ),
                CurveKey::new(
                    1.0,
                    1.0,
                    CurveKeyKind::Cubic {
                        left_tangent: 0.0,
                        right_tangent: 0.0,
                    },
                ),
            ])
            .into(),
            deceleration_curve: Curve::from(vec![
                CurveKey::new(
                    0.0,
                    0.0,
                    CurveKeyKind::Cubic {
                        left_tangent: 0.0,
                        right_tangent: 0.0,
                    },
                ),
                CurveKey::new(
                    1.0,
                    1.0,
                    CurveKeyKind::Cubic {
                        left_tangent: 0.0,
                        right_tangent: 0.0,
                    },
                ),
            ])
            .into(),
            acceleration_time: 0.25.into(),
            deceleration_time: 1.0.into(),
            velocity: Default::default(),
            target_velocity: Default::default(),
            acceleration_coeff: 0.0,
            reactivity: 0.3.into(),
            move_forward: false,
            move_backward: false,
            move_left: false,
            move_right: false,
        }
    }
}

impl_component_provider!(FlyingCameraController);
uuid_provider!(FlyingCameraController = "8d9e2feb-8c61-482c-8ba4-b0b13b201113");

impl ScriptTrait for FlyingCameraController {
    fn on_os_event(&mut self, event: &Event<()>, context: &mut ScriptContext) {
        match event {
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event, .. },
                ..
            } => {
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
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta, .. },
                ..
            } => {
                let speed = *self.sensitivity * context.dt;
                *self.yaw -= (delta.0 as f32) * speed;
                *self.pitch = (*self.pitch + delta.1 as f32 * speed)
                    .max(self.pitch_limit.start)
                    .min(self.pitch_limit.end);
            }
            _ => {}
        }
    }

    fn on_update(&mut self, context: &mut ScriptContext) {
        let mut new_velocity = Vector3::default();

        let this = &mut context.scene.graph[context.handle];

        if self.move_forward {
            new_velocity += this.look_vector();
        }
        if self.move_backward {
            new_velocity -= this.look_vector();
        }
        if self.move_left {
            new_velocity += this.side_vector();
        }
        if self.move_right {
            new_velocity -= this.side_vector();
        }

        if let Some(new_normalized_velocity) = new_velocity.try_normalize(f32::EPSILON) {
            self.acceleration_coeff = (self.acceleration_coeff
                + context.dt / self.acceleration_time.max(context.dt))
            .min(1.0);
            *self.target_velocity = new_normalized_velocity.scale(
                *self.speed
                    * self.acceleration_curve.value_at(self.acceleration_coeff)
                    * context.dt,
            );
        } else {
            self.acceleration_coeff = (self.acceleration_coeff
                - context.dt / self.deceleration_time.max(context.dt))
            .max(0.0);
            if let Some(normalized_velocity) = self.target_velocity.try_normalize(f32::EPSILON) {
                *self.target_velocity = normalized_velocity.scale(
                    *self.speed
                        * self.deceleration_curve.value_at(self.acceleration_coeff)
                        * context.dt,
                );
            } else {
                *self.target_velocity = Vector3::zeros();
            }
        }

        self.velocity
            .follow(&self.target_velocity, *self.reactivity);

        let yaw = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), *self.yaw);
        this.local_transform_mut()
            .set_rotation(
                UnitQuaternion::from_axis_angle(
                    &UnitVector3::new_normalize(yaw * Vector3::x()),
                    *self.pitch,
                ) * yaw,
            )
            .offset(*self.velocity);
    }
}
