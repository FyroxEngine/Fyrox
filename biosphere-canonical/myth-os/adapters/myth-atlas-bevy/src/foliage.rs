use bevy::prelude::*;
use myth_atlas::types::TerrainChunk;

/// A single placed foliage instance (tree, rock, grass tuft, etc.)
#[derive(Component, Debug, Clone)]
pub struct FoliageInstance {
    pub position: Vec3,
    pub scale:    f32,
    pub variant:  u8,
}

/// Scatter foliage on cells whose moisture exceeds the density threshold.
/// Called by the terrain update path after a TerrainChunk is applied.
pub fn scatter_foliage(
    chunk: &TerrainChunk,
    chunk_world_x: f32,
    chunk_world_z: f32,
    cell_size: f32,
    density_threshold: f32,
) -> Vec<FoliageInstance> {
    let mut instances = Vec::new();
    let width = (chunk.heightmap.len() as f32).sqrt() as usize;

    for z in 0..width {
        for x in 0..width {
            let idx = z * width + x;
            let moisture = chunk.moisture.get(idx).copied().unwrap_or(0.0);
            if moisture > density_threshold {
                let wx = chunk_world_x + x as f32 * cell_size;
                let wz = chunk_world_z + z as f32 * cell_size;
                let wy = chunk.heightmap[idx];
                instances.push(FoliageInstance {
                    position: Vec3::new(wx, wy, wz),
                    scale: 0.8 + moisture * 0.4,
                    variant: (idx % 4) as u8,
                });
            }
        }
    }
    instances
}
