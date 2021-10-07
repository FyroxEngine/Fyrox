use crate::{
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
    node_handle: Handle<Node>,
    helper: &SenderHelper,
) -> Option<()> {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
            Sprite::TEXTURE => {
                helper.do_scene_command(SetSpriteTextureCommand::new(
                    node_handle,
                    value.cast_value().cloned()?,
                ));
            }
            Sprite::COLOR => helper.do_scene_command(SetSpriteColorCommand::new(
                node_handle,
                *value.cast_value()?,
            )),
            Sprite::SIZE => helper
                .do_scene_command(SetSpriteSizeCommand::new(node_handle, *value.cast_value()?)),
            Sprite::ROTATION => helper.do_scene_command(SetSpriteRotationCommand::new(
                node_handle,
                *value.cast_value()?,
            )),
            _ => println!("Unhandled property of Transform: {:?}", args),
        }
    }

    Some(())
}
