use crate::{
    core::{pool::Handle, visitor::prelude::*},
    utils::behavior::{BehaviorNode, BehaviorTree},
};

#[derive(Debug, PartialEq, Visit)]
pub enum CompositeNodeKind {
    Sequence,
    Selector,
}

impl Default for CompositeNodeKind {
    fn default() -> Self {
        Self::Sequence
    }
}

#[derive(Debug, PartialEq, Visit)]
pub struct CompositeNode<B> {
    pub children: Vec<Handle<BehaviorNode<B>>>,
    pub kind: CompositeNodeKind,
}

impl<B> Default for CompositeNode<B> {
    fn default() -> Self {
        Self {
            children: Default::default(),
            kind: Default::default(),
        }
    }
}

impl<B> CompositeNode<B> {
    pub fn new(kind: CompositeNodeKind, children: Vec<Handle<BehaviorNode<B>>>) -> Self {
        Self { children, kind }
    }

    pub fn add(self, tree: &mut BehaviorTree<B>) -> Handle<BehaviorNode<B>> {
        tree.add_node(BehaviorNode::Composite(self))
    }
}
