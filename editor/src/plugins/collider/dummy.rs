use crate::{
    fyrox::{
        core::{algebra::Vector3, pool::Handle},
        scene::{node::Node, Scene},
    },
    plugins::collider::{ShapeGizmoTrait, ShapeHandleValue},
};

pub struct DummyShapeGizmo;

impl ShapeGizmoTrait for DummyShapeGizmo {
    fn for_each_handle(&self, _func: &mut dyn FnMut(Handle<Node>)) {}

    fn try_sync_to_collider(
        &self,
        _collider: Handle<Node>,
        _center: Vector3<f32>,
        _side: Vector3<f32>,
        _up: Vector3<f32>,
        _look: Vector3<f32>,
        _scene: &mut Scene,
    ) -> bool {
        false
    }

    fn value_by_handle(
        &self,
        _handle: Handle<Node>,
        _collider: Handle<Node>,
        _scene: &Scene,
    ) -> Option<ShapeHandleValue> {
        None
    }

    fn set_value_by_handle(
        &self,
        _handle: Handle<Node>,
        _value: ShapeHandleValue,
        _collider: Handle<Node>,
        _scene: &mut Scene,
        _initial_collider_local_position: Vector3<f32>,
    ) {
    }
}
