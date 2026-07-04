// THEATRE-BV: Bevy layer (BV) — native ECS rendering inside the Theatre canvas.
//
// BV layers live entirely inside Bevy's world. There is no OutputHandler bridge
// needed here — Bevy IS the renderer for BV. Other layer types (P5, GL, HT)
// will use the OutputHandler trait when they render via WebView/wgpu compute.
//
// Phase 2 demo: a spinning mesh visible in the canvas. In Phase 6 this will be
// replaced by scene code loaded from a GlyphPreset stored in myth-vault.

use bevy::prelude::*;
use myth_wire::ChannelId;

use crate::compositor::TheatreMixer;

// ── Components ────────────────────────────────────────────────────────────────

/// Marks any entity managed by a BV layer channel.
#[derive(Component)]
pub struct BvLayerEntity {
    pub channel_id: ChannelId,
}

/// Spins the entity around its local Y (and slightly X) axis.
#[derive(Component)]
pub struct Spinner {
    pub speed: f32,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct BevyLayerPlugin;

impl Plugin for BevyLayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_scene)
            .add_systems(Update, (spin_entities, sync_bv_visibility));
    }
}

// ── Startup ───────────────────────────────────────────────────────────────────

/// Spawn the default demo BV scene: a purple spinning torus knot-ish cluster.
///
/// In Phase 6 this will be replaced by scene code deserialized from the
/// GlyphPreset loaded on the channel. For now, channel 0 always gets this demo.
fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Centre cube — primary body
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(0.9, 0.9, 0.9)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.18, 0.88),
                metallic: 0.6,
                perceptual_roughness: 0.25,
                emissive: Color::srgb(0.05, 0.01, 0.18).into(),
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        Spinner { speed: 0.7 },
        BvLayerEntity {
            channel_id: ChannelId::new(0),
        },
    ));

    // Orbiting smaller cube
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(0.35, 0.35, 0.35)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.78, 0.70),
                metallic: 0.8,
                perceptual_roughness: 0.15,
                emissive: Color::srgb(0.0, 0.12, 0.11).into(),
                ..default()
            }),
            transform: Transform::from_xyz(1.6, 0.4, 0.0),
            ..default()
        },
        Spinner { speed: -1.4 },
        BvLayerEntity {
            channel_id: ChannelId::new(0),
        },
    ));

    // Directional light — primary sun
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 14_000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 10.0, 6.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Soft fill light from below
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 4_000.0,
            color: Color::srgb(0.3, 0.1, 0.6),
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_xyz(-2.0, -2.0, 2.0),
        ..default()
    });
}

// ── Update systems ────────────────────────────────────────────────────────────

fn spin_entities(time: Res<Time>, mut query: Query<(&mut Transform, &Spinner)>) {
    let dt = time.delta_seconds();
    for (mut tf, spinner) in query.iter_mut() {
        tf.rotate_y(spinner.speed * dt);
        tf.rotate_x(spinner.speed * 0.28 * dt);
    }
}

/// Show/hide BV entities based on their channel's mute state and level.
fn sync_bv_visibility(
    mixer: Res<TheatreMixer>,
    mut query: Query<(&BvLayerEntity, &mut Visibility)>,
) {
    if !mixer.is_changed() {
        return;
    }
    for (entity, mut vis) in query.iter_mut() {
        *vis = match mixer.0.channel(entity.channel_id) {
            Some(ch) if !ch.muted && ch.level > 0.0 => Visibility::Visible,
            _ => Visibility::Hidden,
        };
    }
}
