use crate::{scene::commands::make_set_node_property_command, SceneCommand};
use fyrox::{core::pool::Handle, gui::inspector::PropertyChanged, scene::node::Node};

pub fn handle_transform_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
) -> Option<SceneCommand> {
    Some(make_set_node_property_command(node_handle, args))
}
