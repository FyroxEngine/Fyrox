use crate::{
    core::{reflect::prelude::*, uuid::Uuid, visitor::prelude::*},
    utils::NameProvider,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AnimationEvent {
    pub signal_id: Uuid,
}

#[derive(Clone, Debug, Visit, Reflect, PartialEq)]
pub struct AnimationSignal {
    pub id: Uuid,
    pub name: String,
    pub time: f32,
    pub enabled: bool,
}

impl NameProvider for AnimationSignal {
    fn name(&self) -> &str {
        &self.name
    }
}

impl AnimationSignal {
    pub fn new(id: Uuid, name: &str, time: f32) -> Self {
        Self {
            id,
            name: name.to_owned(),
            time,
            enabled: true,
        }
    }
}

impl Default for AnimationSignal {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: Default::default(),
            time: 0.0,
            enabled: true,
        }
    }
}
