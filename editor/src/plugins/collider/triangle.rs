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

pub struct TriangleShapeGizmo {
    a_handle: Handle<Node>,
    b_handle: Handle<Node>,
    c_handle: Handle<Node>,
}

impl TriangleShapeGizmo {
    pub fn new(root: Handle<Node>, visible: bool, scene: &mut Scene) -> Self {
        Self {
            a_handle: make_handle(scene, root, visible),
            b_handle: make_handle(scene, root, visible),
            c_handle: make_handle(scene, root, visible),
        }
    }
}

impl ShapeGizmoTrait for TriangleShapeGizmo {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>)) {
        for handle in [self.a_handle, self.b_handle, self.c_handle] {
            func(handle)
        }
    }

    fn handle_local_position(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<Vector3<f32>> {
        let Some(ColliderShape::Triangle(triangle)) = try_get_collider_shape(collider, scene)
        else {
            return None;
        };

        if handle == self.a_handle {
            Some(triangle.a)
        } else if handle == self.b_handle {
            Some(triangle.b)
        } else if handle == self.c_handle {
            Some(triangle.c)
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
        let Some(ColliderShape::Triangle(triangle)) = try_get_collider_shape(collider, scene)
        else {
            return None;
        };

        if handle == self.a_handle {
            Some(ShapeHandleValue::Vector(triangle.a))
        } else if handle == self.b_handle {
            Some(ShapeHandleValue::Vector(triangle.b))
        } else if handle == self.c_handle {
            Some(ShapeHandleValue::Vector(triangle.c))
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
        let Some(ColliderShape::Triangle(triangle)) = try_get_collider_shape_mut(collider, scene)
        else {
            return;
        };

        if handle == self.a_handle {
            triangle.a = value.into_vector();
        } else if handle == self.b_handle {
            triangle.b = value.into_vector();
        } else if handle == self.c_handle {
            triangle.c = value.into_vector();
        }
    }

    fn is_vector_handle(&self, handle: Handle<Node>) -> bool {
        handle == self.a_handle || handle == self.b_handle || handle == self.c_handle
    }
}
