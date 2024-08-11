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
        scene::{node::Node, Scene},
    },
    plugins::collider::{ShapeGizmoTrait, ShapeHandleValue},
};

pub struct DummyShapeGizmo;

impl ShapeGizmoTrait for DummyShapeGizmo {
    fn for_each_handle(&self, _func: &mut dyn FnMut(Handle<Node>)) {}

    fn handle_local_position(
        &self,
        _handle: Handle<Node>,
        _collider: Handle<Node>,
        _scene: &Scene,
    ) -> Option<Vector3<f32>> {
        None
    }

    fn value_by_handle(
        &self,
        _handle: Handle<Node>,
        _collider: Handle<Node>,
        _scene: &Scene,
    ) -> Option<ShapeHandleValue> {
        None
    }

    fn set_value_by_handle(
        &self,
        _handle: Handle<Node>,
        _value: ShapeHandleValue,
        _collider: Handle<Node>,
        _scene: &mut Scene,
        _initial_collider_local_position: Vector3<f32>,
    ) {
    }
}
