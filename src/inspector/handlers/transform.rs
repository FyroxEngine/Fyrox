use crate::{
    inspector::SenderHelper,
    scene::commands::graph::{MoveNodeCommand, RotateNodeCommand, ScaleNodeCommand},
};
use rg3d::gui::message::FieldKind;
use rg3d::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        pool::Handle,
    },
    gui::message::PropertyChanged,
    scene::node::Node,
};

pub fn handle_transform_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
    node: &Node,
    helper: &SenderHelper,
) {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
            "local_position" => {
                helper.do_scene_command(MoveNodeCommand::new(
                    node_handle,
                    **node.local_transform().position(),
                    *value.cast_value::<Vector3<f32>>().unwrap(),
                ));
            }
            "local_rotation" => {
                helper.do_scene_command(RotateNodeCommand::new(
                    node_handle,
                    **node.local_transform().rotation(),
                    *value.cast_value::<UnitQuaternion<f32>>().unwrap(),
                ));
            }
            "local_scale" => {
                helper.do_scene_command(ScaleNodeCommand::new(
                    node_handle,
                    **node.local_transform().scale(),
                    *value.cast_value::<Vector3<f32>>().unwrap(),
                ));
            }
            _ => println!("Unhandled property of Transform: {:?}", args),
        }
    }
}
