use crate::{
    fyrox::{
        core::{algebra::Vector3, pool::Handle},
        scene::{
            dim2::collider::{ColliderShape, SegmentShape},
            node::Node,
            Scene,
        },
    },
    plugins::collider::{
        make_handle, set_node_position, try_get_collider_shape_2d, try_get_collider_shape_mut_2d,
        ShapeGizmoTrait, ShapeHandleValue,
    },
};

pub struct Segment2DShapeGizmo {
    begin_handle: Handle<Node>,
    end_handle: Handle<Node>,
}

impl Segment2DShapeGizmo {
    pub fn new(
        segment: &SegmentShape,
        center: Vector3<f32>,
        root: Handle<Node>,
        visible: bool,
        scene: &mut Scene,
    ) -> Self {
        Self {
            begin_handle: make_handle(
                scene,
                center + segment.begin.to_homogeneous(),
                root,
                visible,
            ),
            end_handle: make_handle(scene, center + segment.end.to_homogeneous(), root, visible),
        }
    }
}

impl ShapeGizmoTrait for Segment2DShapeGizmo {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>)) {
        for handle in [self.begin_handle, self.end_handle] {
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
        let Some(ColliderShape::Segment(segment)) = try_get_collider_shape_2d(collider, scene)
        else {
            return false;
        };

        set_node_position(
            self.begin_handle,
            center + segment.begin.to_homogeneous(),
            scene,
        );
        set_node_position(
            self.end_handle,
            center + segment.end.to_homogeneous(),
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
        let Some(ColliderShape::Segment(segment)) = try_get_collider_shape_2d(collider, scene)
        else {
            return None;
        };

        if handle == self.begin_handle {
            Some(ShapeHandleValue::Vector(segment.begin.to_homogeneous()))
        } else if handle == self.end_handle {
            Some(ShapeHandleValue::Vector(segment.end.to_homogeneous()))
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
        let Some(ColliderShape::Segment(segment)) = try_get_collider_shape_mut_2d(collider, scene)
        else {
            return;
        };

        if handle == self.begin_handle {
            segment.begin = value.into_vector().xy();
        } else if handle == self.end_handle {
            segment.end = value.into_vector().xy();
        }
    }

    fn is_vector_handle(&self, handle: Handle<Node>) -> bool {
        handle == self.begin_handle || handle == self.end_handle
    }
}
