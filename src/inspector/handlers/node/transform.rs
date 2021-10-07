use crate::{inspector::SenderHelper, scene::commands::graph::*};
use rg3d::{
    core::pool::Handle,
    gui::message::{FieldKind, PropertyChanged},
    scene::node::Node,
};

pub fn handle_transform_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
    node: &Node,
    helper: &SenderHelper,
) -> Option<()> {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
            "local_position" => {
                helper.do_scene_command(MoveNodeCommand::new(
                    node_handle,
                    **node.local_transform().position(),
                    *value.cast_value()?,
                ));
            }
            "local_rotation" => {
                helper.do_scene_command(RotateNodeCommand::new(
                    node_handle,
                    **node.local_transform().rotation(),
                    *value.cast_value()?,
                ));
            }
            "local_scale" => {
                helper.do_scene_command(ScaleNodeCommand::new(
                    node_handle,
                    **node.local_transform().scale(),
                    *value.cast_value()?,
                ));
            }
            _ => println!("Unhandled property of Transform: {:?}", args),
        }
    }
    Some(())
}
