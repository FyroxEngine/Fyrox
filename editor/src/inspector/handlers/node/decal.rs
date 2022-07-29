use crate::scene::commands::SetNodePropertyCommand;
use crate::SceneCommand;
use fyrox::{core::pool::Handle, gui::inspector::PropertyChanged, scene::node::Node};

pub fn handle_decal_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
) -> Option<SceneCommand> {
    Some(SceneCommand::new(
        SetNodePropertyCommand::from_property_changed(handle, args),
    ))
}
