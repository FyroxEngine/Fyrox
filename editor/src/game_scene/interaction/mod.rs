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
    core::{algebra::Vector3, pool::Handle, pool::ObjectOrVariant},
    scene::{
        camera::{Camera, Projection},
        graph::Graph,
        node::Node,
    },
};

pub mod gizmo;
pub mod move_mode;
pub mod plane;
pub mod rotate_mode;
pub mod scale_mode;
pub mod select_mode;

pub fn calculate_gizmo_distance_scaling(
    graph: &Graph,
    camera: Handle<Camera>,
    gizmo_origin: Handle<impl ObjectOrVariant<Node>>,
) -> Vector3<f32> {
    let s = match graph[camera].projection() {
        Projection::Perspective(proj) => {
            distance_scale_factor(proj.fov)
                * graph[gizmo_origin.to_base()]
                    .global_position()
                    .metric_distance(&graph[camera].global_position())
        }
        Projection::Orthographic(ortho) => 0.4 * ortho.vertical_size,
    };

    Vector3::new(s, s, s)
}

fn distance_scale_factor(fov: f32) -> f32 {
    fov.tan() * 0.1
}
