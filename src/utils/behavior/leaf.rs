use crate::{
    core::{pool::Handle, visitor::prelude::*},
    utils::behavior::{BehaviorNode, BehaviorTree},
};
use std::cell::RefCell;

#[derive(Debug, PartialEq, Visit)]
pub struct LeafNode<B> {
    pub behavior: Option<RefCell<B>>,
}

impl<B> Default for LeafNode<B> {
    fn default() -> Self {
        Self { behavior: None }
    }
}

impl<B> LeafNode<B> {
    pub fn new(behavior: B) -> Self {
        Self {
            behavior: Some(RefCell::new(behavior)),
        }
    }

    pub fn add(self, tree: &mut BehaviorTree<B>) -> Handle<BehaviorNode<B>> {
        tree.add_node(BehaviorNode::Leaf(self))
    }
}
