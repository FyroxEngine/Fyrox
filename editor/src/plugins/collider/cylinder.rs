use crate::{
    fyrox::{
        core::{algebra::Vector3, pool::Handle},
        scene::{collider::ColliderShape, node::Node, Scene},
    },
    plugins::collider::{
        make_handle, try_get_collider_shape, try_get_collider_shape_mut, ShapeGizmoTrait,
        ShapeHandleValue,
    },
};

pub struct CylinderShapeGizmo {
    radius_handle: Handle<Node>,
    half_height_handle: Handle<Node>,
}

impl CylinderShapeGizmo {
    pub fn new(visible: bool, root: Handle<Node>, scene: &mut Scene) -> Self {
        Self {
            radius_handle: make_handle(scene, root, visible),
            half_height_handle: make_handle(scene, root, visible),
        }
    }
}

impl ShapeGizmoTrait for CylinderShapeGizmo {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>)) {
        for handle in [self.radius_handle, self.half_height_handle] {
            func(handle)
        }
    }

    fn handle_local_position(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<Vector3<f32>> {
        let Some(ColliderShape::Cylinder(cylinder)) = try_get_collider_shape(collider, scene)
        else {
            return None;
        };

        if handle == self.radius_handle {
            Some(Vector3::new(cylinder.radius, 0.0, 0.0))
        } else if handle == self.half_height_handle {
            Some(Vector3::new(0.0, cylinder.half_height, 0.0))
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
        if handle == self.radius_handle {
            Some(Vector3::x())
        } else if handle == self.half_height_handle {
            Some(Vector3::y())
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
        let Some(ColliderShape::Cylinder(cylinder)) = try_get_collider_shape(collider, scene)
        else {
            return None;
        };

        if handle == self.radius_handle {
            Some(ShapeHandleValue::Scalar(cylinder.radius))
        } else if handle == self.half_height_handle {
            Some(ShapeHandleValue::Scalar(cylinder.half_height))
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
        let Some(ColliderShape::Cylinder(cylinder)) = try_get_collider_shape_mut(collider, scene)
        else {
            return;
        };

        if handle == self.radius_handle {
            cylinder.radius = value.into_scalar().max(0.0);
        } else if handle == self.half_height_handle {
            cylinder.half_height = value.into_scalar().max(0.0);
        }
    }
}
