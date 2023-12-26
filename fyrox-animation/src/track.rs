//! Track is responsible in animating a property of a single scene node. See [`Track`] docs for more info.

use crate::{
    container::{TrackDataContainer, TrackValueKind},
    core::{reflect::prelude::*, uuid::Uuid, visitor::prelude::*},
    value::{BoundValue, ValueBinding},
    EntityId,
};
use std::fmt::Debug;

/// Track is responsible in animating a property of a single scene node. The track consists up to 4 parametric curves
/// that contains the actual property data. Parametric curves allows the engine to perform various interpolations between
/// key values.
#[derive(Debug, Reflect, Clone, PartialEq)]
pub struct Track<T: EntityId> {
    binding: ValueBinding,
    frames: TrackDataContainer,
    enabled: bool,
    target: T,
    id: Uuid,
}

impl<T: EntityId> Visit for Track<T> {
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

impl<T: EntityId> Default for Track<T> {
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

impl<T: EntityId> Track<T> {
    /// Creates a new track that will animate a property in the given binding. The `container` must have enough parametric
    /// curves to be able to produces property values.
    pub fn new(container: TrackDataContainer, binding: ValueBinding) -> Self {
        Self {
            frames: container,
            binding,
            ..Default::default()
        }
    }

    /// Creates a new track that is responsible in animating a position property of a scene node.
    pub fn new_position() -> Self {
        Self {
            frames: TrackDataContainer::new(TrackValueKind::Vector3),
            binding: ValueBinding::Position,
            ..Default::default()
        }
    }

    /// Creates a new track that is responsible in animating a rotation property of a scene node.
    pub fn new_rotation() -> Self {
        Self {
            frames: TrackDataContainer::new(TrackValueKind::UnitQuaternion),
            binding: ValueBinding::Rotation,
            ..Default::default()
        }
    }

    /// Creates a new track that is responsible in animating a scaling property of a scene node.
    pub fn new_scale() -> Self {
        Self {
            frames: TrackDataContainer::new(TrackValueKind::Vector3),
            binding: ValueBinding::Scale,
            ..Default::default()
        }
    }

    /// Sets target of the track.
    pub fn with_target(mut self, target: T) -> Self {
        self.target = target;
        self
    }

    /// Sets new track binding. See [`ValueBinding`] docs for more info.
    pub fn set_binding(&mut self, binding: ValueBinding) {
        self.binding = binding;
    }

    /// Returns current track binding.
    pub fn binding(&self) -> &ValueBinding {
        &self.binding
    }

    /// Sets a handle of a node that will be animated.
    pub fn set_target(&mut self, target: T) {
        self.target = target;
    }

    /// Returns a handle of a node that will be animated.
    pub fn target(&self) -> T {
        self.target
    }

    /// Returns a reference to the data container.
    pub fn data_container(&self) -> &TrackDataContainer {
        &self.frames
    }

    /// Returns a reference to the data container.
    pub fn data_container_mut(&mut self) -> &mut TrackDataContainer {
        &mut self.frames
    }

    /// Sets new data container and returns the previous one.
    pub fn set_data_container(&mut self, container: TrackDataContainer) -> TrackDataContainer {
        std::mem::replace(&mut self.frames, container)
    }

    /// Tries to get a new property value at a given time position.
    pub fn fetch(&self, time: f32) -> Option<BoundValue> {
        self.frames.fetch(time).map(|v| BoundValue {
            binding: self.binding.clone(),
            value: v,
        })
    }

    /// Enables or disables the track. Disabled tracks won't animate their nodes/properties.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns `true` if the track is enabled, `false` - otherwise.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns length of the track in seconds.
    pub fn time_length(&self) -> f32 {
        self.frames.time_length()
    }

    /// Sets a new id for the track.
    pub fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    /// Returns the id of the track.
    pub fn id(&self) -> Uuid {
        self.id
    }
}
