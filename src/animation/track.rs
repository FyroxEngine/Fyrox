use crate::{
    animation::{
        container::{TrackFramesContainer, TrackValueKind},
        value::{BoundValue, ValueBinding},
    },
    core::{pool::Handle, reflect::prelude::*, uuid::Uuid, visitor::prelude::*},
    scene::{base::InstanceId, node::Node},
};
use std::fmt::Debug;

pub trait TrackTarget: Visit + Debug + Copy + Clone + Default + Reflect {}

impl TrackTarget for Handle<Node> {}
impl TrackTarget for InstanceId {}

#[derive(Debug, Reflect, Clone, PartialEq)]
pub struct Track<T>
where
    T: TrackTarget,
{
    binding: ValueBinding,
    frames: TrackFramesContainer,
    enabled: bool,
    serialize_frames: bool,
    target: T,
    id: Uuid,
}

impl<T> Visit for Track<T>
where
    T: TrackTarget,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.target.visit("Node", &mut region)?;
        self.enabled.visit("Enabled", &mut region)?;

        let _ = self.binding.visit("Binding", &mut region); // Backward compatibility
        let _ = self.serialize_frames.visit("SerializeFrames", &mut region); // Backward compatibility
        let _ = self.id.visit("Id", &mut region); // Backward compatibility

        if self.serialize_frames {
            self.frames.visit("Frames", &mut region)?;
        }

        Ok(())
    }
}

impl<T> Default for Track<T>
where
    T: TrackTarget,
{
    fn default() -> Self {
        Self {
            binding: ValueBinding::Position,
            frames: TrackFramesContainer::default(),
            enabled: true,
            // Keep existing logic: animation instances do not save their frames on serialization,
            // instead they're restoring it from respective animation resource.
            serialize_frames: false,
            target: Default::default(),
            id: Uuid::new_v4(),
        }
    }
}

impl<T> Track<T>
where
    T: TrackTarget,
{
    pub fn new(container: TrackFramesContainer, binding: ValueBinding) -> Self {
        Self {
            frames: container,
            binding,
            ..Default::default()
        }
    }

    pub fn new_position() -> Self {
        Self {
            frames: TrackFramesContainer::new(TrackValueKind::Vector3),
            binding: ValueBinding::Position,
            ..Default::default()
        }
    }

    pub fn new_rotation() -> Self {
        Self {
            frames: TrackFramesContainer::new(TrackValueKind::UnitQuaternion),
            binding: ValueBinding::Rotation,
            ..Default::default()
        }
    }

    pub fn new_scale() -> Self {
        Self {
            frames: TrackFramesContainer::new(TrackValueKind::Vector3),
            binding: ValueBinding::Scale,
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

    pub fn set_serialize_frames(&mut self, state: bool) {
        self.serialize_frames = state;
    }

    pub fn is_serializing_frames(&self) -> bool {
        self.serialize_frames
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}
