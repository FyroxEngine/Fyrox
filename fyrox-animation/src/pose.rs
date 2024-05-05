//! Pose is a set of property values of a node ([`NodePose`]) or a set of nodes ([`AnimationPose`]).

use crate::{value::BoundValue, value::BoundValueCollection, EntityId, RootMotion};
use fxhash::FxHashMap;
use std::collections::hash_map::Entry;

/// A "captured" state of properties of some animated scene node. The pose can be considered as container of values of some
/// properties.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct NodePose<T: EntityId> {
    /// A handle of an animated node.
    pub node: T,

    /// A set of property values.
    pub values: BoundValueCollection,
}

impl<T: EntityId> NodePose<T> {
    /// Performs a blending of the current with some other pose. See [`super::value::TrackValue::blend_with`] docs for more
    /// info.
    pub fn blend_with(&mut self, other: &NodePose<T>, weight: f32) {
        self.values.blend_with(&other.values, weight)
    }
}

/// Animations pose is a set of node poses. See [`NodePose`] docs for more info.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct AnimationPose<T: EntityId> {
    poses: FxHashMap<T, NodePose<T>>,
    root_motion: Option<RootMotion>,
}

impl<T: EntityId> AnimationPose<T> {
    /// Clears the set of node poses in the given animation pose and clones poses from the current animation pose to the given.
    pub fn clone_into(&self, dest: &mut AnimationPose<T>) {
        dest.reset();
        for (handle, local_pose) in self.poses.iter() {
            dest.poses.insert(*handle, local_pose.clone());
        }
        dest.root_motion.clone_from(&self.root_motion);
    }

    /// Sets root motion for the animation pose; the root motion will be blended with other motions
    /// and the result can be obtained on a final pose.
    pub fn set_root_motion(&mut self, root_motion: Option<RootMotion>) {
        self.root_motion = root_motion;
    }

    /// Returns current root motion (if any).
    pub fn root_motion(&self) -> Option<&RootMotion> {
        self.root_motion.as_ref()
    }

    /// Blends current animation pose with another using a weight coefficient. Missing node poses (from either animation poses)
    /// will become a simple copies of a respective node pose.
    pub fn blend_with(&mut self, other: &AnimationPose<T>, weight: f32) {
        for (handle, other_pose) in other.poses.iter() {
            if let Some(current_pose) = self.poses.get_mut(handle) {
                current_pose.blend_with(other_pose, weight);
            } else {
                self.add_node_pose(other_pose.clone());
            }
        }

        self.root_motion
            .get_or_insert_with(Default::default)
            .blend_with(&other.root_motion.clone().unwrap_or_default(), weight);
    }

    fn add_node_pose(&mut self, local_pose: NodePose<T>) {
        self.poses.insert(local_pose.node, local_pose);
    }

    pub(super) fn add_to_node_pose(&mut self, node: T, bound_value: BoundValue) {
        match self.poses.entry(node) {
            Entry::Occupied(entry) => {
                entry.into_mut().values.values.push(bound_value);
            }
            Entry::Vacant(entry) => {
                entry.insert(NodePose {
                    node,
                    values: BoundValueCollection {
                        values: vec![bound_value],
                    },
                });
            }
        }
    }

    /// Clears the pose.
    pub fn reset(&mut self) {
        self.poses.clear();
    }

    /// Returns a reference to inner node pose map.
    pub fn poses(&self) -> &FxHashMap<T, NodePose<T>> {
        &self.poses
    }

    /// Returns a reference to inner node pose map.
    pub fn poses_mut(&mut self) -> &mut FxHashMap<T, NodePose<T>> {
        &mut self.poses
    }
}
