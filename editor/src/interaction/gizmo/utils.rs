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

use crate::{
    fyrox::{
        core::{pool::Handle, some_or_return},
        scene::{graph::Graph, node::Node},
    },
    interaction::calculate_gizmo_distance_scaling,
    scene::{Selection, SelectionContainer},
    settings::Settings,
};
use fyrox::scene::camera::Camera;

pub fn sync_gizmo_with_selection(
    gizmo_origin: Handle<Node>,
    graph: &mut Graph,
    camera: Handle<Camera>,
    settings: &Settings,
    selection: &Selection,
) {
    graph[gizmo_origin].set_visibility(false);

    let selection = some_or_return!(selection.as_graph());
    if selection.is_empty() {
        return;
    }

    let (rotation, position) = some_or_return!(selection.global_rotation_position(graph));

    let node = &mut graph[gizmo_origin];
    node.set_visibility(true);
    node.local_transform_mut()
        .set_rotation(rotation)
        .set_position(position);
    graph.update_hierarchical_data_for_descendants(gizmo_origin);
    let scale = calculate_gizmo_distance_scaling(graph, camera, gizmo_origin)
        * settings.graphics.gizmo_scale;
    graph[gizmo_origin].local_transform_mut().set_scale(scale);
    graph.update_hierarchical_data_for_descendants(gizmo_origin);
}
