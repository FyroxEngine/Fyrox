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
        graph::SceneGraph,
        scene::{
            collider::{Collider, ColliderShape},
            node::Node,
            Scene,
        },
    },
    plugins::collider::{make_handle, try_get_collider_shape, ShapeGizmoTrait, ShapeHandleValue},
};

pub struct CuboidShapeGizmo {
    pos_x_handle: Handle<Node>,
    pos_y_handle: Handle<Node>,
    pos_z_handle: Handle<Node>,
    neg_x_handle: Handle<Node>,
    neg_y_handle: Handle<Node>,
    neg_z_handle: Handle<Node>,
}

impl CuboidShapeGizmo {
    pub fn new(visible: bool, root: Handle<Node>, scene: &mut Scene) -> Self {
        Self {
            pos_x_handle: make_handle(scene, root, visible),
            pos_y_handle: make_handle(scene, root, visible),
            pos_z_handle: make_handle(scene, root, visible),
            neg_x_handle: make_handle(scene, root, visible),
            neg_y_handle: make_handle(scene, root, visible),
            neg_z_handle: make_handle(scene, root, visible),
        }
    }
}

impl ShapeGizmoTrait for CuboidShapeGizmo {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>)) {
        for handle in [
            self.pos_x_handle,
            self.pos_y_handle,
            self.pos_z_handle,
            self.neg_x_handle,
            self.neg_y_handle,
            self.neg_z_handle,
        ] {
            func(handle)
        }
    }

    fn handle_local_position(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<Vector3<f32>> {
        let Some(ColliderShape::Cuboid(cuboid)) = try_get_collider_shape(collider, scene) else {
            return None;
        };

        if handle == self.pos_x_handle {
            Some(Vector3::new(cuboid.half_extents.x, 0.0, 0.0))
        } else if handle == self.pos_y_handle {
            Some(Vector3::new(0.0, cuboid.half_extents.y, 0.0))
        } else if handle == self.pos_z_handle {
            Some(Vector3::new(0.0, 0.0, cuboid.half_extents.z))
        } else if handle == self.neg_x_handle {
            Some(Vector3::new(-cuboid.half_extents.x, 0.0, 0.0))
        } else if handle == self.neg_y_handle {
            Some(Vector3::new(0.0, -cuboid.half_extents.y, 0.0))
        } else if handle == self.neg_z_handle {
            Some(Vector3::new(0.0, 0.0, -cuboid.half_extents.z))
        } else {
            None
        }
    }

    fn handle_major_axis(
        &self,
        handle: Handle<Node>,
        _collider: Handle<Node>,
        _scene: &Scene,
    ) -> Option<Vector3<f32>> {
        if handle == self.pos_x_handle {
            Some(Vector3::x())
        } else if handle == self.pos_y_handle {
            Some(Vector3::y())
        } else if handle == self.pos_z_handle {
            Some(Vector3::z())
        } else if handle == self.neg_x_handle {
            Some(-Vector3::x())
        } else if handle == self.neg_y_handle {
            Some(-Vector3::y())
        } else if handle == self.neg_z_handle {
            Some(-Vector3::z())
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
        let Some(ColliderShape::Cuboid(cuboid)) = try_get_collider_shape(collider, scene) else {
            return None;
        };

        if handle == self.pos_x_handle {
            Some(ShapeHandleValue::Scalar(cuboid.half_extents.x))
        } else if handle == self.pos_y_handle {
            Some(ShapeHandleValue::Scalar(cuboid.half_extents.y))
        } else if handle == self.pos_z_handle {
            Some(ShapeHandleValue::Scalar(cuboid.half_extents.z))
        } else if handle == self.neg_x_handle {
            Some(ShapeHandleValue::Scalar(cuboid.half_extents.x))
        } else if handle == self.neg_y_handle {
            Some(ShapeHandleValue::Scalar(cuboid.half_extents.y))
        } else if handle == self.neg_z_handle {
            Some(ShapeHandleValue::Scalar(cuboid.half_extents.z))
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
        initial_collider_local_position: Vector3<f32>,
    ) {
        let Ok(collider) = scene.graph.try_get_mut_of_type::<Collider>(collider) else {
            return;
        };

        let ColliderShape::Cuboid(cuboid) = collider.shape_mut() else {
            return;
        };

        if handle == self.pos_x_handle {
            cuboid.half_extents.x = value.into_scalar().max(0.0);
        } else if handle == self.pos_y_handle {
            cuboid.half_extents.y = value.into_scalar().max(0.0);
        } else if handle == self.pos_z_handle {
            cuboid.half_extents.z = value.into_scalar().max(0.0);
        } else if handle == self.neg_x_handle {
            cuboid.half_extents.x = value.into_scalar().max(0.0);
            let transform = collider.local_transform_mut();
            transform.set_position(Vector3::new(
                initial_collider_local_position.x - value.into_scalar() / 2.0,
                initial_collider_local_position.y,
                initial_collider_local_position.z,
            ));
        } else if handle == self.neg_y_handle {
            cuboid.half_extents.y = value.into_scalar().max(0.0);
            let transform = collider.local_transform_mut();
            transform.set_position(Vector3::new(
                initial_collider_local_position.x,
                initial_collider_local_position.y - value.into_scalar() / 2.0,
                initial_collider_local_position.z,
            ));
        } else if handle == self.neg_z_handle {
            cuboid.half_extents.z = value.into_scalar().max(0.0);
            let transform = collider.local_transform_mut();
            transform.set_position(Vector3::new(
                initial_collider_local_position.x,
                initial_collider_local_position.y,
                initial_collider_local_position.z - value.into_scalar() / 2.0,
            ));
        }
    }
}
