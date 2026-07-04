use bevy::prelude::*;
use myth_atlas::types::{AtlasConfig, BiomeType, TerrainChunk};

/// Bevy plugin that drives mesh generation from Atlas wire output.
/// Pure rendering adapter — no world-gen logic lives here.
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainConfig>()
            .add_systems(Startup, setup_terrain_mesh)
            .add_systems(Update, update_terrain_chunks);
    }
}

/// Bevy-side rendering config. Separate from AtlasConfig (pure data).
#[derive(Resource)]
pub struct TerrainConfig {
    /// Mesh subdivisions per axis (default 256 → matches grid_resolution)
    pub resolution: u32,
    /// World-space size of the terrain quad in metres
    pub size: f32,
    /// Vertical scale applied to 0–1 heightmap values
    pub height_scale: f32,
    /// WGSL splatmap shader handle
    pub shader_path: &'static str,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            resolution: 256,
            size: 200.0,
            height_scale: 25.0,
            shader_path: "shaders/atlas-terrain.wgsl",
        }
    }
}

/// Bevy component marking the main terrain entity.
#[derive(Component)]
pub struct TerrainMesh {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

/// Bevy component holding per-cell biome data for the splatmap.
#[derive(Component)]
pub struct BiomeMap {
    pub width: u32,
    pub height: u32,
    /// Flat [width × height] array of BiomeType
    pub cells: Vec<BiomeType>,
}

fn setup_terrain_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<TerrainConfig>,
) {
    let res = config.resolution as usize;
    let step = config.size / config.resolution as f32;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(res * res);
    let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(res * res);
    let mut uvs:       Vec<[f32; 2]> = Vec::with_capacity(res * res);
    let mut indices:   Vec<u32>      = Vec::new();

    // Flat grid — height is patched by `update_terrain_chunks` when chunks arrive.
    for z in 0..res {
        for x in 0..res {
            let px = x as f32 * step - config.size * 0.5;
            let pz = z as f32 * step - config.size * 0.5;
            positions.push([px, 0.0, pz]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / res as f32, z as f32 / res as f32]);
        }
    }

    for z in 0..res - 1 {
        for x in 0..res - 1 {
            let i = (z * res + x) as u32;
            indices.extend_from_slice(&[i, i + 1, i + res as u32 + 1, i, i + res as u32 + 1, i + res as u32]);
        }
    }

    let mut mesh = Mesh::new(bevy::render::mesh::PrimitiveTopology::TriangleList, bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD | bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.2),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::default(),
        TerrainMesh { chunk_x: 0, chunk_z: 0 },
    ));
}

/// Receives TerrainChunk wire packets (arriving as Bevy events) and
/// patches the mesh vertex heights. Production code will use a Bevy
/// EventReader on a custom TerrainChunkEvent wrapping TerrainChunk.
fn update_terrain_chunks(
    _meshes: ResMut<Assets<Mesh>>,
    _terrain: Query<&TerrainMesh>,
    _config: Res<TerrainConfig>,
) {
    // Wire integration: subscribe to myth-wire Spatial packets tagged
    // "myth-atlas" and deserialize as TerrainChunk. Height patch happens here.
    // Full implementation deferred until myth-theater event bridge is wired.
}
