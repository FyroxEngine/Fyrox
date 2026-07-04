// GEN-ATOM-01: World Anchor Seeder
// Establishes the origin frame (0, 0, 0) for all coordinate space in the simulation.

use bevy::prelude::*;

/// Marker component — exactly one entity per world carries this.
#[derive(Component, Default)]
pub struct WorldAnchor;

/// Bevy resource holding the world origin transform.
#[derive(Resource, Default)]
pub struct WorldOrigin {
    pub transform: Transform,
}

pub struct WorldAnchorPlugin;

impl Plugin for WorldAnchorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldOrigin>()
            .add_systems(Startup, seed_world_anchor);
    }
}

fn seed_world_anchor(mut commands: Commands, mut origin: ResMut<WorldOrigin>) {
    let anchor_transform = Transform::from_translation(Vec3::ZERO);
    origin.transform = anchor_transform;

    commands.spawn((
        WorldAnchor,
        SpatialBundle {
            transform: anchor_transform,
            ..Default::default()
        },
        Name::new("WorldAnchor"),
    ));

    info!("WorldAnchor seeded at origin (0, 0, 0)");
}
