use crate::{
    inspector::handlers::node::base::handle_base_property_changed, make_command,
    scene::commands::sprite::*, SceneCommand,
};
use rg3d::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{node::Node, sprite::Sprite},
};

pub fn handle_sprite_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
) -> Option<SceneCommand> {
    if let Node::Sprite(_) = node {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                Sprite::TEXTURE => {
                    make_command!(SetSpriteTextureCommand, handle, value)
                }
                Sprite::COLOR => {
                    make_command!(SetSpriteColorCommand, handle, value)
                }
                Sprite::SIZE => {
                    make_command!(SetSpriteSizeCommand, handle, value)
                }
                Sprite::ROTATION => {
                    make_command!(SetSpriteRotationCommand, handle, value)
                }
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                Sprite::BASE => handle_base_property_changed(inner, handle, node),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
