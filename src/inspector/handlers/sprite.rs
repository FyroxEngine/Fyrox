use crate::{
    do_command,
    inspector::SenderHelper,
    scene::commands::sprite::{
        SetSpriteColorCommand, SetSpriteRotationCommand, SetSpriteSizeCommand,
        SetSpriteTextureCommand,
    },
};
use rg3d::{
    core::pool::Handle,
    gui::{message::FieldKind, message::PropertyChanged},
    scene::{node::Node, sprite::Sprite},
};

pub fn handle_sprite_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    helper: &SenderHelper,
) -> Option<()> {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
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
            _ => println!("Unhandled property of Sprite: {:?}", args),
        }
    }

    Some(())
}
