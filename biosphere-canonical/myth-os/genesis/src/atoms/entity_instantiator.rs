// GEN-ATOM-03: Entity Instantiator
// Spawns ECS entities from B-DNA genotype capsules (Quill Actors).

use bevy::prelude::*;
use mythos::identity::MythId;
use serde::{Deserialize, Serialize};

/// The core identity component every Quill Actor carries.
#[derive(Component, Debug, Clone)]
pub struct QuillActor {
    pub myth_id: String,
    pub genotype: Genotype,
    pub name: String,
}

/// Simplified B-DNA genotype: trait sliders [0.0..1.0].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genotype {
    pub aggression: f32,
    pub curiosity: f32,
    pub social_drive: f32,
    pub fear_threshold: f32,
    pub energy: f32,
}

impl Default for Genotype {
    fn default() -> Self {
        Self {
            aggression: 0.1,
            curiosity: 0.7,
            social_drive: 0.5,
            fear_threshold: 0.4,
            energy: 1.0,
        }
    }
}

/// Event: request to spawn a new Quill Actor.
#[derive(Event)]
pub struct SpawnActorRequest {
    pub name: String,
    pub genotype: Genotype,
    pub position: Vec3,
}

pub struct EntityInstantiatorPlugin;

impl Plugin for EntityInstantiatorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnActorRequest>()
            .add_systems(Update, handle_spawn_requests);
    }
}

fn handle_spawn_requests(
    mut commands: Commands,
    mut requests: EventReader<SpawnActorRequest>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for req in requests.read() {
        let id = MythId::new();
        info!(name = %req.name, id = %id, "Spawning Quill Actor");

        commands.spawn((
            QuillActor {
                myth_id: id.as_str(),
                genotype: req.genotype.clone(),
                name: req.name.clone(),
            },
            PbrBundle {
                mesh: meshes.add(Mesh::from(Capsule3d::new(0.3, 0.8))),
                material: materials.add(Color::srgb(0.4, 0.7, 0.9)),
                transform: Transform::from_translation(req.position),
                ..Default::default()
            },
            Name::new(req.name.clone()),
        ));
    }
}
