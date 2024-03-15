use crate::fyrox::{
    core::{pool::Handle, uuid::Uuid},
    generic_animation::Animation,
};
use crate::scene::SelectionContainer;
use std::fmt::{Debug, Formatter};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SelectedEntity {
    Track(Uuid),
    Curve(Uuid),
    Signal(Uuid),
}

#[derive(Eq)]
pub struct AnimationSelection<N>
where
    N: 'static,
{
    pub animation_player: Handle<N>,
    pub animation: Handle<Animation<Handle<N>>>,
    pub entities: Vec<SelectedEntity>,
}

impl<N> Debug for AnimationSelection<N>
where
    N: 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {:?}",
            self.animation_player, self.animation, self.entities
        )
    }
}

impl<N> Clone for AnimationSelection<N>
where
    N: 'static,
{
    fn clone(&self) -> Self {
        Self {
            animation_player: self.animation_player,
            animation: self.animation,
            entities: self.entities.clone(),
        }
    }
}

impl<N> PartialEq for AnimationSelection<N>
where
    N: 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.entities == other.entities
            && self.animation == other.animation
            && self.animation_player == other.animation_player
    }
}

impl<N> SelectionContainer for AnimationSelection<N>
where
    N: 'static,
{
    fn len(&self) -> usize {
        self.entities.len()
    }
}

impl<N> AnimationSelection<N>
where
    N: 'static,
{
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
