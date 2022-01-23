use crate::{
    define_swap_command,
    scene::commands::{Command, SceneContext},
};
use fyrox::{core::color::Color, resource::texture::Texture, scene::node::Node};

define_swap_command! {
    Node::as_sprite_mut,
    SetSpriteSizeCommand(f32): size, set_size, "Set Sprite Size";
    SetSpriteRotationCommand(f32): rotation, set_rotation, "Set Sprite Rotation";
    SetSpriteColorCommand(Color): color, set_color, "Set Sprite Color";
    SetSpriteTextureCommand(Option<Texture>): texture, set_texture, "Set Sprite Texture";
}
