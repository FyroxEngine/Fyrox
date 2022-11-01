//! Animation signal is used as a point at which to notify external observers that animation just
//! started to play a specific frame.

use crate::core::{inspect::prelude::*, reflect::prelude::*, visitor::prelude::*};

/// Animation signal is used as a point at which to notify external observers that animation just
/// started to play a specific frame.
#[derive(Inspect, Visit, Reflect, Debug, Clone)]
pub struct Signal {
    /// Signal id. It should be used to distinguish different signals. For example, `JUMP` signal
    /// can have `id = 0`, while `CROUCH` signal - `id = 1`, etc.
    pub id: u64,

    /// Index of a frame at which to notify external observers.
    pub frame: u32,

    /// Is the signal enabled or not. Disabled signals won't produce any events.
    pub enabled: bool,
}

impl Default for Signal {
    fn default() -> Self {
        Self {
            id: 0,
            frame: 0,
            enabled: true,
        }
    }
}
