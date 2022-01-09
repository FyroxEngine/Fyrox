use crate::{
    define_node_command, get_set_swap,
    scene::commands::{Command, SceneContext},
};
use fyrox::{
    core::{color::Color, pool::Handle},
    resource::texture::Texture,
    scene::{graph::Graph, node::Node},
};

define_node_command!(SetDecalDiffuseTextureCommand("Set Decal Diffuse Texture", Option<Texture>) where fn swap(self, node) {
    get_set_swap!(self, node.as_decal_mut(), diffuse_texture_value, set_diffuse_texture);
});

define_node_command!(SetDecalNormalTextureCommand("Set Decal Normal Texture", Option<Texture>) where fn swap(self, node) {
    get_set_swap!(self, node.as_decal_mut(), normal_texture_value, set_normal_texture);
});

define_node_command!(SetDecalColorCommand("Set Decal Color", Color) where fn swap(self, node) {
    get_set_swap!(self, node.as_decal_mut(), color, set_color);
});

define_node_command!(SetDecalLayerIndexCommand("Set Decal Layer Index", u8) where fn swap(self, node) {
    get_set_swap!(self, node.as_decal_mut(), layer, set_layer);
});
