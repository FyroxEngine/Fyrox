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

use crate::fyrox::core::{reflect::prelude::*, visitor::prelude::*};
use fyrox::scene::camera::Exposure;
use std::ops::Range;

#[derive(Visit, PartialEq, Clone, Debug, Reflect)]
#[reflect(type_uuid = "f468bb97-6017-4b43-b369-94374ecc4239")]
pub struct CameraSettings {
    pub speed: f32,
    pub sensitivity: f32,
    pub zoom_speed: f32,
    #[reflect(min_value = 0.0, max_value = 1000.0)]
    pub zoom_range: Range<f32>,
    pub exposure: Exposure,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            speed: 10.0,
            sensitivity: 1.0,
            zoom_speed: 0.5,
            zoom_range: 0.0..100.0,
            exposure: Default::default(),
        }
    }
}
