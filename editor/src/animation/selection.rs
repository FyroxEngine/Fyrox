use crate::scene::SelectionContainer;
use fyrox::{
    core::{pool::Handle, uuid::Uuid},
    scene::{animation::prelude::*, node::Node},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SelectedEntity {
    Track(Uuid),
    Curve(Uuid),
    Signal(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationSelection {
    pub animation_player: Handle<Node>,
    pub animation: Handle<Animation>,
    pub entities: Vec<SelectedEntity>,
}

impl SelectionContainer for AnimationSelection {
    fn len(&self) -> usize {
        self.entities.len()
    }
}

impl AnimationSelection {
    pub fn first_selected_track(&self) -> Option<Uuid> {
        self.entities.iter().find_map(|e| {
            if let SelectedEntity::Track(id) = e {
                Some(*id)
            } else {
                None
            }
        })
    }
}
