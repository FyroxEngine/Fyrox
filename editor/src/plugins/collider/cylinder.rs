use crate::{
    fyrox::{
        core::{algebra::Vector3, pool::Handle},
        scene::{
            collider::{ColliderShape, CylinderShape},
            node::Node,
            Scene,
        },
    },
    plugins::collider::{
        make_handle, set_node_position, try_get_collider_shape, try_get_collider_shape_mut,
        ShapeGizmoTrait, ShapeHandleValue,
    },
};

pub struct CylinderShapeGizmo {
    radius_handle: Handle<Node>,
    half_height_handle: Handle<Node>,
}

impl CylinderShapeGizmo {
    pub fn new(
        cylinder: &CylinderShape,
        center: Vector3<f32>,
        side: Vector3<f32>,
        up: Vector3<f32>,
        visible: bool,
        root: Handle<Node>,
        scene: &mut Scene,
    ) -> Self {
        Self {
            radius_handle: make_handle(scene, center + side.scale(cylinder.radius), root, visible),
            half_height_handle: make_handle(
                scene,
                center + up.scale(cylinder.half_height),
                root,
                visible,
            ),
        }
    }
}

impl ShapeGizmoTrait for CylinderShapeGizmo {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>)) {
        for handle in [self.radius_handle, self.half_height_handle] {
            func(handle)
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

    fn try_sync_to_collider(
        &self,
        collider: Handle<Node>,
        center: Vector3<f32>,
        side: Vector3<f32>,
        up: Vector3<f32>,
        _look: Vector3<f32>,
        scene: &mut Scene,
    ) -> bool {
        let Some(ColliderShape::Cylinder(cylinder)) = try_get_collider_shape(collider, scene)
        else {
            return false;
        };

        set_node_position(
            self.radius_handle,
            center + side.scale(cylinder.radius),
            scene,
        );
        set_node_position(
            self.half_height_handle,
            center + up.scale(cylinder.half_height),
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
