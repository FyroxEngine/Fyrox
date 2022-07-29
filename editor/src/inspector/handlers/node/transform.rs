use crate::scene::commands::SetNodePropertyCommand;
use crate::SceneCommand;
use fyrox::{core::pool::Handle, gui::inspector::PropertyChanged, scene::node::Node};

pub fn handle_transform_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
) -> Option<SceneCommand> {
    Some(SceneCommand::new(
        SetNodePropertyCommand::from_property_changed(node_handle, args),
    ))
}
