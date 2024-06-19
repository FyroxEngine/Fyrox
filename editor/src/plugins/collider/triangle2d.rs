use crate::{
    fyrox::{
        core::{algebra::Vector3, pool::Handle},
        scene::{
            dim2::collider::{ColliderShape, TriangleShape},
            node::Node,
            Scene,
        },
    },
    plugins::collider::{
        make_handle, set_node_position, try_get_collider_shape_2d, try_get_collider_shape_mut_2d,
        ShapeGizmoTrait, ShapeHandleValue,
    },
};

pub struct Triangle2DShapeGizmo {
    a_handle: Handle<Node>,
    b_handle: Handle<Node>,
    c_handle: Handle<Node>,
}

impl Triangle2DShapeGizmo {
    pub fn new(
        triangle: &TriangleShape,
        center: Vector3<f32>,
        root: Handle<Node>,
        visible: bool,
        scene: &mut Scene,
    ) -> Self {
        Self {
            a_handle: make_handle(scene, center + triangle.a.to_homogeneous(), root, visible),
            b_handle: make_handle(scene, center + triangle.b.to_homogeneous(), root, visible),
            c_handle: make_handle(scene, center + triangle.c.to_homogeneous(), root, visible),
        }
    }
}

impl ShapeGizmoTrait for Triangle2DShapeGizmo {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>)) {
        for handle in [self.a_handle, self.b_handle, self.c_handle] {
            func(handle)
        }
    }

    fn try_sync_to_collider(
        &self,
        collider: Handle<Node>,
        center: Vector3<f32>,
        _side: Vector3<f32>,
        _up: Vector3<f32>,
        _look: Vector3<f32>,
        scene: &mut Scene,
    ) -> bool {
        let Some(ColliderShape::Triangle(triangle)) = try_get_collider_shape_2d(collider, scene)
        else {
            return false;
        };

        set_node_position(self.a_handle, center + triangle.a.to_homogeneous(), scene);
        set_node_position(self.b_handle, center + triangle.b.to_homogeneous(), scene);
        set_node_position(self.c_handle, center + triangle.c.to_homogeneous(), scene);

        true
    }

    fn value_by_handle(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<ShapeHandleValue> {
        let Some(ColliderShape::Triangle(triangle)) = try_get_collider_shape_2d(collider, scene)
        else {
            return None;
        };

        if handle == self.a_handle {
            Some(ShapeHandleValue::Vector(triangle.a.to_homogeneous()))
        } else if handle == self.b_handle {
            Some(ShapeHandleValue::Vector(triangle.b.to_homogeneous()))
        } else if handle == self.c_handle {
            Some(ShapeHandleValue::Vector(triangle.c.to_homogeneous()))
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
        let Some(ColliderShape::Triangle(triangle)) =
            try_get_collider_shape_mut_2d(collider, scene)
        else {
            return;
        };

        if handle == self.a_handle {
            triangle.a = value.into_vector().xy();
        } else if handle == self.b_handle {
            triangle.b = value.into_vector().xy();
        } else if handle == self.c_handle {
            triangle.c = value.into_vector().xy();
        }
    }

    fn is_vector_handle(&self, handle: Handle<Node>) -> bool {
        handle == self.a_handle || handle == self.b_handle || handle == self.c_handle
    }
}
