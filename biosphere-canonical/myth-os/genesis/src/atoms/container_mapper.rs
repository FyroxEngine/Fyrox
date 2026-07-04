// GEN-ATOM-04: Container Mapper
//
// Reads the placed tile results from AssetRegistry::RoomChain and enforces
// socket compatibility between adjacent tiles in the scene.
//
// Original Phase 4: socket-based adjacency validation, room boundary warnings.

use bevy::prelude::*;
use tracing::warn;

use crate::asset_registry::{AssetEntry, AssetRegistry, PlacedTile, TileDir, tiles_compatible};

pub struct ContainerMapperPlugin;

impl Plugin for ContainerMapperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, validate_room_adjacency);
    }
}

// ── Adjacency validation ──────────────────────────────────────────────────────

/// After startup, scan all placed tiles and report any socket mismatches.
/// This is a dev diagnostic — mismatches don't crash the world.
fn validate_room_adjacency(
    tiles:    Query<(&PlacedTile, &Name)>,
    registry: Res<AssetRegistry>,
) {
    if registry.entries.is_empty() { return; }

    // Build a grid lookup
    let grid: std::collections::HashMap<(i32, i32), (&PlacedTile, &Name)> =
        tiles.iter().map(|(t, n)| ((t.grid_x, t.grid_z), (t, n))).collect();

    let dirs = [TileDir::North, TileDir::South, TileDir::East, TileDir::West];

    for ((gx, gz), (tile, name)) in &grid {
        let Some(entry_a) = registry.entries.get(tile.entry_index) else { continue; };

        for dir in &dirs {
            let neighbor_pos = dir.offset((*gx, *gz));
            let Some((neighbor_tile, neighbor_name)) = grid.get(&neighbor_pos) else { continue };
            let Some(entry_b) = registry.entries.get(neighbor_tile.entry_index) else { continue };

            if !tiles_compatible(&entry_a.manifest, &entry_b.manifest, *dir) {
                warn!(
                    tile_a    = %name,
                    tile_b    = %neighbor_name,
                    direction = ?dir,
                    socket_a  = ?socket_for(&entry_a, dir),
                    socket_b  = ?socket_for(&entry_b, &dir.opposite()),
                    "Socket mismatch between adjacent tiles"
                );
            }
        }
    }
}

fn socket_for(entry: &AssetEntry, dir: &TileDir) -> Option<String> {
    let s = entry.manifest.sockets.as_ref()?;
    match dir {
        TileDir::North => s.north.clone(),
        TileDir::South => s.south.clone(),
        TileDir::East  => s.east.clone(),
        TileDir::West  => s.west.clone(),
    }
}
