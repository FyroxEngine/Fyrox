use crate::{
    animation::{
        container::{TrackDataContainer, TrackValueKind},
        value::{BoundValue, ValueBinding},
    },
    core::{pool::Handle, reflect::prelude::*, uuid::Uuid, visitor::prelude::*},
    scene::node::Node,
};
use std::fmt::Debug;

#[derive(Debug, Reflect, Clone, PartialEq)]
pub struct Track {
    binding: ValueBinding,
    frames: TrackDataContainer,
    enabled: bool,
    target: Handle<Node>,
    id: Uuid,
}

impl Visit for Track {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.target.visit("Node", &mut region)?;
        self.enabled.visit("Enabled", &mut region)?;

        let _ = self.binding.visit("Binding", &mut region); // Backward compatibility
        let _ = self.id.visit("Id", &mut region); // Backward compatibility
        let _ = self.frames.visit("Frames", &mut region); // Backward compatibility

        Ok(())
    }
}

impl Default for Track {
    fn default() -> Self {
        Self {
            binding: ValueBinding::Position,
            frames: TrackDataContainer::default(),
            enabled: true,
            target: Default::default(),
            id: Uuid::new_v4(),
        }
    }
}

impl Track {
    pub fn new(container: TrackDataContainer, binding: ValueBinding) -> Self {
        Self {
            frames: container,
            binding,
            ..Default::default()
        }
    }

    pub fn new_position() -> Self {
        Self {
            frames: TrackDataContainer::new(TrackValueKind::Vector3),
            binding: ValueBinding::Position,
            ..Default::default()
        }
    }

    pub fn new_rotation() -> Self {
        Self {
            frames: TrackDataContainer::new(TrackValueKind::UnitQuaternion),
            binding: ValueBinding::Rotation,
            ..Default::default()
        }
    }

    pub fn new_scale() -> Self {
        Self {
            frames: TrackDataContainer::new(TrackValueKind::Vector3),
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

    pub fn set_target(&mut self, target: Handle<Node>) {
        self.target = target;
    }

    pub fn target(&self) -> Handle<Node> {
        self.target
    }

    pub fn frames_container(&self) -> &TrackDataContainer {
        &self.frames
    }

    pub fn frames_container_mut(&mut self) -> &mut TrackDataContainer {
        &mut self.frames
    }

    pub fn set_frames_container(&mut self, container: TrackDataContainer) -> TrackDataContainer {
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

    pub fn id(&self) -> Uuid {
        self.id
    }
}
