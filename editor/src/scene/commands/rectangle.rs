use crate::{
    define_node_command, get_set_swap,
    scene::commands::{Command, SceneContext},
};
use rg3d::{
    core::{color::Color, pool::Handle},
    resource::texture::Texture,
    scene::{graph::Graph, node::Node},
};

define_node_command!(SetRectangleColorCommand("Set Rectangle Color", Color) where fn swap(self, node) {
    get_set_swap!(self, node.as_rectangle_mut(), color, set_color);
});

define_node_command!(SetRectangleTextureCommand("Set Rectangle Texture", Option<Texture>) where fn swap(self, node) {
    get_set_swap!(self, node.as_rectangle_mut(), texture_value, set_texture);
});
