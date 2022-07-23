use crate::{
    define_swap_command,
    scene::commands::{Command, SceneContext},
};
use fyrox::{core::color::Color, core::math::Rect, resource::texture::Texture, scene::node::Node};

define_swap_command! {
    Node::as_rectangle_mut,
    SetRectangleColorCommand(Color): color, set_color, "Set Rectangle Color";
    SetRectangleUvRectCommand(Rect<f32>): uv_rect, set_uv_rect, "Set Rectangle Uv Rect";
    SetRectangleTextureCommand(Option<Texture>): texture_value, set_texture, "Set Rectangle Texture";
}
