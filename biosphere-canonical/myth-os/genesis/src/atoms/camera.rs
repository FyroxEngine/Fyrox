// GEN-CAM: Orbit + Fly camera controller.
//
// Default mode: Orbit (spherical arm around a focus point).
//   Right-drag  → yaw + pitch
//   Scroll      → zoom distance (4–140 units)
//
// Fly mode (F key to toggle, Esc to exit):
//   Mouse-look  → yaw + pitch (cursor locked)
//   WASD        → forward / backward / strafe
//   Q / E       → descend / ascend
//   Shift       → 3.5× speed
//
// The rig's yaw + pitch are shared between modes so the transition is seamless.

use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    window::CursorGrabMode,
};

// ── Tuning ────────────────────────────────────────────────────────────────────

const ORBIT_ROT_SENS:    f32 = 0.004;
const ORBIT_ZOOM_SENS:   f32 = 1.8;
const FLY_MOUSE_SENS:    f32 = 0.003;
const FLY_BASE_SPEED:    f32 = 8.0;
const FLY_SPRINT_MUL:    f32 = 3.5;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum CamMode {
    #[default]
    Orbit,
    Fly,
}

/// Shared spherical-coordinate state — drives both Orbit and Fly.
#[derive(Resource)]
pub struct CameraRig {
    pub mode:     CamMode,
    pub yaw:      f32,   // radians, rotation around world Y
    pub pitch:    f32,   // radians, negative = looking down
    pub distance: f32,   // orbit arm length (Fly keeps this for mode-switch continuity)
    pub focus:    Vec3,  // orbit pivot — updated each Fly frame so switch back is smooth
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            mode:     CamMode::Orbit,
            yaw:      0.0,
            pitch:    -0.45,
            distance: 28.0,
            focus:    Vec3::new(0.0, 3.0, 0.0),
        }
    }
}

/// Marker component on the main 3-D scene camera entity.
#[derive(Component)]
pub struct SceneCamera;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraRig>()
            .add_systems(Startup, spawn_lights_and_camera)
            .add_systems(Update, drive_camera);
    }
}

// ── Startup ───────────────────────────────────────────────────────────────────

fn spawn_lights_and_camera(mut commands: Commands) {
    // Directional sun
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance:     12_000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_xyz(6.0, 18.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    // Camera — orbit math repositions it every frame, initial pos is just a fallback
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 22.0, 28.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        SceneCamera,
    ));
}

// ── Per-frame update ──────────────────────────────────────────────────────────

fn drive_camera(
    time:        Res<Time>,
    mut rig:     ResMut<CameraRig>,
    keys:        Res<ButtonInput<KeyCode>>,
    mouse_btns:  Res<ButtonInput<MouseButton>>,
    mut motions: EventReader<MouseMotion>,
    mut scrolls: EventReader<MouseWheel>,
    mut cam_q:   Query<&mut Transform, With<SceneCamera>>,
    mut wins:    Query<&mut Window>,
) {
    let Ok(mut tf) = cam_q.get_single_mut() else {
        for _ in motions.read() {}
        for _ in scrolls.read() {}
        return;
    };

    // ── Mode toggle ───────────────────────────────────────────────────────────
    if keys.just_pressed(KeyCode::KeyF) {
        rig.mode = match rig.mode {
            CamMode::Orbit => CamMode::Fly,
            CamMode::Fly   => CamMode::Orbit,
        };
        set_cursor_locked(&mut wins, rig.mode == CamMode::Fly);
    }
    if keys.just_pressed(KeyCode::Escape) && rig.mode == CamMode::Fly {
        rig.mode = CamMode::Orbit;
        set_cursor_locked(&mut wins, false);
    }

    // ── Accumulate raw input ──────────────────────────────────────────────────
    let mut mouse_delta = Vec2::ZERO;
    for ev in motions.read() {
        mouse_delta += ev.delta;
    }

    let mut scroll_y = 0.0_f32;
    for ev in scrolls.read() {
        scroll_y += ev.y;
    }

    // ── Dispatch ──────────────────────────────────────────────────────────────
    match rig.mode {
        CamMode::Orbit => orbit_update(&mut rig, &mut tf, &mouse_btns, mouse_delta, scroll_y),
        CamMode::Fly   => fly_update(&mut rig, &mut tf, &keys, &time, mouse_delta),
    }
}

fn orbit_update(
    rig:        &mut CameraRig,
    tf:         &mut Transform,
    mouse_btns: &ButtonInput<MouseButton>,
    delta:      Vec2,
    scroll:     f32,
) {
    if mouse_btns.pressed(MouseButton::Right) {
        rig.yaw   -= delta.x * ORBIT_ROT_SENS;
        rig.pitch  = (rig.pitch - delta.y * ORBIT_ROT_SENS).clamp(-1.45, -0.04);
    }
    rig.distance = (rig.distance - scroll * ORBIT_ZOOM_SENS).clamp(4.0, 140.0);

    let rot       = Quat::from_euler(EulerRot::YXZ, rig.yaw, rig.pitch, 0.0);
    tf.translation = rig.focus + rot * Vec3::new(0.0, 0.0, rig.distance);
    tf.look_at(rig.focus, Vec3::Y);
}

fn fly_update(
    rig:   &mut CameraRig,
    tf:    &mut Transform,
    keys:  &ButtonInput<KeyCode>,
    time:  &Time,
    delta: Vec2,
) {
    // Mouse-look
    rig.yaw   -= delta.x * FLY_MOUSE_SENS;
    rig.pitch  = (rig.pitch - delta.y * FLY_MOUSE_SENS).clamp(-1.45, 1.45);

    let rot   = Quat::from_euler(EulerRot::YXZ, rig.yaw, rig.pitch, 0.0);
    let fwd   = rot * Vec3::NEG_Z;
    let right = rot * Vec3::X;

    let mut vel = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { vel += fwd;   }
    if keys.pressed(KeyCode::KeyS) { vel -= fwd;   }
    if keys.pressed(KeyCode::KeyA) { vel -= right;  }
    if keys.pressed(KeyCode::KeyD) { vel += right;  }
    if keys.pressed(KeyCode::KeyE) || keys.pressed(KeyCode::Space) { vel += Vec3::Y; }
    if keys.pressed(KeyCode::KeyQ) { vel -= Vec3::Y; }

    let speed = if keys.pressed(KeyCode::ShiftLeft) {
        FLY_BASE_SPEED * FLY_SPRINT_MUL
    } else {
        FLY_BASE_SPEED
    };

    tf.translation += vel.normalize_or_zero() * speed * time.delta_seconds();
    tf.rotation     = rot;

    // Keep focus in sync so orbit mode re-enters smoothly
    rig.focus = tf.translation - rot * Vec3::new(0.0, 0.0, rig.distance);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn set_cursor_locked(wins: &mut Query<&mut Window>, locked: bool) {
    if let Ok(mut win) = wins.get_single_mut() {
        win.cursor.visible   = !locked;
        win.cursor.grab_mode = if locked {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
    }
}
