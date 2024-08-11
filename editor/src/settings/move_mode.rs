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

use crate::fyrox::core::{algebra::Vector3, math, reflect::prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Reflect)]
pub struct MoveInteractionModeSettings {
    pub grid_snapping: bool,
    pub x_snap_step: f32,
    pub y_snap_step: f32,
    pub z_snap_step: f32,
}

impl Default for MoveInteractionModeSettings {
    fn default() -> Self {
        Self {
            grid_snapping: false,
            x_snap_step: 0.05,
            y_snap_step: 0.05,
            z_snap_step: 0.05,
        }
    }
}

impl MoveInteractionModeSettings {
    pub fn try_snap_vector_to_grid(&self, v: Vector3<f32>) -> Vector3<f32> {
        if self.grid_snapping {
            Vector3::new(
                math::round_to_step(v.x, self.x_snap_step),
                math::round_to_step(v.y, self.y_snap_step),
                math::round_to_step(v.z, self.z_snap_step),
            )
        } else {
            v
        }
    }
}
