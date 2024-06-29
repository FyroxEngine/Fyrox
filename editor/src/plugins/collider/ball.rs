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

pub struct BallShapeGizmo {
    radius_handle: Handle<Node>,
}

impl BallShapeGizmo {
    pub fn new(root: Handle<Node>, visible: bool, scene: &mut Scene) -> Self {
        Self {
            radius_handle: make_handle(scene, root, visible),
        }
    }
}

impl ShapeGizmoTrait for BallShapeGizmo {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>)) {
        func(self.radius_handle)
    }

    fn handle_local_position(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<Vector3<f32>> {
        let Some(ColliderShape::Ball(ball)) = try_get_collider_shape(collider, scene) else {
            return None;
        };

        if handle == self.radius_handle {
            Some(Vector3::new(ball.radius, 0.0, 0.0))
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
        let Some(ColliderShape::Ball(ball)) = try_get_collider_shape(collider, scene) else {
            return None;
        };

        if handle == self.radius_handle {
            Some(ShapeHandleValue::Scalar(ball.radius))
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
        let Some(ColliderShape::Ball(ball)) = try_get_collider_shape_mut(collider, scene) else {
            return;
        };

        if handle == self.radius_handle {
            ball.radius = value.into_scalar().max(0.0);
        }
    }
}
