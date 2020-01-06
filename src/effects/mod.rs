//! Effects module
//!
//! # Overview
//!
//! Provides unified way of creating and using effects.

use crate::effects::reverb::Reverb;

pub mod reverb;

/// See module docs.
pub enum Effect {
    /// Reberberation effect. See corresponding module for more info.
    Reverb(Reverb)
}

impl Effect {
    pub(in crate) fn feed(&mut self, left: f32, right: f32) -> (f32, f32) {
        match self {
            Effect::Reverb(reverb) => reverb.feed(left, right),
        }
    }
}