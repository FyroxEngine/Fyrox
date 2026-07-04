// GEN-ATOM-05: Gravitational Field Setter
use bevy::prelude::*;

#[derive(Resource)]
pub struct GravityConfig {
    pub vector: Vec3,
}

impl Default for GravityConfig {
    fn default() -> Self {
        Self { vector: Vec3::new(0.0, -9.81, 0.0) }
    }
}

pub struct GravityPlugin;

impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GravityConfig>();
    }
}
