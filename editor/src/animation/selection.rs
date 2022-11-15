use fyrox::animation::Animation;
use fyrox::{
    core::{pool::Handle, uuid::Uuid},
    scene::node::Node,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SelectedEntity {
    Track(Uuid),
    Curve(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationSelection {
    pub animation_player: Handle<Node>,
    pub animation: Handle<Animation>,
    pub entities: Vec<SelectedEntity>,
}

impl AnimationSelection {
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }
}
