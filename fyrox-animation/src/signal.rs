//! Signal is a named marker on specific time position on the animation timeline. See [`AnimationSignal`] docs for more info.

use crate::core::{reflect::prelude::*, uuid::Uuid, visitor::prelude::*};
use fyrox_core::NameProvider;

/// An event happened in an animation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AnimationEvent {
    /// An id of an animation event.
    pub signal_id: Uuid,

    /// Name of the signal emitted the event.
    pub name: String,
}

/// Signal is a named marker on specific time position on the animation timeline. Signal will emit an event if the animation playback
/// time passes signal's position from left-to-right (or vice versa depending on playback direction). Signals are usually used to
/// attach some specific actions to a position in time. For example, you can have a walking animation and you want to emit sounds
/// when character's feet touch ground. In this case you need to add a few signals at times when each foot touches the ground.
/// After that all you need to do is to fetch animation events one-by-one and emit respective sounds. See [`AnimationSignal`] docs
/// for more info and examples.
#[derive(Clone, Debug, Visit, Reflect, PartialEq)]
pub struct AnimationSignal {
    /// An id of the animation signal. Any event produced by the signal will have this id.
    pub id: Uuid,

    /// Name of the animation signal. Could be used to find the signal in a container of signals.
    pub name: String,

    /// A position (in seconds) on an animation time line.
    pub time: f32,

    /// The flag defines whether the signal is enabled or not. Disabled signals won't produce any events.
    pub enabled: bool,
}

impl NameProvider for AnimationSignal {
    fn name(&self) -> &str {
        &self.name
    }
}

impl AnimationSignal {
    /// Creates a new enabled animation signal with a given id, name and time position.
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
