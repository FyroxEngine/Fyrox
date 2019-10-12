use std::sync::{Mutex, Arc};
use crate::{
    resource::texture::Texture,
    gui::{
        EventSource,
        event::UIEvent
    }
};
use crate::gui::Drawable;
use crate::gui::draw::{DrawingContext, CommandKind, CommandTexture};
use rg3d_core::math::Rect;
use rg3d_core::color::Color;

pub struct Image {
    texture: Arc<Mutex<Texture>>,
}

impl Drawable for Image {
    fn draw(&mut self, drawing_context: &mut DrawingContext, bounds: &Rect<f32>, color: Color) {
        drawing_context.push_rect_filled(bounds, None, color);
        drawing_context.commit(CommandKind::Geometry, CommandTexture::Texture(self.texture.clone()))
    }
}

impl EventSource for Image {
    fn emit_event(&mut self) -> Option<UIEvent> {
        None
    }
}