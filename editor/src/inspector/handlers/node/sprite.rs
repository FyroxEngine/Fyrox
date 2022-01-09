use crate::{
    handle_properties, inspector::handlers::node::base::handle_base_property_changed,
    scene::commands::sprite::*, SceneCommand,
};
use fyrox::{
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
            FieldKind::Object(ref value) => {
                handle_properties!(args.name.as_ref(), handle, value,
                    Sprite::TEXTURE => SetSpriteTextureCommand,
                    Sprite::COLOR => SetSpriteColorCommand,
                    Sprite::SIZE => SetSpriteSizeCommand,
                    Sprite::ROTATION => SetSpriteRotationCommand
                )
            }
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
