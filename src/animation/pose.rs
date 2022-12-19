use crate::{
    animation::{value::BoundValue, value::BoundValueCollection},
    core::pool::Handle,
    scene::{graph::Graph, graph::NodePool, node::Node},
    utils::log::{Log, MessageKind},
};
use fxhash::FxHashMap;
use std::collections::hash_map::Entry;

#[derive(Clone, Debug, PartialEq)]
pub struct NodePose {
    pub node: Handle<Node>,
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
    fn weighted_clone(&self, weight: f32) -> Self {
        Self {
            node: self.node,
            values: self.values.weighted_clone(weight),
        }
    }

    pub fn blend_with(&mut self, other: &NodePose, weight: f32) {
        self.values.blend_with(&other.values, weight)
    }

    pub fn values(&self) -> &BoundValueCollection {
        &self.values
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct AnimationPose {
    poses: FxHashMap<Handle<Node>, NodePose>,
}

impl AnimationPose {
    pub fn clone_into(&self, dest: &mut AnimationPose) {
        dest.reset();
        for (handle, local_pose) in self.poses.iter() {
            dest.poses.insert(*handle, local_pose.clone());
        }
    }

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
    }

    fn add_node_pose(&mut self, local_pose: NodePose) {
        self.poses.insert(local_pose.node, local_pose);
    }

    pub fn add_to_node_pose(&mut self, node: Handle<Node>, bound_value: BoundValue) {
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

    pub fn reset(&mut self) {
        self.poses.clear();
    }

    pub fn poses(&self) -> &FxHashMap<Handle<Node>, NodePose> {
        &self.poses
    }

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
