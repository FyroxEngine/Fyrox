use crate::core::pool::Handle;
use crate::utils::behavior::{BehaviorNode, BehaviorTree};

pub enum CompositeNodeKind {
    Sequence,
    Selector,
}

pub struct CompositeNode<B> {
    pub children: Vec<Handle<BehaviorNode<B>>>,
    pub kind: CompositeNodeKind,
}

impl<B> CompositeNode<B> {
    pub fn new(kind: CompositeNodeKind, children: Vec<Handle<BehaviorNode<B>>>) -> Self {
        Self { children, kind }
    }

    pub fn add(self, tree: &mut BehaviorTree<B>) -> Handle<BehaviorNode<B>> {
        tree.add_node(BehaviorNode::Composite(self))
    }
}
