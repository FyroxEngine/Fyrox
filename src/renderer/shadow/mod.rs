#![warn(clippy::too_many_arguments)]

use crate::core::math::frustum::Frustum;
use crate::scene::node::Node;

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

fn should_cast_shadows(node: &Node, light_frustum: &Frustum) -> bool {
    node.global_visibility() && {
        match node {
            Node::Mesh(mesh) => {
                mesh.cast_shadows() && (!mesh.frustum_culling() || light_frustum.is_intersects_aabb(&mesh.world_bounding_box()))
            }
            Node::Terrain(terrain) => {
                terrain.cast_shadows()
                    && (!terrain.frustum_culling() || light_frustum.is_intersects_aabb(&terrain.world_bounding_box()))
            }
            _ => false,
        }
    }
}
