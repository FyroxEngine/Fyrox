use crate::core::pool::Handle;
use crate::utils::behavior::{BehaviorNode, BehaviorTree};
use std::cell::RefCell;

pub struct LeafNode<B> {
    pub behavior: RefCell<B>,
}

impl<B> LeafNode<B> {
    pub fn new(behavior: B) -> Self {
        Self {
            behavior: RefCell::new(behavior),
        }
    }

    pub fn add(self, tree: &mut BehaviorTree<B>) -> Handle<BehaviorNode<B>> {
        tree.add_node(BehaviorNode::Leaf(self))
    }
}
