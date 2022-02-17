//! Visibility cache stores information about objects visibility for a single frame.
//!
//! For more info see [`VisibilityCache`]

use crate::scene::graph::NodePool;
use crate::{
    core::{algebra::Vector3, math::frustum::Frustum, pool::Handle},
    scene::node::Node,
};
use fxhash::FxHashMap;

/// Visibility cache stores information about objects visibility for a single frame. Allows you to quickly check
/// if an object is visible or not.
///
/// # Notes
///
/// Visibility cache stores very coarse information about object visibility, it does not include any kind of occlusion
/// tests of whatsoever. It just a simple frustum test + level-of-detail (LOD) system.
///
/// LODs have priority over other visibility options, if a level is not active, then its every object will be hidden,
/// not matter if the actual visibility state is `visible`.
///
/// # Performance
///
/// The cache is based on hash map, so it is very fast and has O(1) complexity for fetching.
#[derive(Default, Debug, Clone)]
pub struct VisibilityCache {
    map: FxHashMap<Handle<Node>, bool>,
}

impl From<FxHashMap<Handle<Node>, bool>> for VisibilityCache {
    fn from(map: FxHashMap<Handle<Node>, bool>) -> Self {
        Self { map }
    }
}

impl VisibilityCache {
    /// Replaces internal map with empty and returns previous value. This trick is useful
    /// to reuse hash map to prevent redundant memory allocations.
    pub fn invalidate(&mut self) -> FxHashMap<Handle<Node>, bool> {
        std::mem::take(&mut self.map)
    }

    /// Clears the cache.
    pub fn clear(&mut self) {
        self.map.clear()
    }

    /// Updates visibility cache - checks visibility for each node in given graph, also performs
    /// frustum culling if frustum set is specified.
    pub fn update(
        &mut self,
        nodes: &NodePool,
        observer_position: Vector3<f32>,
        z_near: f32,
        z_far: f32,
        frustums: Option<&[&Frustum]>,
    ) {
        self.map.clear();

        // Check LODs first, it has priority over other visibility settings.
        for node in nodes.iter() {
            if let Some(lod_group) = node.lod_group() {
                for level in lod_group.levels.iter() {
                    for &object in level.objects.iter() {
                        if let Some(object_ref) = nodes.try_borrow(*object) {
                            let distance =
                                observer_position.metric_distance(&object_ref.global_position());
                            let z_range = z_far - z_near;
                            let normalized_distance = (distance - z_near) / z_range;
                            let visible = normalized_distance >= level.begin()
                                && normalized_distance <= level.end();
                            self.map.insert(*object, visible);
                        }
                    }
                }
            }
        }

        // Fill rest of data from global visibility flag of nodes and check frustums (if any).
        for (handle, node) in nodes.pair_iter() {
            // We need to fill only unfilled entries, none of visibility flags of a node can
            // make it visible again if lod group hid it.
            self.map.entry(handle).or_insert_with(|| {
                let mut visibility = node.global_visibility();
                if visibility && node.frustum_culling() {
                    // If a node globally visible, check it with each frustum (if any).
                    if let Some(frustums) = frustums {
                        let mut visible_by_any_frustum = false;
                        for frustum in frustums {
                            if frustum.is_intersects_aabb(&node.world_bounding_box()) {
                                visible_by_any_frustum = true;
                                break;
                            }
                        }
                        visibility = visible_by_any_frustum;
                    }
                }
                visibility
            });
        }
    }

    /// Checks whether the node is visible or not.
    ///
    /// # Complexity
    ///
    /// Constant, O(1)
    pub fn is_visible(&self, node: Handle<Node>) -> bool {
        self.map.get(&node).cloned().unwrap_or(false)
    }
}
