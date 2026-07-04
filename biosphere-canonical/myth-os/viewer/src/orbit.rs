use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
};
use bevy_egui::EguiContexts;

#[derive(Resource)]
pub struct Orbit {
    pub target: Vec3,
    pub radius: f32,
    pub yaw:    f32,
    pub pitch:  f32,
}

impl Default for Orbit {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            radius: 5.0,
            yaw:    0.4,
            pitch:  0.3,
        }
    }
}

pub fn run(
    mut orbit:     ResMut<Orbit>,
    mut camera_q:  Query<&mut Transform, With<Camera3d>>,
    mut contexts:  EguiContexts,
    mouse_btn:     Res<ButtonInput<MouseButton>>,
    mut motion:    EventReader<MouseMotion>,
    mut scroll:    EventReader<MouseWheel>,
) {
    let egui_busy = contexts.ctx_mut().wants_pointer_input();

    for ev in scroll.read() {
        if !egui_busy {
            orbit.radius = (orbit.radius - ev.y * 0.6).clamp(0.1, 500.0);
        }
    }

    if mouse_btn.pressed(MouseButton::Left) && !egui_busy {
        for ev in motion.read() {
            orbit.yaw   -= ev.delta.x * 0.005;
            orbit.pitch  = (orbit.pitch - ev.delta.y * 0.005).clamp(-1.55, 1.55);
        }
    } else {
        // drain so events don't queue up across frames
        for _ in motion.read() {}
    }

    let Ok(mut cam) = camera_q.get_single_mut() else { return };
    let pos = Vec3::new(
        orbit.radius * orbit.pitch.cos() * orbit.yaw.sin(),
        orbit.radius * orbit.pitch.sin(),
        orbit.radius * orbit.pitch.cos() * orbit.yaw.cos(),
    ) + orbit.target;

    cam.translation = pos;
    cam.look_at(orbit.target, Vec3::Y);
}
