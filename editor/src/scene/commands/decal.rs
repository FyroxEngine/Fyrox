use crate::{
    define_swap_command,
    scene::commands::{Command, SceneContext},
};
use fyrox::{core::color::Color, resource::texture::Texture, scene::node::Node};

define_swap_command! {
    Node::as_decal_mut,
    SetDecalDiffuseTextureCommand(Option<Texture>): diffuse_texture_value, set_diffuse_texture, "Set Decal Diffuse Texture";
    SetDecalNormalTextureCommand(Option<Texture>): normal_texture_value, set_normal_texture, "Set Decal Normal Texture";
    SetDecalColorCommand(Color): color, set_color, "Set Decal Color";
    SetDecalLayerIndexCommand(u8): layer, set_layer, "Set Decal Layer Index";
}
