#![warn(clippy::too_many_arguments)]

use crate::core::math::frustum::Frustum;
use crate::renderer::batch::{SurfaceInstance, SurfaceInstanceFlags};
use crate::scene::mesh::Mesh;
use crate::scene::node::Node;
use crate::scene::terrain::Terrain;

pub mod csm;
pub mod point;
pub mod spot;

pub fn cascade_size(base_size: usize, cascade: usize) -> usize {
    match cascade {
        0 => base_size,
        1 => (base_size / 2).max(1),
        2 => (base_size / 4).max(1),
        _ => unreachable!(),
    }
}

fn should_cast_shadows(surface_instance: &SurfaceInstance, light_frustum: &Frustum) -> bool {
    surface_instance
        .flags
        .contains(SurfaceInstanceFlags::IS_VISIBLE)
        && surface_instance
            .flags
            .contains(SurfaceInstanceFlags::CAST_SHADOWS)
        && (!surface_instance
            .flags
            .contains(SurfaceInstanceFlags::FRUSTUM_CULLING)
            || light_frustum.is_intersects_aabb(&surface_instance.world_aabb))
}
