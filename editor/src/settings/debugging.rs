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

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Reflect)]
pub struct DebuggingSettings {
    pub show_physics: bool,
    pub show_bounds: bool,
    pub show_tbn: bool,
    #[serde(default)]
    pub show_terrains: bool,
    #[serde(default)]
    pub show_light_bounds: bool,
    #[serde(default)]
    pub show_camera_bounds: bool,
    #[reflect(description = "Size of pictograms in meters. It is used for objects like lights.")]
    #[serde(default)]
    pub pictogram_size: f32,
    #[reflect(
        description = "Forces the editor to save the scene in text form as well as standard binary."
    )]
    #[serde(default)]
    pub save_scene_in_text_form: bool,
}

impl Default for DebuggingSettings {
    fn default() -> Self {
        Self {
            show_physics: true,
            show_bounds: true,
            show_tbn: false,
            show_terrains: false,
            show_light_bounds: true,
            show_camera_bounds: true,
            pictogram_size: 0.33,
            save_scene_in_text_form: false,
        }
    }
}
