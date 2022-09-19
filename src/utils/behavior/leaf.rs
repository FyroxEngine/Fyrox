//! Leaf is a "final" node of a behavior tree. It contains user-defined action which
//! is able to mutate given context.

use crate::{
    core::{pool::Handle, visitor::prelude::*},
    utils::behavior::{BehaviorNode, BehaviorTree},
};
use std::cell::RefCell;

/// See module docs.
#[derive(Debug, PartialEq, Visit, Eq, Clone)]
pub struct LeafNode<B>
where
    B: Clone,
{
    /// User-defined behavior.
    pub behavior: Option<RefCell<B>>,
}

impl<B> Default for LeafNode<B>
where
    B: Clone,
{
    fn default() -> Self {
        Self { behavior: None }
    }
}

impl<B> LeafNode<B>
where
    B: Clone + 'static,
{
    /// Creates new leaf node with given action.
    pub fn new(behavior: B) -> Self {
        Self {
            behavior: Some(RefCell::new(behavior)),
        }
    }

    /// Adds self to given behavior tree and returns handle to self.
    pub fn add_to(self, tree: &mut BehaviorTree<B>) -> Handle<BehaviorNode<B>> {
        tree.add_node(BehaviorNode::Leaf(self))
    }
}
