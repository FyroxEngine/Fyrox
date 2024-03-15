use crate::fyrox::{
    core::pool::Handle,
    generic_animation::machine::{PoseNode, State, Transition},
};
use crate::scene::SelectionContainer;
use std::fmt::{Debug, Formatter};

#[derive(Eq)]
pub enum SelectedEntity<N: 'static> {
    Transition(Handle<Transition<Handle<N>>>),
    State(Handle<State<Handle<N>>>),
    PoseNode(Handle<PoseNode<Handle<N>>>),
}

impl<N> Debug for SelectedEntity<N>
where
    N: 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transition(v) => write!(f, "{v}"),
            Self::State(v) => write!(f, "{v}"),
            Self::PoseNode(v) => write!(f, "{v}"),
        }
    }
}

impl<N> Clone for SelectedEntity<N>
where
    N: 'static,
{
    fn clone(&self) -> Self {
        match self {
            Self::Transition(v) => Self::Transition(*v),
            Self::State(v) => Self::State(*v),
            Self::PoseNode(v) => Self::PoseNode(*v),
        }
    }
}

impl<N> PartialEq for SelectedEntity<N>
where
    N: 'static,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Transition(a), Self::Transition(b)) => *a == *b,
            (Self::State(a), Self::State(b)) => *a == *b,
            (Self::PoseNode(a), Self::PoseNode(b)) => *a == *b,
            _ => false,
        }
    }
}

#[derive(Eq, Default)]
pub struct AbsmSelection<N: 'static> {
    pub absm_node_handle: Handle<N>,
    pub layer: Option<usize>,
    pub entities: Vec<SelectedEntity<N>>,
}

impl<N> Debug for AbsmSelection<N>
where
    N: 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {:?} {:?}",
            self.absm_node_handle, self.layer, self.entities
        )
    }
}

impl<N> Clone for AbsmSelection<N>
where
    N: 'static,
{
    fn clone(&self) -> Self {
        Self {
            absm_node_handle: self.absm_node_handle,
            layer: self.layer,
            entities: self.entities.clone(),
        }
    }
}

impl<N> PartialEq for AbsmSelection<N>
where
    N: 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.entities == other.entities
            && self.layer == other.layer
            && self.absm_node_handle == other.absm_node_handle
    }
}

impl<N: 'static> SelectionContainer for AbsmSelection<N> {
    fn len(&self) -> usize {
        self.entities.len()
    }
}
