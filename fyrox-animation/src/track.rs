// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Track is responsible in animating a property of a single scene node. See [`Track`] docs for more info.

use crate::{
    container::{TrackDataContainer, TrackValueKind},
    core::{reflect::prelude::*, uuid::Uuid, visitor::prelude::*},
    value::{BoundValue, ValueBinding},
    EntityId,
};
use std::fmt::Debug;

/// Track binding contains a handle to a target object that will be animated by an animation track.
/// Additionally, the binding could be disabled to temporarily prevent animation from affecting the
/// target.
#[derive(Debug, Visit, Reflect, Clone, PartialEq)]
pub struct TrackBinding<T: EntityId> {
    /// The binding could be disabled to temporarily prevent animation from affecting the target.
    pub enabled: bool,
    /// A target bound to a track. The actual track id is stored as a key in hash map of bindings in
    /// the animation.
    pub target: T,
}

impl<T: EntityId> Default for TrackBinding<T> {
    fn default() -> Self {
        Self {
            enabled: true,
            target: Default::default(),
        }
    }
}

impl<T: EntityId> TrackBinding<T> {
    /// Creates a new enabled track binding.
    pub fn new(target: T) -> Self {
        Self {
            enabled: true,
            target,
        }
    }

    /// Sets a handle of a node that will be animated.
    pub fn set_target(&mut self, target: T) {
        self.target = target;
    }

    /// Returns a handle of a node that will be animated.
    pub fn target(&self) -> T {
        self.target
    }

    /// Enables or disables the track. Disabled tracks won't animate their nodes/properties.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns `true` if the track is enabled, `false` - otherwise.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Sets target of the track.
    pub fn with_target(mut self, target: T) -> Self {
        self.target = target;
        self
    }
}

/// Track is responsible in animating a property of a single scene node. The track consists up to 4 parametric curves
/// that contains the actual property data. Parametric curves allows the engine to perform various interpolations between
/// key values.
#[derive(Debug, Reflect, Clone, PartialEq)]
pub struct Track {
    pub(super) binding: ValueBinding,
    pub(super) frames: TrackDataContainer,
    pub(super) id: Uuid,
}

impl Visit for Track {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

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
            id: Uuid::new_v4(),
        }
    }
}

impl Track {
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

    /// Sets new track binding. See [`ValueBinding`] docs for more info.
    pub fn set_value_binding(&mut self, binding: ValueBinding) {
        self.binding = binding;
    }

    /// Returns current track binding.
    pub fn value_binding(&self) -> &ValueBinding {
        &self.binding
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
