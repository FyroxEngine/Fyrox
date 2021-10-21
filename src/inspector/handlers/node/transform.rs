use crate::{do_command, inspector::SenderHelper, scene::commands::graph::*};
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
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            "local_position" => {
                helper.do_scene_command(MoveNodeCommand::new(
                    node_handle,
                    **node.local_transform().position(),
                    *value.cast_value()?,
                ));
                Some(())
            }
            "local_rotation" => {
                helper.do_scene_command(RotateNodeCommand::new(
                    node_handle,
                    **node.local_transform().rotation(),
                    *value.cast_value()?,
                ));
                Some(())
            }
            "local_scale" => {
                helper.do_scene_command(ScaleNodeCommand::new(
                    node_handle,
                    **node.local_transform().scale(),
                    *value.cast_value()?,
                ));
                Some(())
            }
            "pre_rotation" => {
                do_command!(helper, SetPreRotationCommand, node_handle, value)
            }
            "post_rotation" => {
                do_command!(helper, SetPostRotationCommand, node_handle, value)
            }
            "rotation_offset" => {
                do_command!(helper, SetRotationOffsetCommand, node_handle, value)
            }
            "rotation_pivot" => {
                do_command!(helper, SetRotationPivotCommand, node_handle, value)
            }
            "scaling_offset" => {
                do_command!(helper, SetScaleOffsetCommand, node_handle, value)
            }
            "scaling_pivot" => {
                do_command!(helper, SetScalePivotCommand, node_handle, value)
            }
            _ => None,
        },
        _ => None,
    }
}
