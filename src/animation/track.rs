use crate::{
    animation::{
        container::TrackFramesContainer,
        value::{BoundValue, ValueBinding},
    },
    core::{pool::Handle, visitor::prelude::*},
    scene::node::Node,
};

#[derive(Debug, Visit, Clone)]
pub struct Track {
    #[visit(optional)] // Backward compatibility
    binding: ValueBinding,
    #[visit(skip)] // TODO: Use a switch to enable/disable frames serialization.
    frames: TrackFramesContainer,
    enabled: bool,
    node: Handle<Node>,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            binding: ValueBinding::Position,
            frames: TrackFramesContainer::Vector3(Default::default()),
            enabled: true,
            node: Default::default(),
        }
    }
}

impl Track {
    pub fn new(container: TrackFramesContainer) -> Track {
        Self {
            frames: container,
            ..Default::default()
        }
    }

    pub fn set_binding(&mut self, binding: ValueBinding) {
        self.binding = binding;
    }

    pub fn binding(&self) -> &ValueBinding {
        &self.binding
    }

    pub fn set_node(&mut self, node: Handle<Node>) {
        self.node = node;
    }

    pub fn node(&self) -> Handle<Node> {
        self.node
    }

    pub fn frames_container(&self) -> &TrackFramesContainer {
        &self.frames
    }

    pub fn frames_container_mut(&mut self) -> &mut TrackFramesContainer {
        &mut self.frames
    }

    pub fn set_frames_container(
        &mut self,
        container: TrackFramesContainer,
    ) -> TrackFramesContainer {
        std::mem::replace(&mut self.frames, container)
    }

    pub fn fetch(&self, time: f32) -> Option<BoundValue> {
        self.frames.fetch(time).map(|v| BoundValue {
            binding: self.binding.clone(),
            value: v,
        })
    }

    pub fn enable(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn time_length(&self) -> f32 {
        self.frames.time_length()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
