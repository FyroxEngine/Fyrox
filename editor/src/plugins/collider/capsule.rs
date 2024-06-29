use crate::{
    fyrox::{
        core::{algebra::Vector3, math, pool::Handle},
        scene::{collider::ColliderShape, node::Node, Scene},
    },
    plugins::collider::{
        make_handle, try_get_collider_shape, try_get_collider_shape_mut, ShapeGizmoTrait,
        ShapeHandleValue,
    },
};

pub struct CapsuleShapeGizmo {
    radius_handle: Handle<Node>,
    begin_handle: Handle<Node>,
    end_handle: Handle<Node>,
}

impl CapsuleShapeGizmo {
    pub fn new(visible: bool, root: Handle<Node>, scene: &mut Scene) -> Self {
        Self {
            radius_handle: make_handle(scene, root, visible),
            begin_handle: make_handle(scene, root, visible),
            end_handle: make_handle(scene, root, visible),
        }
    }
}

impl ShapeGizmoTrait for CapsuleShapeGizmo {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>)) {
        for handle in [self.radius_handle, self.begin_handle, self.end_handle] {
            func(handle)
        }
    }

    fn handle_local_position(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<Vector3<f32>> {
        let Some(ColliderShape::Capsule(capsule)) = try_get_collider_shape(collider, scene) else {
            return None;
        };

        if handle == self.radius_handle {
            let perp = math::get_arbitrary_line_perpendicular(capsule.begin, capsule.end)
                .unwrap_or_else(Vector3::x)
                .scale(capsule.radius);

            Some(capsule.begin + perp)
        } else if handle == self.begin_handle {
            Some(capsule.begin)
        } else if handle == self.end_handle {
            Some(capsule.end)
        } else {
            None
        }
    }

    fn handle_major_axis(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<Vector3<f32>> {
        let Some(ColliderShape::Capsule(capsule)) = try_get_collider_shape(collider, scene) else {
            return None;
        };

        if handle == self.radius_handle {
            Some(
                math::get_arbitrary_line_perpendicular(capsule.begin, capsule.end)
                    .unwrap_or_else(Vector3::x),
            )
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
        let Some(ColliderShape::Capsule(capsule)) = try_get_collider_shape(collider, scene) else {
            return None;
        };

        if handle == self.radius_handle {
            Some(ShapeHandleValue::Scalar(capsule.radius))
        } else if handle == self.begin_handle {
            Some(ShapeHandleValue::Vector(capsule.begin))
        } else if handle == self.end_handle {
            Some(ShapeHandleValue::Vector(capsule.end))
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
        let Some(ColliderShape::Capsule(capsule)) = try_get_collider_shape_mut(collider, scene)
        else {
            return;
        };

        if handle == self.radius_handle {
            capsule.radius = value.into_scalar().max(0.0);
        } else if handle == self.begin_handle {
            capsule.begin = value.into_vector();
        } else if handle == self.end_handle {
            capsule.end = value.into_vector();
        }
    }

    fn is_vector_handle(&self, handle: Handle<Node>) -> bool {
        handle == self.begin_handle || handle == self.end_handle
    }
}
