use crate::{inspector::handlers::node::base::handle_base_property_changed, SceneCommand};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{node::Node, sprite::Sprite},
};

pub fn handle_pivot_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &mut Node,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            Sprite::BASE => handle_base_property_changed(inner, handle, node),
            _ => None,
        },
        _ => None,
    }
}
