use crate::{
    fyrox::{
        core::{algebra::Vector3, pool::Handle},
        scene::{
            dim2::collider::{BallShape, ColliderShape},
            node::Node,
            Scene,
        },
    },
    plugins::collider::{
        make_handle, set_node_position, try_get_collider_shape_2d, try_get_collider_shape_mut_2d,
        ShapeGizmoTrait, ShapeHandleValue,
    },
};

pub struct Ball2DShapeGizmo {
    radius_handle: Handle<Node>,
}

impl Ball2DShapeGizmo {
    pub fn new(
        ball: &BallShape,
        center: Vector3<f32>,
        side: Vector3<f32>,
        root: Handle<Node>,
        visible: bool,
        scene: &mut Scene,
    ) -> Self {
        Self {
            radius_handle: make_handle(scene, center + side.scale(ball.radius), root, visible),
        }
    }
}

impl ShapeGizmoTrait for Ball2DShapeGizmo {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>)) {
        func(self.radius_handle)
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

    fn try_sync_to_collider(
        &self,
        collider: Handle<Node>,
        center: Vector3<f32>,
        side: Vector3<f32>,
        _up: Vector3<f32>,
        _look: Vector3<f32>,
        scene: &mut Scene,
    ) -> bool {
        let Some(ColliderShape::Ball(ball)) = try_get_collider_shape_2d(collider, scene) else {
            return false;
        };

        set_node_position(self.radius_handle, center + side.scale(ball.radius), scene);

        true
    }

    fn value_by_handle(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<ShapeHandleValue> {
        let Some(ColliderShape::Ball(ball)) = try_get_collider_shape_2d(collider, scene) else {
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
        let Some(ColliderShape::Ball(ball)) = try_get_collider_shape_mut_2d(collider, scene) else {
            return;
        };

        if handle == self.radius_handle {
            ball.radius = value.into_scalar();
        }
    }
}
