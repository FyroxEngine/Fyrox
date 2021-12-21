use crate::{make_command, scene::commands::graph::*, SceneCommand};
use rg3d::scene::base::Base;
use rg3d::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::node::Node,
};

pub fn handle_transform_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
    base: &Base,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            "local_position" => Some(SceneCommand::new(MoveNodeCommand::new(
                node_handle,
                **base.local_transform().position(),
                *value.cast_value()?,
            ))),
            "local_rotation" => Some(SceneCommand::new(RotateNodeCommand::new(
                node_handle,
                **base.local_transform().rotation(),
                *value.cast_value()?,
            ))),
            "local_scale" => Some(SceneCommand::new(ScaleNodeCommand::new(
                node_handle,
                **base.local_transform().scale(),
                *value.cast_value()?,
            ))),
            "pre_rotation" => {
                make_command!(SetPreRotationCommand, node_handle, value)
            }
            "post_rotation" => {
                make_command!(SetPostRotationCommand, node_handle, value)
            }
            "rotation_offset" => {
                make_command!(SetRotationOffsetCommand, node_handle, value)
            }
            "rotation_pivot" => {
                make_command!(SetRotationPivotCommand, node_handle, value)
            }
            "scaling_offset" => {
                make_command!(SetScaleOffsetCommand, node_handle, value)
            }
            "scaling_pivot" => {
                make_command!(SetScalePivotCommand, node_handle, value)
            }
            _ => None,
        },
        _ => None,
    }
}
