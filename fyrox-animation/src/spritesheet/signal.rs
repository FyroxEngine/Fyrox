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
