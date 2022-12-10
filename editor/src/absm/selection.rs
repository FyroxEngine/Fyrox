use fyrox::animation::machine::{PoseNode, State, Transition};
use fyrox::core::pool::Handle;
use fyrox::scene::node::Node;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum SelectedEntity {
    Transition(Handle<Transition>),
    State(Handle<State>),
    PoseNode(Handle<PoseNode>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsmSelection {
    pub absm_node_handle: Handle<Node>,
    pub layer: usize,
    pub entities: Vec<SelectedEntity>,
}

impl AbsmSelection {
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }
}
