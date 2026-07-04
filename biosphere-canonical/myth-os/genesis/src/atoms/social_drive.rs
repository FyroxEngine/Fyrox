// GEN-ATOM-10: Social Drive Injector — agent need modulation
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct SocialDrives {
    pub hunger: f32,
    pub fear: f32,
    pub prestige: f32,
}

pub struct SocialDrivePlugin;
impl Plugin for SocialDrivePlugin {
    fn build(&self, _app: &mut App) {}
}
