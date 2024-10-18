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

use crate::fyrox::{
    core::{algebra::Vector2, reflect::prelude::*},
    gui::dock::config::DockingManagerLayoutDescriptor,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Reflect)]
pub struct WindowsSettings {
    #[serde(default)]
    pub window_position: Vector2<f32>,
    #[serde(default)]
    pub window_size: Vector2<f32>,
    #[serde(default)]
    pub window_maximized: bool,
    #[serde(default)]
    #[reflect(hidden)]
    pub layout: Option<DockingManagerLayoutDescriptor>,
}

impl Default for WindowsSettings {
    fn default() -> Self {
        Self {
            window_position: Vector2::new(0.0, 0.0),
            window_size: Vector2::new(1024.0, 768.0),
            window_maximized: true,
            layout: None,
        }
    }
}
