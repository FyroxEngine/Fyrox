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

use crate::fyrox::core::reflect::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::Range;

fn default_zoom_speed() -> f32 {
    0.5
}

fn default_zoom_range() -> Range<f32> {
    0.0f32..100.0f32
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Reflect)]
pub struct CameraSettings {
    #[serde(default)]
    pub speed: f32,
    #[serde(default = "default_zoom_speed")]
    pub zoom_speed: f32,
    #[reflect(min_value = 0.0, max_value = 1000.0)]
    #[serde(default = "default_zoom_range")]
    pub zoom_range: Range<f32>,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            speed: 10.0,
            zoom_speed: default_zoom_speed(),
            zoom_range: default_zoom_range(),
        }
    }
}
