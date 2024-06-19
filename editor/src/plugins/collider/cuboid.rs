use crate::{
    fyrox::{
        core::{algebra::Vector3, pool::Handle},
        graph::SceneGraph,
        scene::{
            collider::{Collider, ColliderShape, CuboidShape},
            node::Node,
            Scene,
        },
    },
    plugins::collider::{
        make_handle, set_node_position, try_get_collider_shape, ShapeGizmoTrait, ShapeHandleValue,
    },
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
    pub fn new(
        cuboid: &CuboidShape,
        center: Vector3<f32>,
        side: Vector3<f32>,
        up: Vector3<f32>,
        look: Vector3<f32>,
        visible: bool,
        root: Handle<Node>,
        scene: &mut Scene,
    ) -> Self {
        Self {
            pos_x_handle: make_handle(
                scene,
                center + side.scale(cuboid.half_extents.x),
                root,
                visible,
            ),
            pos_y_handle: make_handle(
                scene,
                center + up.scale(cuboid.half_extents.y),
                root,
                visible,
            ),
            pos_z_handle: make_handle(
                scene,
                center + look.scale(cuboid.half_extents.z),
                root,
                visible,
            ),
            neg_x_handle: make_handle(
                scene,
                center - side.scale(cuboid.half_extents.x),
                root,
                visible,
            ),
            neg_y_handle: make_handle(
                scene,
                center - up.scale(cuboid.half_extents.y),
                root,
                visible,
            ),
            neg_z_handle: make_handle(
                scene,
                center - look.scale(cuboid.half_extents.z),
                root,
                visible,
            ),
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

    fn try_sync_to_collider(
        &self,
        collider: Handle<Node>,
        center: Vector3<f32>,
        side: Vector3<f32>,
        up: Vector3<f32>,
        look: Vector3<f32>,
        scene: &mut Scene,
    ) -> bool {
        let Some(ColliderShape::Cuboid(cuboid)) = try_get_collider_shape(collider, scene) else {
            return false;
        };

        set_node_position(
            self.pos_x_handle,
            center + side.scale(cuboid.half_extents.x),
            scene,
        );
        set_node_position(
            self.pos_y_handle,
            center + up.scale(cuboid.half_extents.y),
            scene,
        );
        set_node_position(
            self.pos_z_handle,
            center + look.scale(cuboid.half_extents.z),
            scene,
        );
        set_node_position(
            self.neg_x_handle,
            center - side.scale(cuboid.half_extents.x),
            scene,
        );
        set_node_position(
            self.neg_y_handle,
            center - up.scale(cuboid.half_extents.y),
            scene,
        );
        set_node_position(
            self.neg_z_handle,
            center - look.scale(cuboid.half_extents.z),
            scene,
        );

        true
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
        let Some(collider) = scene.graph.try_get_mut_of_type::<Collider>(collider) else {
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
