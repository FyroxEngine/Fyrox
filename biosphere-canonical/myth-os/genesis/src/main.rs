// Genesis is a simulation engine — many atoms and widgets are scaffolded ahead
// of full wiring. Suppress dead_code warnings across the whole binary crate.
#![allow(dead_code, unused_imports, unused_variables)]

mod asset_registry;
mod atoms;
mod containers;
mod mixer;
mod scanner;
mod ui;

use asset_registry::{AssetRegistry, AssetRegistryPlugin};
use atoms::{
    camera::{CameraPlugin, SceneCamera},
    collision::CollisionPlugin,
    container_mapper::ContainerMapperPlugin,
    dialogue::DialoguePlugin,
    entity_instantiator::{EntityInstantiatorPlugin, Genotype, SpawnActorRequest},
    fabrication::FabricationPlugin,
    gravity::GravityPlugin,
    instinct::InstinctPlugin,
    kinematics::KinematicsPlugin,
    narrative_memory::NarrativePlugin,
    reality_lock::RealityLockPlugin,
    servo::ServoPlugin,
    signal_broadcaster::SignalBroadcasterPlugin,
    social_drive::SocialDrivePlugin,
    thermodynamics::ThermodynamicsPlugin,
    topology::{TerrainEntity, TopologyPlugin},
    world_anchor::WorldAnchorPlugin,
};
use bevy::{log::LogPlugin, prelude::*, window::WindowPlugin};
use containers::ContainerPlugin;
use mixer::TraktorMixerPlugin;
use scanner::{ensure_module_manifests, ModuleRegistry};
use ui::rack::{RackState, RackUiPlugin};

fn main() {
    // Seed the 16 module manifests on disk if they're missing, then scan.
    ensure_module_manifests("assets/modules");
    let registry = ModuleRegistry::load_from("assets/modules");

    // Scan the assets/ directory for qforge manifests (arch, bio, mech, char, prop).
    let asset_registry = AssetRegistry::load_from("assets");

    App::new()
        // ── Bevy defaults ──────────────────────────────────────────────────
        .add_plugins(
            DefaultPlugins.build()
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Genesis — World".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .set(LogPlugin {
                    filter: "genesis=debug,bevy_render=warn,bevy_ecs=warn,wgpu=error,egui=warn".into(),
                    level: bevy::log::Level::DEBUG,
                    ..Default::default()
                }),
        )
        // ── Sky colour & ambient light ─────────────────────────────────────
        .insert_resource(ClearColor(Color::srgb(0.42, 0.62, 0.85)))
        .insert_resource(AmbientLight {
            color:      Color::srgb(0.38, 0.48, 0.66),
            brightness: 220.0,
        })
        // ── Module registry ────────────────────────────────────────────────
        .insert_resource(registry)
        // ── Asset registry (qforge manifests) ─────────────────────────────
        .insert_resource(asset_registry)
        // ── Layer 0: Camera ────────────────────────────────────────────────
        .add_plugins(CameraPlugin)
        // ── Layer I: Manifestation ─────────────────────────────────────────
        .add_plugins(WorldAnchorPlugin)
        .add_plugins(TopologyPlugin)
        .add_plugins(EntityInstantiatorPlugin)
        .add_plugins(ContainerMapperPlugin)
        .add_plugins(AssetRegistryPlugin)
        // ── Layer II: Physics ──────────────────────────────────────────────
        .add_plugins(GravityPlugin)
        .add_plugins(CollisionPlugin)
        .add_plugins(ThermodynamicsPlugin)
        .add_plugins(KinematicsPlugin)
        // ── Layer III: Consciousness ───────────────────────────────────────
        .add_plugins(InstinctPlugin)
        .add_plugins(SocialDrivePlugin)
        .add_plugins(NarrativePlugin)
        .add_plugins(DialoguePlugin)
        // ── Layer IV: Fabrication ──────────────────────────────────────────
        .add_plugins(SignalBroadcasterPlugin::default())
        .add_plugins(FabricationPlugin)
        .add_plugins(ServoPlugin)
        .add_plugins(RealityLockPlugin)
        // ── Mixer (Traktor S4) ─────────────────────────────────────────────
        .add_plugins(TraktorMixerPlugin::default())
        // ── Container loader (.qgenesis files) ────────────────────────────
        .add_plugins(ContainerPlugin)
        // ── Rack UI ────────────────────────────────────────────────────────
        .add_plugins(RackUiPlugin)
        // ── Scene setup ────────────────────────────────────────────────────
        .add_systems(Startup, spawn_first_actor)
        // ── MIDI → terrain height bridge ───────────────────────────────────
        .add_systems(Update, sync_terrain_height)
        .run();
}

// ── First actor spawn ─────────────────────────────────────────────────────────

fn spawn_first_actor(mut writer: EventWriter<SpawnActorRequest>) {
    writer.send(SpawnActorRequest {
        name: "Quill-Alpha".into(),
        genotype: Genotype {
            aggression:    0.1,
            curiosity:     0.9,
            social_drive:  0.6,
            fear_threshold: 0.3,
            energy:        1.0,
        },
        position: Vec3::new(0.0, 1.0, 0.0),
    });
}

// ── MIDI → terrain height scale ───────────────────────────────────────────────
//
// GEN-01 Terrain receives its fader value (CC 7, Ch1) as `height_scale` in the
// RackState param map.  We reflect it onto the terrain entity's Y scale so the
// landscape stretches live while the user moves the fader.

fn sync_terrain_height(
    rack:  Res<RackState>,
    mut q: Query<&mut Transform, With<TerrainEntity>>,
) {
    if !rack.is_changed() {
        return;
    }
    let h = rack.get_param("GEN-01", "height_scale");
    if h > 0.0 {
        if let Ok(mut tf) = q.get_single_mut() {
            tf.scale.y = h;
        }
    }
}
