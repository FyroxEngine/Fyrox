use crate::{
    animation::{
        container::TrackFramesContainer,
        value::{BoundValue, ValueBinding},
    },
    core::{pool::Handle, visitor::prelude::*},
    scene::{base::InstanceId, node::Node},
};
use std::fmt::Debug;

pub trait TrackTarget: Visit + Debug + Copy + Clone + Default {}

impl TrackTarget for Handle<Node> {}
impl TrackTarget for InstanceId {}

#[derive(Debug, Visit, Clone)]
pub struct Track<T>
where
    T: TrackTarget,
{
    #[visit(optional)] // Backward compatibility
    binding: ValueBinding,
    #[visit(skip)] // TODO: Use a switch to enable/disable frames serialization.
    frames: TrackFramesContainer,
    enabled: bool,
    #[visit(rename = "Node")]
    target: T,
}

impl<T> Default for Track<T>
where
    T: TrackTarget,
{
    fn default() -> Self {
        Self {
            binding: ValueBinding::Position,
            frames: TrackFramesContainer::Vector3(Default::default()),
            enabled: true,
            target: Default::default(),
        }
    }
}

impl<T> Track<T>
where
    T: TrackTarget,
{
    pub fn new(container: TrackFramesContainer) -> Self {
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

    pub fn set_target(&mut self, target: T) {
        self.target = target;
    }

    pub fn target(&self) -> T {
        self.target
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
