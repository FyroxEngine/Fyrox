use crate::{
    fyrox::{
        core::{algebra::Vector3, pool::Handle},
        scene::{
            collider::{ColliderShape, SegmentShape},
            node::Node,
            Scene,
        },
    },
    plugins::collider::{
        make_handle, set_node_position, try_get_collider_shape, try_get_collider_shape_mut,
        ShapeGizmoTrait, ShapeHandleValue,
    },
};

pub struct SegmentShapeGizmo {
    begin_handle: Handle<Node>,
    end_handle: Handle<Node>,
}

impl SegmentShapeGizmo {
    pub fn new(
        segment: &SegmentShape,
        center: Vector3<f32>,
        root: Handle<Node>,
        visible: bool,
        scene: &mut Scene,
    ) -> Self {
        Self {
            begin_handle: make_handle(scene, center + segment.begin, root, visible),
            end_handle: make_handle(scene, center + segment.end, root, visible),
        }
    }
}

impl ShapeGizmoTrait for SegmentShapeGizmo {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>)) {
        for handle in [self.begin_handle, self.end_handle] {
            func(handle)
        }
    }

    fn handle_major_axis(&self, _handle: Handle<Node>) -> Option<Vector3<f32>> {
        None
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
        let Some(ColliderShape::Segment(segment)) = try_get_collider_shape(collider, scene) else {
            return false;
        };

        set_node_position(self.begin_handle, center + segment.begin, scene);
        set_node_position(self.end_handle, center + segment.end, scene);

        true
    }

    fn value_by_handle(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<ShapeHandleValue> {
        let Some(ColliderShape::Segment(segment)) = try_get_collider_shape(collider, scene) else {
            return None;
        };

        if handle == self.begin_handle {
            Some(ShapeHandleValue::Vector(segment.begin))
        } else if handle == self.end_handle {
            Some(ShapeHandleValue::Vector(segment.end))
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
        let Some(ColliderShape::Segment(segment)) = try_get_collider_shape_mut(collider, scene)
        else {
            return;
        };

        if handle == self.begin_handle {
            segment.begin = value.into_vector();
        } else if handle == self.end_handle {
            segment.end = value.into_vector();
        }
    }

    fn is_vector_handle(&self, handle: Handle<Node>) -> bool {
        handle == self.begin_handle || handle == self.end_handle
    }
}
