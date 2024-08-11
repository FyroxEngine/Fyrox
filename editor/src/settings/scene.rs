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

use crate::fyrox::{core::algebra::Vector3, core::pool::ErasedHandle, scene::camera::Projection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct SceneCameraSettings {
    pub position: Vector3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    #[serde(default)]
    pub projection: Projection,
}

impl Default for SceneCameraSettings {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 1.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            projection: Default::default(),
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct NodeInfo {
    pub is_expanded: bool,
}

impl Default for NodeInfo {
    fn default() -> Self {
        Self { is_expanded: true }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Default)]
pub struct SceneSettings {
    pub camera_settings: SceneCameraSettings,
    pub node_infos: HashMap<ErasedHandle, NodeInfo>,
}
