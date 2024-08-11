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
        core::{algebra::Vector3, pool::Handle},
        scene::{dim2::collider::ColliderShape, node::Node, Scene},
    },
    plugins::collider::{
        make_handle, try_get_collider_shape_2d, try_get_collider_shape_mut_2d, ShapeGizmoTrait,
        ShapeHandleValue,
    },
};

pub struct Segment2DShapeGizmo {
    begin_handle: Handle<Node>,
    end_handle: Handle<Node>,
}

impl Segment2DShapeGizmo {
    pub fn new(root: Handle<Node>, visible: bool, scene: &mut Scene) -> Self {
        Self {
            begin_handle: make_handle(scene, root, visible),
            end_handle: make_handle(scene, root, visible),
        }
    }
}

impl ShapeGizmoTrait for Segment2DShapeGizmo {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>)) {
        for handle in [self.begin_handle, self.end_handle] {
            func(handle)
        }
    }

    fn handle_local_position(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<Vector3<f32>> {
        let Some(ColliderShape::Segment(segment)) = try_get_collider_shape_2d(collider, scene)
        else {
            return None;
        };

        if handle == self.begin_handle {
            Some(segment.begin.to_homogeneous())
        } else if handle == self.end_handle {
            Some(segment.end.to_homogeneous())
        } else {
            None
        }
    }

    fn value_by_handle(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<ShapeHandleValue> {
        let Some(ColliderShape::Segment(segment)) = try_get_collider_shape_2d(collider, scene)
        else {
            return None;
        };

        if handle == self.begin_handle {
            Some(ShapeHandleValue::Vector(segment.begin.to_homogeneous()))
        } else if handle == self.end_handle {
            Some(ShapeHandleValue::Vector(segment.end.to_homogeneous()))
        } else {
            None
        }
    }

    fn set_value_by_handle(
        &self,
        handle: Handle<Node>,
        value: ShapeHandleValue,
        collider: Handle<Node>,
        scene: &mut Scene,
        _initial_collider_local_position: Vector3<f32>,
    ) {
        let Some(ColliderShape::Segment(segment)) = try_get_collider_shape_mut_2d(collider, scene)
        else {
            return;
        };

        if handle == self.begin_handle {
            segment.begin = value.into_vector().xy();
        } else if handle == self.end_handle {
            segment.end = value.into_vector().xy();
        }
    }

    fn is_vector_handle(&self, handle: Handle<Node>) -> bool {
        handle == self.begin_handle || handle == self.end_handle
    }
}
