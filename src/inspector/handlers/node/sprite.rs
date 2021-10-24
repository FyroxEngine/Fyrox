use crate::inspector::handlers::node::base::handle_base_property_changed;
use crate::{do_command, inspector::SenderHelper, scene::commands::sprite::*};
use rg3d::{
    core::pool::Handle,
    gui::{message::FieldKind, message::PropertyChanged},
    scene::{node::Node, sprite::Sprite},
};

pub fn handle_sprite_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
    helper: &SenderHelper,
) -> Option<()> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            Sprite::TEXTURE => {
                do_command!(helper, SetSpriteTextureCommand, handle, value)
            }
            Sprite::COLOR => {
                do_command!(helper, SetSpriteColorCommand, handle, value)
            }
            Sprite::SIZE => {
                do_command!(helper, SetSpriteSizeCommand, handle, value)
            }
            Sprite::ROTATION => {
                do_command!(helper, SetSpriteRotationCommand, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            Sprite::BASE => handle_base_property_changed(&inner, handle, node, helper),
            _ => None,
        },
        _ => None,
    }
}
