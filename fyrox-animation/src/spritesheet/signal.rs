//! Animation signal is used as a point at which to notify external observers that animation just
//! started to play a specific frame.

use crate::core::{reflect::prelude::*, visitor::prelude::*};
use fyrox_core::uuid_provider;

/// Animation signal is used as a point at which to notify external observers that animation just
/// started to play a specific frame.
#[derive(PartialEq, Visit, Reflect, Debug, Clone)]
pub struct Signal {
    /// Signal id. It should be used to distinguish different signals. For example, `JUMP` signal
    /// can have `id = 0`, while `CROUCH` signal - `id = 1`, etc.
    pub id: u64,

    /// Index of a frame at which to notify external observers.
    pub frame: u32,

    /// Is the signal enabled or not. Disabled signals won't produce any events.
    pub enabled: bool,
}

uuid_provider!(Signal = "30fd963f-4ce7-4dcc-bdff-691897267420");

impl Default for Signal {
    fn default() -> Self {
        Self {
            id: 0,
            frame: 0,
            enabled: true,
        }
    }
}
