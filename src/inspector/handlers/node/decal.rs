use crate::inspector::handlers::node::base::handle_base_property_changed;
use crate::{do_command, inspector::SenderHelper, scene::commands::decal::*};
use rg3d::{
    core::pool::Handle,
    gui::message::{FieldKind, PropertyChanged},
    scene::{decal::Decal, node::Node},
};

pub fn handle_decal_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
    helper: &SenderHelper,
) -> Option<()> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            Decal::DIFFUSE_TEXTURE => {
                do_command!(helper, SetDecalDiffuseTextureCommand, handle, value)
            }
            Decal::NORMAL_TEXTURE => {
                do_command!(helper, SetDecalNormalTextureCommand, handle, value)
            }
            Decal::COLOR => {
                do_command!(helper, SetDecalColorCommand, handle, value)
            }
            Decal::LAYER => {
                do_command!(helper, SetDecalLayerIndexCommand, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            Decal::BASE => handle_base_property_changed(&inner, handle, node, helper),
            _ => None,
        },
        _ => None,
    }
}
