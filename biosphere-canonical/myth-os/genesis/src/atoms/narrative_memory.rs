// GEN-ATOM-11: Narrative Memory Trace — causal event streams
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct NarrativeMemory {
    pub events: Vec<String>,
}

pub struct NarrativePlugin;
impl Plugin for NarrativePlugin {
    fn build(&self, _app: &mut App) {}
}
