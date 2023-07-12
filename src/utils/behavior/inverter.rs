//! A node, that inverts its child state ([`super::Status::Failure`] becomes [`super::Status::Success`] and vice versa, [`super::Status::Running`] remains
//! unchanged)

use crate::{
    core::{pool::Handle, visitor::prelude::*},
    utils::behavior::{BehaviorNode, BehaviorTree},
};

/// See module docs.
#[derive(Debug, PartialEq, Visit, Eq, Clone)]
pub struct Inverter<B>
where
    B: Clone,
{
    /// A handle of child node, the state of which will be inverted.
    pub child: Handle<BehaviorNode<B>>,
}

impl<B> Default for Inverter<B>
where
    B: Clone,
{
    fn default() -> Self {
        Self {
            child: Default::default(),
        }
    }
}

impl<B> Inverter<B>
where
    B: Clone + 'static,
{
    /// Creates new inverter node with given action.
    pub fn new(child: Handle<BehaviorNode<B>>) -> Self {
        Self { child }
    }

    /// Adds self to given behavior tree and returns handle to self.
    pub fn add_to(self, tree: &mut BehaviorTree<B>) -> Handle<BehaviorNode<B>> {
        tree.add_node(BehaviorNode::Inverter(self))
    }
}
