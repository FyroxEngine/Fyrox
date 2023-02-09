//! Pose is a set of property values of a node ([`NodePose`]) or a set of nodes ([`AnimationPose`]).

use crate::{
    animation::{value::BoundValue, value::BoundValueCollection, RootMotion},
    core::pool::Handle,
    scene::{graph::Graph, graph::NodePool, node::Node},
    utils::log::{Log, MessageKind},
};
use fxhash::FxHashMap;
use std::collections::hash_map::Entry;

/// A "captured" state of properties of some animated scene node. The pose can be considered as container of values of some
/// properties.
#[derive(Clone, Debug, PartialEq)]
pub struct NodePose {
    /// A handle of an animated node.
    pub node: Handle<Node>,

    /// A set of property values.
    pub values: BoundValueCollection,
}

impl Default for NodePose {
    fn default() -> Self {
        Self {
            node: Handle::NONE,
            values: Default::default(),
        }
    }
}

impl NodePose {
    /// Performs a weighted cloning of the pose. See [`super::value::TrackValue::weighted_clone`] docs for more info.
    fn weighted_clone(&self, weight: f32) -> Self {
        Self {
            node: self.node,
            values: self.values.weighted_clone(weight),
        }
    }

    /// Performs a blending of the current with some other pose. See [`super::value::TrackValue::blend_with`] docs for more
    /// info.
    pub fn blend_with(&mut self, other: &NodePose, weight: f32) {
        self.values.blend_with(&other.values, weight)
    }
}

/// Animations pose is a set of node poses. See [`NodePose`] docs for more info.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct AnimationPose {
    poses: FxHashMap<Handle<Node>, NodePose>,
    root_motion: Option<RootMotion>,
}

impl AnimationPose {
    /// Clears the set of node poses in the given animation pose and clones poses from the current animation pose to the given.
    pub fn clone_into(&self, dest: &mut AnimationPose) {
        dest.reset();
        for (handle, local_pose) in self.poses.iter() {
            dest.poses.insert(*handle, local_pose.clone());
        }
        dest.root_motion = self.root_motion.clone();
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
    /// will become a weighted copies of a respective node pose.
    pub fn blend_with(&mut self, other: &AnimationPose, weight: f32) {
        for (handle, other_pose) in other.poses.iter() {
            if let Some(current_pose) = self.poses.get_mut(handle) {
                current_pose.blend_with(other_pose, weight);
            } else {
                // There are no corresponding local pose, do fake blend between identity
                // pose and other.
                self.add_node_pose(other_pose.weighted_clone(weight));
            }
        }

        self.root_motion
            .get_or_insert_with(Default::default)
            .blend_with(&other.root_motion.clone().unwrap_or_default(), weight);
    }

    fn add_node_pose(&mut self, local_pose: NodePose) {
        self.poses.insert(local_pose.node, local_pose);
    }

    pub(super) fn add_to_node_pose(&mut self, node: Handle<Node>, bound_value: BoundValue) {
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
    pub fn poses(&self) -> &FxHashMap<Handle<Node>, NodePose> {
        &self.poses
    }

    /// Returns a reference to inner node pose map.
    pub fn poses_mut(&mut self) -> &mut FxHashMap<Handle<Node>, NodePose> {
        &mut self.poses
    }

    pub(crate) fn apply_internal(&self, nodes: &mut NodePool) {
        for (node, local_pose) in self.poses.iter() {
            if node.is_none() {
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargeting failed!");
            } else if let Some(node) = nodes.try_borrow_mut(*node) {
                local_pose.values.apply(node);
            }
        }
    }

    /// Tries to set each value to the each property from the animation pose to respective scene nodes.
    pub fn apply(&self, graph: &mut Graph) {
        for (node, local_pose) in self.poses.iter() {
            if node.is_none() {
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargeting failed!");
            } else if let Some(node) = graph.try_get_mut(*node) {
                local_pose.values.apply(node);
            }
        }
    }

    /// Calls given callback function for each node and allows you to apply pose with your own
    /// rules. This could be useful if you need to ignore transform some part of pose for a node.
    pub fn apply_with<C>(&self, graph: &mut Graph, mut callback: C)
    where
        C: FnMut(&mut Node, Handle<Node>, &NodePose),
    {
        for (node, local_pose) in self.poses.iter() {
            if node.is_none() {
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargeting failed!");
            } else if let Some(node_ref) = graph.try_get_mut(*node) {
                callback(node_ref, *node, local_pose);
            }
        }
    }
}
