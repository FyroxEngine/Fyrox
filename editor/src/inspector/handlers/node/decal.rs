use crate::{
    inspector::handlers::node::base::handle_base_property_changed, make_command,
    scene::commands::decal::*, SceneCommand,
};
use rg3d::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{decal::Decal, node::Node},
};

pub fn handle_decal_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
) -> Option<SceneCommand> {
    if let Node::Decal(_) = node {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                Decal::DIFFUSE_TEXTURE => {
                    make_command!(SetDecalDiffuseTextureCommand, handle, value)
                }
                Decal::NORMAL_TEXTURE => {
                    make_command!(SetDecalNormalTextureCommand, handle, value)
                }
                Decal::COLOR => {
                    make_command!(SetDecalColorCommand, handle, value)
                }
                Decal::LAYER => {
                    make_command!(SetDecalLayerIndexCommand, handle, value)
                }
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                Decal::BASE => handle_base_property_changed(inner, handle, node),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
