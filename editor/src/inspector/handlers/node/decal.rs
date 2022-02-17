use crate::{
    handle_properties, inspector::handlers::node::base::handle_base_property_changed,
    scene::commands::decal::*, SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{decal::Decal, node::Node},
};

pub fn handle_decal_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
) -> Option<SceneCommand> {
    if node.is_decal() {
        match args.value {
            FieldKind::Object(ref value) => {
                handle_properties!(args.name.as_ref(), handle, value,
                    Decal::DIFFUSE_TEXTURE => SetDecalDiffuseTextureCommand,
                    Decal::NORMAL_TEXTURE => SetDecalNormalTextureCommand,
                    Decal::COLOR => SetDecalColorCommand,
                    Decal::LAYER => SetDecalLayerIndexCommand
                )
            }
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
