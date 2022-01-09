use crate::{
    define_node_command, get_set_swap,
    scene::commands::{Command, SceneContext},
};
use fyrox::{
    core::{color::Color, pool::Handle},
    resource::texture::Texture,
    scene::{graph::Graph, node::Node},
};

define_node_command!(SetSpriteSizeCommand("Set Sprite Size", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_sprite_mut(), size, set_size);
});

define_node_command!(SetSpriteRotationCommand("Set Sprite Rotation", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_sprite_mut(), rotation, set_rotation);
});

define_node_command!(SetSpriteColorCommand("Set Sprite Color", Color) where fn swap(self, node) {
    get_set_swap!(self, node.as_sprite_mut(), color, set_color);
});

define_node_command!(SetSpriteTextureCommand("Set Sprite Texture", Option<Texture>) where fn swap(self, node) {
    get_set_swap!(self, node.as_sprite_mut(), texture, set_texture);
});
